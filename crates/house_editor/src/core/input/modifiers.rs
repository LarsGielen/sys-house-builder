use std::{fmt, str::FromStr};

use bevy::input::{ButtonInput, keyboard::KeyCode};

/// A modifier key, left or right.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum Modifier {
	Ctrl,
	Shift,
	Alt,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub struct ModifierSet {
	pub ctrl: bool,
	pub shift: bool,
	pub alt: bool,
}

impl ModifierSet {
	pub fn without(mut self, modifier: Modifier) -> Self {
		match modifier {
			Modifier::Ctrl => self.ctrl = false,
			Modifier::Shift => self.shift = false,
			Modifier::Alt => self.alt = false,
		}
		self
	}

	pub fn contains(&self, modifier: Modifier) -> bool {
		match modifier {
			Modifier::Ctrl => self.ctrl,
			Modifier::Shift => self.shift,
			Modifier::Alt => self.alt,
		}
	}

	pub fn from_keys(button_input: &ButtonInput<KeyCode>) -> ModifierSet {
		ModifierSet {
			ctrl: button_input.pressed(KeyCode::ControlLeft)
				|| button_input.pressed(KeyCode::ControlRight),
			shift: button_input.pressed(KeyCode::ShiftLeft)
				|| button_input.pressed(KeyCode::ShiftRight),
			alt: button_input.pressed(KeyCode::AltLeft) || button_input.pressed(KeyCode::AltRight),
		}
	}
}

impl Modifier {
	pub(crate) const NAMES: [(&'static str, Modifier); 3] = [
		("ctrl", Modifier::Ctrl),
		("shift", Modifier::Shift),
		("alt", Modifier::Alt),
	];

	fn keys(self) -> [KeyCode; 2] {
		match self {
			Modifier::Ctrl => [KeyCode::ControlLeft, KeyCode::ControlRight],
			Modifier::Shift => [KeyCode::ShiftLeft, KeyCode::ShiftRight],
			Modifier::Alt => [KeyCode::AltLeft, KeyCode::AltRight],
		}
	}

	pub(crate) fn pressed(self, keys: &ButtonInput<KeyCode>) -> bool {
		self.keys().into_iter().any(|key| keys.pressed(key))
	}

	pub(crate) fn just_pressed(self, keys: &ButtonInput<KeyCode>) -> bool {
		self.keys().into_iter().any(|key| keys.just_pressed(key))
	}
}

/// Formats for display to the user, as the prefix of a shortcut, such as `Ctrl+Shift+`.
impl fmt::Display for ModifierSet {
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

impl FromStr for ModifierSet {
	type Err = String;

	/// Parses only the modifiers, such as `ctrl+shift`, case-insensitive.
	fn from_str(text: &str) -> Result<Self, String> {
		let mut modifiers = ModifierSet::default();
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
mod tests {
	use super::*;

	#[test]
	fn parses_modifiers_in_any_order_and_case() {
		let modifiers: ModifierSet = "Shift+ctrl".parse().unwrap();

		assert!(modifiers.ctrl && modifiers.shift && !modifiers.alt);
	}

	#[test]
	fn empty_text_is_no_modifiers() {
		assert_eq!("".parse::<ModifierSet>(), Ok(ModifierSet::default()));
	}

	#[test]
	fn rejects_unknown_and_repeated_modifiers() {
		assert!("meta".parse::<ModifierSet>().is_err());
		assert!("ctrl+ctrl".parse::<ModifierSet>().is_err());
		assert!("ctrl+".parse::<ModifierSet>().is_err());
	}

	#[test]
	fn reads_either_side_of_a_held_modifier() {
		let mut keys = ButtonInput::<KeyCode>::default();
		assert_eq!(ModifierSet::from_keys(&keys), ModifierSet::default());

		keys.press(KeyCode::ShiftRight);
		keys.press(KeyCode::ControlLeft);
		let held = ModifierSet::from_keys(&keys);
		assert!(held.ctrl && held.shift && !held.alt);
	}

	#[test]
	fn displays_as_a_shortcut_prefix() {
		assert_eq!(
			"ctrl+shift".parse::<ModifierSet>().unwrap().to_string(),
			"Ctrl+Shift+"
		);
		assert_eq!(ModifierSet::default().to_string(), "");
	}
}
