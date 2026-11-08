use std::{fmt, str::FromStr};

use bevy::math::Vec2;
use serde::Deserialize;

use crate::core::input::{
	AxisMode, PhysicalButton, ValueKind, context::ClaimedInput, sources::SourceId, value::Signal,
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
		up: PhysicalButton,
		down: PhysicalButton,
	},
	Composite2D {
		up: PhysicalButton,
		down: PhysicalButton,
		left: PhysicalButton,
		right: PhysicalButton,
	},
}

impl AxisInput {
	const NAMED: [Self; 2] = [Self::MouseMotion, Self::Scroll];

	/// Fails when two directions are the same input, since they would cancel each other.
	pub fn composite_1d(up: PhysicalButton, down: PhysicalButton) -> Result<Self, String> {
		let composite = Self::Composite1D { up, down };
		composite.check_distinct_directions()?;
		Ok(composite)
	}

	/// Fails when two directions are the same input, since they would cancel each other.
	pub fn composite_2d(
		up: PhysicalButton,
		down: PhysicalButton,
		left: PhysicalButton,
		right: PhysicalButton,
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
	pub fn directions(self) -> Vec<PhysicalButton> {
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

	/// What this input takes from lower contexts: the mouse input itself, or each button of a
	/// composite.
	pub(crate) fn claimed_inputs(self) -> Vec<ClaimedInput> {
		match self {
			Self::MouseMotion => vec![ClaimedInput::MouseMotion],
			Self::Scroll => vec![ClaimedInput::Scroll],
			_ => self
				.directions()
				.into_iter()
				.map(ClaimedInput::Button)
				.collect(),
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
				ValueKind::Button | ValueKind::Toggle => "a button cannot have a scale".to_string(),
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

#[cfg(test)]
mod tests {
	use bevy::prelude::{KeyCode, MouseButton};

	use super::*;

	#[test]
	fn parses_every_axis_input_by_its_name_case_insensitively() {
		for input in AxisInput::NAMED {
			assert_eq!(input.name().unwrap().parse::<AxisInput>(), Ok(input));
		}
		assert_eq!(
			"Mouse_Motion".parse::<AxisInput>(),
			Ok(AxisInput::MouseMotion)
		);
		assert_eq!(" scroll ".parse::<AxisInput>(), Ok(AxisInput::Scroll));
	}

	#[test]
	fn rejects_unknown_names_and_shortcuts() {
		assert!("wheel".parse::<AxisInput>().is_err());
		assert!("".parse::<AxisInput>().is_err());
		assert!("shift+scroll".parse::<AxisInput>().is_err());
		assert!("mouse_right".parse::<AxisInput>().is_err());
	}

	#[test]
	fn each_input_produces_one_exact_relative_kind() {
		assert_eq!(
			AxisInput::MouseMotion.value_kind(),
			ValueKind::Axis2D(AxisMode::Relative)
		);
		assert_eq!(
			AxisInput::Scroll.value_kind(),
			ValueKind::Axis1D(AxisMode::Relative)
		);
	}

	#[test]
	fn displays_the_keymap_name() {
		assert_eq!(AxisInput::MouseMotion.to_string(), "mouse_motion");
	}

	#[test]
	fn a_scale_multiplies_each_component_and_a_negative_one_inverts() {
		assert_eq!(
			Scale::One(-2.0).apply(Signal::Axis1D(3.0)),
			Signal::Axis1D(-6.0)
		);
		assert_eq!(
			Scale::Two([0.5, -1.0]).apply(Signal::Axis2D(Vec2::new(4.0, 2.0))),
			Signal::Axis2D(Vec2::new(2.0, -2.0))
		);
	}

	#[test]
	fn the_identity_scale_leaves_a_value_unchanged() {
		let kind_1d = ValueKind::Axis1D(AxisMode::Relative);
		let kind_2d = ValueKind::Axis2D(AxisMode::Relative);

		assert_eq!(
			Scale::identity(kind_1d).apply(Signal::Axis1D(3.0)),
			Signal::Axis1D(3.0)
		);
		assert_eq!(
			Scale::identity(kind_2d).apply(Signal::Axis2D(Vec2::new(1.0, 2.0))),
			Signal::Axis2D(Vec2::new(1.0, 2.0))
		);
	}

	fn key(key: KeyCode) -> PhysicalButton {
		PhysicalButton::Key(key)
	}

	fn wasd() -> AxisInput {
		AxisInput::composite_2d(
			key(KeyCode::KeyW),
			key(KeyCode::KeyS),
			key(KeyCode::KeyA),
			key(KeyCode::KeyD),
		)
		.unwrap()
	}

	#[test]
	fn composites_produce_absolute_axes_of_their_dimension() {
		let rise = AxisInput::composite_1d(key(KeyCode::Space), key(KeyCode::KeyC)).unwrap();

		assert_eq!(wasd().value_kind(), ValueKind::Axis2D(AxisMode::Absolute));
		assert_eq!(rise.value_kind(), ValueKind::Axis1D(AxisMode::Absolute));
	}

	#[test]
	fn a_composite_cannot_use_the_same_input_for_two_directions() {
		let same_axis = AxisInput::composite_1d(key(KeyCode::KeyW), key(KeyCode::KeyW));
		assert!(same_axis.is_err());

		let across_axes = AxisInput::composite_2d(
			key(KeyCode::KeyW),
			key(KeyCode::KeyS),
			key(KeyCode::KeyA),
			key(KeyCode::KeyW),
		);
		assert!(across_axes.is_err());
	}

	#[test]
	fn a_composite_is_a_mouse_input_only_when_every_direction_is_a_mouse_button() {
		let mouse = |button| PhysicalButton::Mouse(button);
		let zoom =
			AxisInput::composite_1d(mouse(MouseButton::Left), mouse(MouseButton::Right)).unwrap();
		let mixed = AxisInput::composite_1d(mouse(MouseButton::Left), key(KeyCode::KeyC)).unwrap();

		assert_eq!(zoom.source(), SourceId::MOUSE);
		assert_eq!(mixed.source(), SourceId::KEYBOARD);
		assert_eq!(wasd().source(), SourceId::KEYBOARD);
		assert_eq!(AxisInput::Scroll.source(), SourceId::MOUSE);
	}

	#[test]
	fn composites_have_no_name_and_do_not_parse() {
		assert_eq!(wasd().name(), None);
		assert!("wasd".parse::<AxisInput>().is_err());
	}

	#[test]
	fn a_composite_displays_its_directions() {
		assert_eq!(
			wasd().to_string(),
			"composite 2d (up=w, down=s, left=a, right=d)"
		);
	}

	#[test]
	fn only_composites_have_directions() {
		assert!(AxisInput::MouseMotion.directions().is_empty());
		assert_eq!(wasd().directions().len(), 4);
	}
}
