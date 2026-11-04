use std::{collections::BTreeMap, fmt, str::FromStr};

use bevy::{asset::io::file::FileAssetReader, prelude::*};
use serde::Deserialize;

use crate::editor::controls::{CameraControl, action_definitions::BoundAction};

const KEYMAP_FILE: &str = "keymap.toml";

// Embedded so there is always a valid keymap to fall back to, without duplicating the bindings in code.
const BUILT_IN_KEYMAP: &str = include_str!("../../../assets/keymap.toml");

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShortcutInput {
	Key(KeyCode),
	Mouse(MouseButton),
}

impl ShortcutInput {
	fn from_name(name: &str) -> Option<Self> {
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

	fn name(&self) -> Option<&'static str> {
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

	fn pressed(
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shortcut {
	input: ShortcutInput,
	ctrl: bool,
	shift: bool,
	alt: bool,
}

impl Shortcut {
	// Modifiers must match exactly so that Ctrl+Shift+Z does not also fire the Ctrl+Z binding.
	fn just_pressed(
		&self,
		keys: &ButtonInput<KeyCode>,
		mouse_buttons: &ButtonInput<MouseButton>,
	) -> bool {
		self.input.just_pressed(keys, mouse_buttons)
			&& keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) == self.ctrl
			&& keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) == self.shift
			&& keys.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]) == self.alt
	}

	// Modifiers are ignored on purpose: they only select the shortcut when it starts, and
	// releasing one mid-drag must not end the drag.
	fn input_held(
		&self,
		keys: &ButtonInput<KeyCode>,
		mouse_buttons: &ButtonInput<MouseButton>,
	) -> bool {
		self.input.pressed(keys, mouse_buttons)
	}
}

/// Formats for display to the user, such as `Ctrl+Shift+Z`.
impl fmt::Display for Shortcut {
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
		let input = ShortcutInput::from_name(&key_name)
			.ok_or_else(|| format!("unknown key or mouse button \"{key_name}\" in \"{text}\""))?;

		let mut shortcut = Self {
			input,
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
	bindings: Vec<(Shortcut, BoundAction)>,
}

impl Default for Keymap {
	fn default() -> Self {
		Self::parse(BUILT_IN_KEYMAP).expect("the built-in keymap.toml is valid")
	}
}

impl Keymap {
	pub fn shortcuts_for(&self, action: BoundAction) -> impl Iterator<Item = &Shortcut> {
		self.bindings
			.iter()
			.filter(move |(_, bound_action)| *bound_action == action)
			.map(|(shortcut, _)| shortcut)
	}

	/// The camera control whose shortcut was pressed this frame, if any.
	pub fn camera_control_pressed(
		&self,
		keys: &ButtonInput<KeyCode>,
		mouse_buttons: &ButtonInput<MouseButton>,
	) -> Option<CameraControl> {
		self.bindings
			.iter()
			.find_map(|(shortcut, bound_action)| match bound_action {
				BoundAction::Camera(control) if shortcut.just_pressed(keys, mouse_buttons) => {
					Some(*control)
				}
				_ => None,
			})
	}

	/// Whether the input of any shortcut of the control is still down, regardless of modifiers.
	pub fn camera_control_held(
		&self,
		control: CameraControl,
		keys: &ButtonInput<KeyCode>,
		mouse_buttons: &ButtonInput<MouseButton>,
	) -> bool {
		self.bindings.iter().any(|(shortcut, bound_action)| {
			*bound_action == BoundAction::Camera(control)
				&& shortcut.input_held(keys, mouse_buttons)
		})
	}

