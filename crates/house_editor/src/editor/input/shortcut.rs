use std::{fmt, str::FromStr};

use bevy::prelude::*;

use crate::editor::input::{modifiers::Modifiers, state::SourceId};

const KEY_NAMES: &[(&str, KeyCode)] = &[
	("a", KeyCode::KeyA),
	("b", KeyCode::KeyB),
	("c", KeyCode::KeyC),
	("d", KeyCode::KeyD),
	("e", KeyCode::KeyE),
	("f", KeyCode::KeyF),
	("g", KeyCode::KeyG),
	("h", KeyCode::KeyH),
	("i", KeyCode::KeyI),
	("j", KeyCode::KeyJ),
	("k", KeyCode::KeyK),
	("l", KeyCode::KeyL),
	("m", KeyCode::KeyM),
	("n", KeyCode::KeyN),
	("o", KeyCode::KeyO),
	("p", KeyCode::KeyP),
	("q", KeyCode::KeyQ),
	("r", KeyCode::KeyR),
	("s", KeyCode::KeyS),
	("t", KeyCode::KeyT),
	("u", KeyCode::KeyU),
	("v", KeyCode::KeyV),
	("w", KeyCode::KeyW),
	("x", KeyCode::KeyX),
	("y", KeyCode::KeyY),
	("z", KeyCode::KeyZ),
	("0", KeyCode::Digit0),
	("1", KeyCode::Digit1),
	("2", KeyCode::Digit2),
	("3", KeyCode::Digit3),
	("4", KeyCode::Digit4),
	("5", KeyCode::Digit5),
	("6", KeyCode::Digit6),
	("7", KeyCode::Digit7),
	("8", KeyCode::Digit8),
	("9", KeyCode::Digit9),
	("f1", KeyCode::F1),
	("f2", KeyCode::F2),
	("f3", KeyCode::F3),
	("f4", KeyCode::F4),
	("f5", KeyCode::F5),
	("f6", KeyCode::F6),
	("f7", KeyCode::F7),
	("f8", KeyCode::F8),
	("f9", KeyCode::F9),
	("f10", KeyCode::F10),
	("f11", KeyCode::F11),
	("f12", KeyCode::F12),
	("space", KeyCode::Space),
	("enter", KeyCode::Enter),
	("escape", KeyCode::Escape),
	("tab", KeyCode::Tab),
	("backspace", KeyCode::Backspace),
	("delete", KeyCode::Delete),
	("up", KeyCode::ArrowUp),
	("down", KeyCode::ArrowDown),
	("left", KeyCode::ArrowLeft),
	("right", KeyCode::ArrowRight),
];

// Prefixed so they cannot collide with key names such as "left", which is the arrow key.
const MOUSE_BUTTON_NAMES: &[(&str, MouseButton)] = &[
	("mouse_left", MouseButton::Left),
	("mouse_middle", MouseButton::Middle),
	("mouse_right", MouseButton::Right),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ShortcutInput {
	Key(KeyCode),
	Mouse(MouseButton),
}

impl ShortcutInput {
	pub(crate) fn from_name(name: &str) -> Option<Self> {
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
		}
	}

	pub(crate) fn source(&self) -> SourceId {
		match self {
			Self::Key(_) => SourceId::KEYBOARD,
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
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Shortcut {
	input: ShortcutInput,
	modifiers: Modifiers,
}

impl Shortcut {
	pub(crate) fn new(input: ShortcutInput, modifiers: Modifiers) -> Self {
		Self { input, modifiers }
	}

	// Modifiers must match exactly so that Ctrl+Shift+Z does not also fire the Ctrl+Z binding.
	pub(crate) fn just_pressed(
		&self,
		keys: &ButtonInput<KeyCode>,
		mouse_buttons: &ButtonInput<MouseButton>,
	) -> bool {
		self.input.just_pressed(keys, mouse_buttons) && self.modifiers == Modifiers::from_keys(keys)
	}

	// Modifiers are ignored on purpose: they only select the shortcut when it starts, and
	// releasing one mid-drag must not end the drag.
	pub(crate) fn input_held(
		&self,
		keys: &ButtonInput<KeyCode>,
		mouse_buttons: &ButtonInput<MouseButton>,
	) -> bool {
		self.input.pressed(keys, mouse_buttons)
	}

	pub(crate) fn source(&self) -> SourceId {
		self.input.source()
	}
}

/// Formats for display to the user, such as `Ctrl+Shift+Z`.
impl fmt::Display for Shortcut {
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

impl FromStr for Shortcut {
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
			.parse::<Modifiers>()
			.map_err(|error| format!("{error} in \"{text}\""))?;
		let input = ShortcutInput::from_name(&key_name)
			.ok_or_else(|| format!("unknown key or mouse button \"{key_name}\" in \"{text}\""))?;

		Ok(Shortcut { input, modifiers })
	}
}

#[cfg(test)]
#[path = "tests/shortcut_tests.rs"]
mod tests;
