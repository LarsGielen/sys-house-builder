use std::{fmt, str::FromStr};

use bevy::prelude::*;

use crate::core::input::{
	context::{Claim, ClaimedInput},
	input_names::{KEY_NAMES, MOUSE_BUTTON_NAMES},
	modifiers::{Modifier, ModifierSet},
	sources::SourceId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum PhysicalButton {
	Key(KeyCode),
	Mouse(MouseButton),
	Modifier(Modifier),
}

impl PhysicalButton {
	pub(crate) fn from_name(name: &str) -> Option<Self> {
		let modifier = Modifier::NAMES
			.iter()
			.find(|(modifier_name, _)| *modifier_name == name);
		if let Some((_, modifier)) = modifier {
			return Some(Self::Modifier(*modifier));
		}

		let key = KEY_NAMES.iter().find(|(key_name, _)| *key_name == name);
		let button = MOUSE_BUTTON_NAMES
			.iter()
			.find(|(button_name, _)| *button_name == name);
		match (key, button) {
			(Some((_, key)), _) => Some(Self::Key(*key)),
			(None, Some((_, button))) => Some(Self::Mouse(*button)),
			(None, None) => None,
		}
	}

	pub(crate) fn name(&self) -> Option<&'static str> {
		match self {
			Self::Key(key) => KEY_NAMES
				.iter()
				.find(|(_, other)| other == key)
				.map(|(name, _)| *name),
			Self::Mouse(button) => MOUSE_BUTTON_NAMES
				.iter()
				.find(|(_, other)| other == button)
				.map(|(name, _)| *name),
			Self::Modifier(modifier) => Modifier::NAMES
				.iter()
				.find(|(_, other)| other == modifier)
				.map(|(name, _)| *name),
		}
	}

	/// The modifiers held besides this input. A modifier key is not its own modifier, so a
	/// shortcut on `shift` itself needs no Shift to be held, and `ctrl+shift` means Ctrl is held.
	pub(crate) fn other_modifiers(&self, keys: &ButtonInput<KeyCode>) -> ModifierSet {
		let held = ModifierSet::from_keys(keys);
		match self {
			Self::Modifier(modifier) => held.without(*modifier),
			Self::Key(_) | Self::Mouse(_) => held,
		}
	}

	fn just_pressed(
		&self,
		keys: &ButtonInput<KeyCode>,
		mouse_buttons: &ButtonInput<MouseButton>,
	) -> bool {
		match self {
			Self::Key(key) => keys.just_pressed(*key),
			Self::Mouse(button) => mouse_buttons.just_pressed(*button),
			Self::Modifier(modifier) => modifier.just_pressed(keys),
		}
	}

	pub(crate) fn source(&self) -> SourceId {
		match self {
			Self::Key(_) | Self::Modifier(_) => SourceId::KEYBOARD,
			Self::Mouse(_) => SourceId::MOUSE,
		}
	}

	pub(crate) fn pressed(
		&self,
		keys: &ButtonInput<KeyCode>,
		mouse_buttons: &ButtonInput<MouseButton>,
	) -> bool {
		match self {
			Self::Key(key) => keys.pressed(*key),
			Self::Mouse(button) => mouse_buttons.pressed(*button),
			Self::Modifier(modifier) => modifier.pressed(keys),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ButtonCombo {
	input: PhysicalButton,
	modifiers: ModifierSet,
}

impl ButtonCombo {
	/// `ctrl+<key>`, for rules in code such as a context that lets Ctrl+S through.
	pub fn ctrl(key: KeyCode) -> Self {
		Self {
			input: PhysicalButton::Key(key),
			modifiers: ModifierSet {
				ctrl: true,
				..ModifierSet::default()
			},
		}
	}

	pub(crate) fn new(input: PhysicalButton, modifiers: ModifierSet) -> Self {
		Self { input, modifiers }
	}

	// Modifiers must match exactly so that Ctrl+Shift+Z does not also fire the Ctrl+Z binding.
	pub(crate) fn just_pressed(
		&self,
		keys: &ButtonInput<KeyCode>,
		mouse_buttons: &ButtonInput<MouseButton>,
	) -> bool {
		self.input.just_pressed(keys, mouse_buttons)
			&& self.modifiers == self.input.other_modifiers(keys)
	}

	// Modifiers are ignored on purpose: they only select the shortcut when it starts, and
	// releasing one mid-drag must not end the drag.
	pub(crate) fn pressed(
		&self,
		keys: &ButtonInput<KeyCode>,
		mouse_buttons: &ButtonInput<MouseButton>,
	) -> bool {
		self.input.pressed(keys, mouse_buttons)
	}

	pub(crate) fn source(&self) -> SourceId {
		self.input.source()
	}

	/// The input and modifiers this shortcut takes, for contexts to decide who handles it.
	pub(crate) fn claim(&self) -> Claim {
		Claim {
			input: ClaimedInput::Button(self.input),
			modifiers: self.modifiers,
		}
	}
}

/// Formats for display to the user, such as `Ctrl+Shift+Z`.
impl fmt::Display for ButtonCombo {
	fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
		write!(formatter, "{}", self.modifiers)?;

		match self.input.name() {
			Some(name) => {
				// "mouse_middle" is shown as "Mouse middle".
				let name = name.replace('_', " ");
				let mut characters = name.chars();
				let first = characters.next().map(|c| c.to_ascii_uppercase());
				write!(
					formatter,
					"{}{}",
					first.unwrap_or_default(),
					characters.as_str()
				)
			}
			// Unreachable for shortcuts parsed from text, but keeps Display total.
			None => write!(formatter, "{:?}", self.input),
		}
	}
}

impl FromStr for ButtonCombo {
	type Err = String;

	/// Parses text such as `ctrl+shift+z`: modifiers first, the key last, case-insensitive.
	fn from_str(text: &str) -> Result<Self, String> {
		let mut tokens: Vec<String> = text
			.split('+')
			.map(|token| token.trim().to_ascii_lowercase())
			.collect();

		let key_name = tokens.pop().unwrap_or_default();

		let modifiers = tokens
			.join("+")
			.parse::<ModifierSet>()
			.map_err(|error| format!("{error} in \"{text}\""))?;
		let input = PhysicalButton::from_name(&key_name)
			.ok_or_else(|| format!("unknown key or mouse button \"{key_name}\" in \"{text}\""))?;
		if let PhysicalButton::Modifier(modifier) = input
			&& modifiers.contains(modifier)
		{
			return Err(format!(
				"\"{key_name}\" cannot be its own modifier in \"{text}\""
			));
		}

		Ok(ButtonCombo { input, modifiers })
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::core::input::modifiers::{Modifier, ModifierSet};

	#[test]
	fn shortcut_parsing_is_case_insensitive_and_checks_modifiers() {
		let button_combo: ButtonCombo = "Ctrl+Shift+Z".parse().unwrap();
		assert_eq!(button_combo.input, PhysicalButton::Key(KeyCode::KeyZ));
		assert!(
			button_combo.modifiers.ctrl
				&& button_combo.modifiers.shift
				&& !button_combo.modifiers.alt
		);

		assert!("ctrl+ctrl+z".parse::<ButtonCombo>().is_err());
		assert!("meta+z".parse::<ButtonCombo>().is_err());
		assert!("ctrl+".parse::<ButtonCombo>().is_err());
	}

	#[test]
	fn displays_shortcuts_for_the_user() {
		let button_combo: ButtonCombo = "ctrl+shift+z".parse().unwrap();
		assert_eq!(button_combo.to_string(), "Ctrl+Shift+Z");

		let button_combo: ButtonCombo = "f5".parse().unwrap();
		assert_eq!(button_combo.to_string(), "F5");
	}

	#[test]
	fn parses_mouse_buttons_without_confusing_them_with_arrow_keys() {
		let button_combo: ButtonCombo = "shift+mouse_middle".parse().unwrap();
		assert_eq!(
			button_combo.input,
			PhysicalButton::Mouse(MouseButton::Middle)
		);
		assert!(button_combo.modifiers.shift);

		let arrow: ButtonCombo = "ctrl+left".parse().unwrap();
		assert_eq!(arrow.input, PhysicalButton::Key(KeyCode::ArrowLeft));

		assert!("mouse_back".parse::<ButtonCombo>().is_err());
	}

	#[test]
	fn displays_mouse_shortcuts_for_the_user() {
		let button_combo: ButtonCombo = "shift+mouse_middle".parse().unwrap();
		assert_eq!(button_combo.to_string(), "Shift+Mouse middle");
	}

	#[test]
	fn mouse_shortcut_requires_exact_modifiers_when_pressed() {
		let orbit: ButtonCombo = "mouse_middle".parse().unwrap();
		let pan: ButtonCombo = "shift+mouse_middle".parse().unwrap();

		let mut keys = ButtonInput::<KeyCode>::default();
		let mut mouse_buttons = ButtonInput::<MouseButton>::default();
		keys.press(KeyCode::ShiftLeft);
		mouse_buttons.press(MouseButton::Middle);

		assert!(pan.just_pressed(&keys, &mouse_buttons));
		assert!(!orbit.just_pressed(&keys, &mouse_buttons));
	}

	#[test]
	fn a_modifier_key_is_a_shortcut_input() {
		let shift: ButtonCombo = "shift".parse().unwrap();
		assert_eq!(shift.input, PhysicalButton::Modifier(Modifier::Shift));
		assert_eq!(shift.modifiers, ModifierSet::default());
		assert_eq!(shift.to_string(), "Shift");

		let both: ButtonCombo = "ctrl+shift".parse().unwrap();
		assert_eq!(both.input, PhysicalButton::Modifier(Modifier::Shift));
		assert!(both.modifiers.ctrl && !both.modifiers.shift);
		assert_eq!(both.to_string(), "Ctrl+Shift");
	}

	#[test]
	fn a_modifier_key_cannot_be_its_own_modifier() {
		assert!("shift+shift".parse::<ButtonCombo>().is_err());
		assert!("ctrl+shift+ctrl".parse::<ButtonCombo>().is_err());
	}

	#[test]
	fn a_modifier_shortcut_needs_only_the_other_modifiers() {
		let mut keys = ButtonInput::<KeyCode>::default();
		let mouse = ButtonInput::<MouseButton>::default();
		let shift: ButtonCombo = "shift".parse().unwrap();
		let ctrl_shift: ButtonCombo = "ctrl+shift".parse().unwrap();

		// Either shift key counts, and Shift itself is not an extra modifier.
		keys.press(KeyCode::ShiftRight);
		assert!(shift.just_pressed(&keys, &mouse));
		assert!(!ctrl_shift.just_pressed(&keys, &mouse));

		keys.clear();
		keys.press(KeyCode::ControlLeft);
		keys.clear();
		keys.press(KeyCode::ShiftLeft);
		assert!(ctrl_shift.just_pressed(&keys, &mouse));
		assert!(!shift.just_pressed(&keys, &mouse));
	}
}
