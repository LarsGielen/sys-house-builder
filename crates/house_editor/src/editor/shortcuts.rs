use std::{collections::BTreeMap, str::FromStr};

use bevy::{asset::io::file::FileAssetReader, prelude::*};
use serde::Deserialize;

use crate::editor::EditorAction;

const KEYMAP_FILE: &str = "keymap.toml";

// Embedded so there is always a valid keymap to fall back to, without duplicating the bindings in code.
const BUILT_IN_KEYMAP: &str = include_str!("../../assets/keymap.toml");

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

pub struct ShortcutsPlugin;

impl Plugin for ShortcutsPlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(Keymap::load())
			.add_systems(Update, trigger_shortcuts);
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shortcut {
	key: KeyCode,
	ctrl: bool,
	shift: bool,
	alt: bool,
}

impl Shortcut {
	// Modifiers must match exactly so that Ctrl+Shift+Z does not also fire the Ctrl+Z binding.
	fn matches(&self, keys: &ButtonInput<KeyCode>) -> bool {
		keys.just_pressed(self.key)
			&& keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) == self.ctrl
			&& keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) == self.shift
			&& keys.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]) == self.alt
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
		let key = KEY_NAMES
			.iter()
			.find(|(name, _)| *name == key_name)
			.map(|(_, key)| *key)
			.ok_or_else(|| format!("unknown key \"{key_name}\" in \"{text}\""))?;

		let mut shortcut = Self {
			key,
			ctrl: false,
			shift: false,
			alt: false,
		};
		for modifier in tokens {
			let flag = match modifier.as_str() {
				"ctrl" => &mut shortcut.ctrl,
				"shift" => &mut shortcut.shift,
				"alt" => &mut shortcut.alt,
				_ => return Err(format!("unknown modifier \"{modifier}\" in \"{text}\"")),
			};
			if *flag {
				return Err(format!("repeated modifier \"{modifier}\" in \"{text}\""));
			}
			*flag = true;
		}
		Ok(shortcut)
	}
}

#[derive(Deserialize)]
struct KeymapFile {
	bindings: BTreeMap<String, Vec<String>>,
}

#[derive(Resource)]
pub struct Keymap {
	bindings: Vec<(Shortcut, EditorAction)>,
}

impl Default for Keymap {
	fn default() -> Self {
		Self::parse(BUILT_IN_KEYMAP).expect("the built-in keymap.toml is valid")
	}
}

impl Keymap {
	/// Reads the keymap from the assets folder, falling back to the built-in keymap if the file
	/// is missing or invalid so that a broken file never leaves the editor without shortcuts.
	fn load() -> Self {
		let path = FileAssetReader::get_base_path()
			.join("assets")
			.join(KEYMAP_FILE);
		let text = match std::fs::read_to_string(&path) {
			Ok(text) => text,
			Err(error) => {
				warn!(
					"cannot read {}: {error}; using built-in keymap",
					path.display()
				);
				return Self::default();
			}
		};
		match Self::parse(&text) {
			Ok(keymap) => keymap,
			Err(error) => {
				warn!("invalid {}: {error}; using built-in keymap", path.display());
				Self::default()
			}
		}
	}

	// Builds the whole keymap before returning it, so an error never yields a partial keymap.
	fn parse(text: &str) -> Result<Self, String> {
		let file: KeymapFile = toml::from_str(text).map_err(|error| error.to_string())?;

		let mut bindings: Vec<(Shortcut, EditorAction)> = Vec::new();
		for (action_name, shortcut_texts) in file.bindings {
			let action = EditorAction::from_name(&action_name)
				.ok_or_else(|| format!("unknown action \"{action_name}\""))?;
			for shortcut_text in shortcut_texts {
				let shortcut: Shortcut = shortcut_text
					.parse()
					.map_err(|error| format!("action \"{action_name}\": {error}"))?;
				if let Some((_, other_action)) =
					bindings.iter().find(|(existing, _)| *existing == shortcut)
				{
					return Err(format!(
						"shortcut \"{shortcut_text}\" is bound to both {other_action:?} and {action:?}"
					));
				}
				bindings.push((shortcut, action));
			}
		}
		Ok(Self { bindings })
	}
}

fn trigger_shortcuts(keys: Res<ButtonInput<KeyCode>>, keymap: Res<Keymap>, mut commands: Commands) {
	// First match only: one key press must fire at most one action.
	let Some((_, action)) = keymap
		.bindings
		.iter()
		.find(|(shortcut, _)| shortcut.matches(&keys))
	else {
		return;
	};

	commands.trigger(*action);
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn built_in_keymap_is_valid() {
		let keymap = Keymap::parse(BUILT_IN_KEYMAP).unwrap();
		assert_eq!(keymap.bindings.len(), 5);
	}

	#[test]
	fn shortcut_parsing_is_case_insensitive_and_checks_modifiers() {
		let shortcut: Shortcut = "Ctrl+Shift+Z".parse().unwrap();
		assert_eq!(shortcut.key, KeyCode::KeyZ);
		assert!(shortcut.ctrl && shortcut.shift && !shortcut.alt);

		assert!("ctrl+ctrl+z".parse::<Shortcut>().is_err());
		assert!("meta+z".parse::<Shortcut>().is_err());
		assert!("ctrl+".parse::<Shortcut>().is_err());
	}

	#[test]
	fn rejects_unknown_actions_and_conflicting_shortcuts() {
		let unknown_action = "[bindings]\n\"editor.explode\" = [\"ctrl+x\"]";
		assert!(Keymap::parse(unknown_action).is_err());

		let conflict = "[bindings]\n\"editor.undo\" = [\"ctrl+z\"]\n\"editor.redo\" = [\"ctrl+z\"]";
		assert!(Keymap::parse(conflict).is_err());
	}
}
