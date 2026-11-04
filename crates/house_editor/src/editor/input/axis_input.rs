use std::{fmt, str::FromStr};

use crate::editor::input::{AxisMode, ValueKind};

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

#[cfg(test)]
#[path = "tests/axis_inputs_tests.rs"]
mod tests;
