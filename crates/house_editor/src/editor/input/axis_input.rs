use std::{fmt, str::FromStr};

use bevy::math::Vec2;
use serde::Deserialize;

use crate::editor::input::{AxisMode, ValueKind, modifiers::Modifiers, signal::Signal};

/// A physical input that produces an axis value, bound to an axis action in the keymap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AxisInput {
	MouseMotion,
	Scroll,
}

impl AxisInput {
	const ALL: [Self; 2] = [Self::MouseMotion, Self::Scroll];

	// Stored in the keymap file, so they must stay stable even if a variant is renamed.
	pub fn name(self) -> &'static str {
		match self {
			Self::MouseMotion => "mouse_motion",
			Self::Scroll => "scroll",
		}
	}

	/// The exact kind of action this input can drive. The keymap compares it with the action's kind,
	/// so motion cannot be bound to a button or to an absolute axis.
	pub fn value_kind(self) -> ValueKind {
		match self {
			Self::MouseMotion => ValueKind::Axis2D(AxisMode::Relative),
			Self::Scroll => ValueKind::Axis1D(AxisMode::Relative),
		}
	}
}

impl FromStr for AxisInput {
	type Err = String;

	/// Parses the name of an axis input, case-insensitive.
	fn from_str(text: &str) -> Result<Self, String> {
		let name = text.trim().to_ascii_lowercase();
		Self::ALL
			.into_iter()
			.find(|input| input.name() == name)
			.ok_or_else(|| format!("unknown axis input \"{text}\""))
	}
}

impl fmt::Display for AxisInput {
	fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
		formatter.write_str(self.name())
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