	/// Reads the keymap from the assets folder, falling back to the built-in keymap if the file
	/// is missing or invalid so that a broken file never leaves the editor without shortcuts.
	pub(super) fn load() -> Self {
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

		let mut bindings: Vec<(Shortcut, BoundAction)> = Vec::new();
		for (action_name, shortcut_texts) in file.bindings {
			let action = BoundAction::from_name(&action_name)
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

pub(super) fn trigger_shortcuts(
	keys: Res<ButtonInput<KeyCode>>,
	mouse_buttons: Res<ButtonInput<MouseButton>>,
	keymap: Res<Keymap>,
	mut commands: Commands,
) {
	// First match only: one key press must fire at most one action. Camera controls are skipped
	// because they are held rather than triggered.
	let action = keymap
		.bindings
		.iter()
		.find_map(|(shortcut, bound_action)| match bound_action {
			BoundAction::Editor(action) if shortcut.just_pressed(&keys, &mouse_buttons) => {
				Some(*action)
			}
			_ => None,
		});

	if let Some(action) = action {
		commands.trigger(action);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn built_in_keymap_is_valid() {
		let keymap = Keymap::parse(BUILT_IN_KEYMAP).unwrap();
		assert_eq!(keymap.bindings.len(), 7);
	}

	#[test]
	fn shortcut_parsing_is_case_insensitive_and_checks_modifiers() {
		let shortcut: Shortcut = "Ctrl+Shift+Z".parse().unwrap();
		assert_eq!(shortcut.input, ShortcutInput::Key(KeyCode::KeyZ));
		assert!(shortcut.ctrl && shortcut.shift && !shortcut.alt);

		assert!("ctrl+ctrl+z".parse::<Shortcut>().is_err());
		assert!("meta+z".parse::<Shortcut>().is_err());
		assert!("ctrl+".parse::<Shortcut>().is_err());
	}

	#[test]
	fn displays_shortcuts_for_the_user() {
		let shortcut: Shortcut = "ctrl+shift+z".parse().unwrap();
		assert_eq!(shortcut.to_string(), "Ctrl+Shift+Z");

		let shortcut: Shortcut = "f5".parse().unwrap();
		assert_eq!(shortcut.to_string(), "F5");
	}

	#[test]
	fn parses_mouse_buttons_without_confusing_them_with_arrow_keys() {
		let shortcut: Shortcut = "shift+mouse_middle".parse().unwrap();
		assert_eq!(shortcut.input, ShortcutInput::Mouse(MouseButton::Middle));
		assert!(shortcut.shift);

		let arrow: Shortcut = "ctrl+left".parse().unwrap();
		assert_eq!(arrow.input, ShortcutInput::Key(KeyCode::ArrowLeft));

		assert!("mouse_back".parse::<Shortcut>().is_err());
	}

	#[test]
	fn displays_mouse_shortcuts_for_the_user() {
		let shortcut: Shortcut = "shift+mouse_middle".parse().unwrap();
		assert_eq!(shortcut.to_string(), "Shift+Mouse middle");
	}

	#[test]
	fn mouse_shortcut_requires_exact_modifiers_when_pressed() {
		let orbit: Shortcut = "mouse_middle".parse().unwrap();
		let pan: Shortcut = "shift+mouse_middle".parse().unwrap();

		let mut keys = ButtonInput::<KeyCode>::default();
		let mut mouse_buttons = ButtonInput::<MouseButton>::default();
		keys.press(KeyCode::ShiftLeft);
		mouse_buttons.press(MouseButton::Middle);

		assert!(pan.just_pressed(&keys, &mouse_buttons));
		assert!(!orbit.just_pressed(&keys, &mouse_buttons));
	}

	// Fixed here instead of read from the built-in keymap, whose default bindings may change.
	const CAMERA_KEYMAP: &str = "[bindings]\n\"camera.orbit\" = [\"mouse_middle\"]\n\"camera.pan\" = [\"shift+mouse_middle\"]";

	#[test]
	fn held_camera_control_ignores_modifiers() {
		let keymap = Keymap::parse(CAMERA_KEYMAP).unwrap();
		let keys = ButtonInput::<KeyCode>::default();
		let mut mouse_buttons = ButtonInput::<MouseButton>::default();
		mouse_buttons.press(MouseButton::Middle);

		// Shift was released after the pan started, which must not end the drag.
		assert!(keymap.camera_control_held(CameraControl::Pan, &keys, &mouse_buttons));
		assert!(keymap.camera_control_held(CameraControl::Orbit, &keys, &mouse_buttons));
	}

	#[test]
	fn finds_the_pressed_camera_control() {
		let keymap = Keymap::parse(CAMERA_KEYMAP).unwrap();
		let mut keys = ButtonInput::<KeyCode>::default();
		let mut mouse_buttons = ButtonInput::<MouseButton>::default();
		mouse_buttons.press(MouseButton::Middle);
		assert_eq!(
			keymap.camera_control_pressed(&keys, &mouse_buttons),
			Some(CameraControl::Orbit)
		);

		keys.press(KeyCode::ShiftLeft);
		assert_eq!(
			keymap.camera_control_pressed(&keys, &mouse_buttons),
			Some(CameraControl::Pan)
		);
	}

	#[test]
	fn rejects_unknown_actions_and_conflicting_shortcuts() {
		let unknown_action = "[bindings]\n\"editor.explode\" = [\"ctrl+x\"]";
		assert!(Keymap::parse(unknown_action).is_err());

		let conflict = "[bindings]\n\"editor.undo\" = [\"ctrl+z\"]\n\"editor.redo\" = [\"ctrl+z\"]";
		assert!(Keymap::parse(conflict).is_err());

		let camera_conflict = "[bindings]\n\"camera.orbit\" = [\"mouse_middle\"]\n\"camera.pan\" = [\"mouse_middle\"]";
		assert!(Keymap::parse(camera_conflict).is_err());
	}
}
