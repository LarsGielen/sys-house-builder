use std::{fmt, str::FromStr};

use bevy::input::{ButtonInput, keyboard::KeyCode};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub struct Modifiers {
	pub ctrl: bool,
	pub shift: bool,
	pub alt: bool,
}

impl Modifiers {
	pub fn from_keys(button_input: &ButtonInput<KeyCode>) -> Modifiers {
		Modifiers {
			ctrl: button_input.pressed(KeyCode::ControlLeft)
				|| button_input.pressed(KeyCode::ControlRight),
			shift: button_input.pressed(KeyCode::ShiftLeft)
				|| button_input.pressed(KeyCode::ShiftRight),
			alt: button_input.pressed(KeyCode::AltLeft) || button_input.pressed(KeyCode::AltRight),
		}
	}
}

/// Formats for display to the user, as the prefix of a shortcut, such as `Ctrl+Shift+`.
impl fmt::Display for Modifiers {
	fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
		for (is_held, name) in [
			(self.ctrl, "Ctrl"),
			(self.shift, "Shift"),
			(self.alt, "Alt"),
		] {
			if is_held {
				write!(formatter, "{name}+")?;
			}
		}

		Ok(())
	}
}

impl FromStr for Modifiers {
	type Err = String;

	/// Parses only the modifiers, such as `ctrl+shift`, case-insensitive.
	fn from_str(text: &str) -> Result<Self, String> {
		let mut modifiers = Modifiers::default();
		if text.trim().is_empty() {
			return Ok(modifiers);
		}

		for token in text.split('+') {
			let modifier = token.trim().to_ascii_lowercase();
			let flag = match modifier.as_str() {
				"ctrl" => &mut modifiers.ctrl,
				"shift" => &mut modifiers.shift,
				"alt" => &mut modifiers.alt,
				_ => return Err(format!("unknown modifier \"{modifier}\"")),
			};
			if *flag {
				return Err(format!("repeated modifier \"{modifier}\""));
			}
			*flag = true;
		}

		Ok(modifiers)
	}
}

#[cfg(test)]
#[path = "tests/modifiers_tests.rs"]
mod tests;
