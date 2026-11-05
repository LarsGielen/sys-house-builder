use std::{fmt, str::FromStr};

use bevy::math::Vec2;
use serde::Deserialize;

use crate::editor::input::{
	AxisMode, ShortcutInput, ValueKind, modifiers::Modifiers, signal::Signal, state::SourceId,
};

/// A physical input that produces an axis value, bound to an axis action in the keymap.
///
/// The mouse inputs are relative: they report what changed this frame. A composite turns buttons
/// into an absolute axis: it reports which directions are held right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AxisInput {
	MouseMotion,
	Scroll,
	Composite1D {
		up: ShortcutInput,
		down: ShortcutInput,
	},
	Composite2D {
		up: ShortcutInput,
		down: ShortcutInput,
		left: ShortcutInput,
		right: ShortcutInput,
	},
}

impl AxisInput {
	const NAMED: [Self; 2] = [Self::MouseMotion, Self::Scroll];

	/// Fails when two directions are the same input, since they would cancel each other.
	pub fn composite_1d(up: ShortcutInput, down: ShortcutInput) -> Result<Self, String> {
		let composite = Self::Composite1D { up, down };
		composite.check_distinct_directions()?;
		Ok(composite)
	}

	/// Fails when two directions are the same input, since they would cancel each other.
	pub fn composite_2d(
		up: ShortcutInput,
		down: ShortcutInput,
		left: ShortcutInput,
		right: ShortcutInput,
	) -> Result<Self, String> {
		let composite = Self::Composite2D {
			up,
			down,
			left,
			right,
		};
		composite.check_distinct_directions()?;
		Ok(composite)
	}

	/// The name stored in the keymap file. Composites have none: they are written as a table.
	// Stable, even if a variant is renamed.
	pub fn name(self) -> Option<&'static str> {
		match self {
			Self::MouseMotion => Some("mouse_motion"),
			Self::Scroll => Some("scroll"),
			Self::Composite1D { .. } | Self::Composite2D { .. } => None,
		}
	}

	/// The exact kind of action this input can drive. The keymap compares it with the action's kind,
	/// so motion cannot be bound to a button or to an absolute axis.
	pub fn value_kind(self) -> ValueKind {
		match self {
			Self::MouseMotion => ValueKind::Axis2D(AxisMode::Relative),
			Self::Scroll => ValueKind::Axis1D(AxisMode::Relative),
			Self::Composite1D { .. } => ValueKind::Axis1D(AxisMode::Absolute),
			Self::Composite2D { .. } => ValueKind::Axis2D(AxisMode::Absolute),
		}
	}

	/// The buttons a composite reads, empty for the other inputs.
	pub fn directions(self) -> Vec<ShortcutInput> {
		match self {
			Self::MouseMotion | Self::Scroll => Vec::new(),
			Self::Composite1D { up, down } => vec![up, down],
			Self::Composite2D {
				up,
				down,
				left,
				right,
			} => vec![up, down, left, right],
		}
	}

	/// The source that arbitrates this input. A composite of only mouse buttons is a mouse input,
	/// any other composite is a keyboard input.
	pub fn source(self) -> SourceId {
		match self {
			Self::MouseMotion | Self::Scroll => SourceId::MOUSE,
			Self::Composite1D { .. } | Self::Composite2D { .. } => {
				let all_mouse = self
					.directions()
					.iter()
					.all(|direction| direction.source() == SourceId::MOUSE);
				if all_mouse {
					SourceId::MOUSE
				} else {
					SourceId::KEYBOARD
				}
			}
		}
	}

	fn check_distinct_directions(self) -> Result<(), String> {
		let directions = self.directions();
		for (index, direction) in directions.iter().enumerate() {
			if directions[..index].contains(direction) {
				return Err(format!(
					"\"{}\" is used for two directions of the same composite",
					direction.name().unwrap_or("?")
				));
			}
		}
		Ok(())
	}
}

impl FromStr for AxisInput {
	type Err = String;

	/// Parses the name of a mouse axis input, case-insensitive. Composites are not named.
	fn from_str(text: &str) -> Result<Self, String> {
		let name = text.trim().to_ascii_lowercase();
		Self::NAMED
			.into_iter()
			.find(|input| input.name() == Some(name.as_str()))
			.ok_or_else(|| format!("unknown axis input \"{text}\""))
	}
}

impl fmt::Display for AxisInput {
	fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
		if let Some(name) = self.name() {
			return formatter.write_str(name);
		}

		const LABELS: [&str; 4] = ["up", "down", "left", "right"];
		let directions = self.directions();
		let kind = if directions.len() == 2 { "1d" } else { "2d" };
		write!(formatter, "composite {kind} (")?;
		for (index, direction) in directions.iter().enumerate() {
			if index > 0 {
				write!(formatter, ", ")?;
			}
			write!(
				formatter,
				"{}={}",
				LABELS[index],
				direction.name().unwrap_or("?")
			)?;
		}
		write!(formatter, ")")
	}
}

/// Multiplies the value of an axis binding: a number for 1D axes and `[x, y]` for 2D axes.
/// Negative values invert, so there is no separate invert option.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Scale {
	One(f32),
	Two([f32; 2]),
}

impl Scale {
	/// The scale that leaves a value of this kind unchanged, for bindings that have none.
	pub fn identity(kind: ValueKind) -> Self {
		match kind {
			ValueKind::Axis2D(_) => Self::Two([1.0, 1.0]),
			_ => Self::One(1.0),
		}
	}

	/// Checks that the scale fits the kind of value it multiplies and cannot silence it: a zero or
	/// non-finite factor would make the binding do nothing.
	pub fn validate(self, kind: ValueKind) -> Result<Self, String> {
		let (fits, factors) = match (self, kind) {
			(Scale::One(factor), ValueKind::Axis1D(_)) => (true, vec![factor]),
			(Scale::Two(factors), ValueKind::Axis2D(_)) => (true, factors.to_vec()),
			(Scale::One(factor), _) => (false, vec![factor]),
			(Scale::Two(factors), _) => (false, factors.to_vec()),
		};

		if !fits {
			return Err(match kind {
				ValueKind::Axis2D(_) => "a 2D axis needs a scale of the form [x, y]".to_string(),
				ValueKind::Axis1D(_) => "a 1D axis needs a single number as scale".to_string(),
				ValueKind::Button => "a button cannot have a scale".to_string(),
			});
		}
		if factors
			.iter()
			.any(|factor| !factor.is_finite() || *factor == 0.0)
		{
			return Err("a scale must be finite and not zero".to_string());
		}
		Ok(self)
	}

	pub fn apply(self, signal: Signal) -> Signal {
		match (self, signal) {
			(Scale::One(factor), Signal::Axis1D(value)) => Signal::Axis1D(value * factor),
			(Scale::Two([x, y]), Signal::Axis2D(value)) => Signal::Axis2D(value * Vec2::new(x, y)),
			// The keymap only pairs a scale with a signal of its own shape.
			_ => signal,
		}
	}
}

/// An axis input bound to an action by name, with the scale applied to its value.
///
/// It only contributes while the held modifiers equal `modifiers` exactly, so `scroll` stops
/// while Shift is held and `shift+scroll` takes over. Unlike a shortcut it is not latched: it
/// is evaluated every frame, so pressing or releasing a modifier switches binding immediately.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisBinding {
	pub input: AxisInput,
	pub modifiers: Modifiers,
	pub scale: Scale,
	pub action: &'static str,
}

#[cfg(test)]
#[path = "tests/axis_inputs_tests.rs"]
mod tests;
