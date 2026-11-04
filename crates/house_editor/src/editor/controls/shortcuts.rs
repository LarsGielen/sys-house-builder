use std::collections::BTreeMap;

use bevy::{asset::io::file::FileAssetReader, prelude::*};
use serde::Deserialize;

use crate::editor::{
	controls::{CameraControl, EditorAction},
	input::{Action, ActionGroup, ActionState, Shortcut},
};

const KEYMAP_FILE: &str = "keymap.toml";

// Embedded so there is always a valid keymap to fall back to, without duplicating the bindings in code.
const BUILT_IN_KEYMAP: &str = include_str!("../../../assets/keymap.toml");

#[derive(Deserialize)]
struct KeymapFile {
	bindings: BTreeMap<String, Vec<String>>,
}

/// Shortcuts bound to registered actions by name, so the keymap works for any action group.
#[derive(Resource)]
pub struct Keymap {
	bindings: Vec<(Shortcut, &'static str)>,
}

impl Keymap {
	fn built_in(actions: &ActionState) -> Self {
		Self::parse(BUILT_IN_KEYMAP, actions).expect("the built-in keymap.toml is valid")
	}

	pub fn shortcuts_for(&self, action: impl Action) -> impl Iterator<Item = &Shortcut> {
		let name = action.name();
		self.bindings
			.iter()
			.filter(move |(_, bound_name)| *bound_name == name)
			.map(|(shortcut, _)| shortcut)
	}

	/// The camera control whose shortcut was pressed this frame, if any.
	pub fn camera_control_pressed(
		&self,
		keys: &ButtonInput<KeyCode>,
		mouse_buttons: &ButtonInput<MouseButton>,
	) -> Option<CameraControl> {
		self.bindings.iter().find_map(|(shortcut, name)| {
			CameraControl::from_name(name).filter(|_| shortcut.just_pressed(keys, mouse_buttons))
		})
	}

	/// Whether the input of any shortcut of the control is still down, regardless of modifiers.
	pub fn camera_control_held(
		&self,
		control: CameraControl,
		keys: &ButtonInput<KeyCode>,
		mouse_buttons: &ButtonInput<MouseButton>,
	) -> bool {
		self.shortcuts_for(control)
			.any(|shortcut| shortcut.input_held(keys, mouse_buttons))
	}

	/// Reads the keymap from the assets folder, falling back to the built-in keymap if the file
	/// is missing or invalid so that a broken file never leaves the editor without shortcuts.
	/// The actions it binds must already be registered.
	pub(super) fn load(actions: &ActionState) -> Self {
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
				return Self::built_in(actions);
			}
		};
		match Self::parse(&text, actions) {
			Ok(keymap) => keymap,
			Err(error) => {
				warn!("invalid {}: {error}; using built-in keymap", path.display());
				Self::built_in(actions)
			}
		}
	}

	// Builds the whole keymap before returning it, so an error never yields a partial keymap.
	fn parse(text: &str, actions: &ActionState) -> Result<Self, String> {
		let file: KeymapFile = toml::from_str(text).map_err(|error| error.to_string())?;

		let mut bindings: Vec<(Shortcut, &'static str)> = Vec::new();
		for (action_name, shortcut_texts) in file.bindings {
			let action = actions
				.registered_name(&action_name)
				.ok_or_else(|| format!("unknown action \"{action_name}\""))?;
			for shortcut_text in shortcut_texts {
				let shortcut: Shortcut = shortcut_text
					.parse()
					.map_err(|error| format!("action \"{action_name}\": {error}"))?;
				if let Some((_, other_action)) =
					bindings.iter().find(|(existing, _)| *existing == shortcut)
				{
					return Err(format!(
						"shortcut \"{shortcut_text}\" is bound to both {other_action} and {action}"
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
	// First match only: one key press must fire at most one action. Actions that are not editor
	// actions are skipped because they are held rather than triggered.
	let action = keymap.bindings.iter().find_map(|(shortcut, name)| {
		EditorAction::from_name(name).filter(|_| shortcut.just_pressed(&keys, &mouse_buttons))
	});

	if let Some(action) = action {
		commands.trigger(action);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::editor::controls::EditorAction;

	fn registered_actions() -> ActionState {
		let mut actions = ActionState::default();
		actions.register_group::<EditorAction>();
		actions.register_group::<CameraControl>();
		actions
	}

	#[test]
	fn built_in_keymap_is_valid() {
		let keymap = Keymap::parse(BUILT_IN_KEYMAP, &registered_actions()).unwrap();
		assert_eq!(keymap.bindings.len(), 7);
	}

	// Fixed here instead of read from the built-in keymap, whose default bindings may change.
	const CAMERA_KEYMAP: &str = "[bindings]\n\"camera.orbit\" = [\"mouse_middle\"]\n\"camera.pan\" = [\"shift+mouse_middle\"]";

	#[test]
	fn held_camera_control_ignores_modifiers() {
		let keymap = Keymap::parse(CAMERA_KEYMAP, &registered_actions()).unwrap();
		let keys = ButtonInput::<KeyCode>::default();
		let mut mouse_buttons = ButtonInput::<MouseButton>::default();
		mouse_buttons.press(MouseButton::Middle);

		// Shift was released after the pan started, which must not end the drag.
		assert!(keymap.camera_control_held(CameraControl::Pan, &keys, &mouse_buttons));
		assert!(keymap.camera_control_held(CameraControl::Orbit, &keys, &mouse_buttons));
	}

	#[test]
	fn finds_the_pressed_camera_control() {
		let keymap = Keymap::parse(CAMERA_KEYMAP, &registered_actions()).unwrap();
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
	fn finds_all_shortcuts_of_an_action() {
		let keymap = Keymap::parse(BUILT_IN_KEYMAP, &registered_actions()).unwrap();
		let redo = EditorAction::Redo;
		assert_eq!(keymap.shortcuts_for(redo).count(), 2);

		let keymap_without_editor_actions =
			Keymap::parse(CAMERA_KEYMAP, &registered_actions()).unwrap();
		assert_eq!(keymap_without_editor_actions.shortcuts_for(redo).count(), 0);
	}

	#[test]
	fn rejects_unknown_actions_and_conflicting_shortcuts() {
		let unknown_action = "[bindings]\n\"editor.explode\" = [\"ctrl+x\"]";
		assert!(Keymap::parse(unknown_action, &registered_actions()).is_err());

		let conflict = "[bindings]\n\"editor.undo\" = [\"ctrl+z\"]\n\"editor.redo\" = [\"ctrl+z\"]";
		assert!(Keymap::parse(conflict, &registered_actions()).is_err());

		let camera_conflict = "[bindings]\n\"camera.orbit\" = [\"mouse_middle\"]\n\"camera.pan\" = [\"mouse_middle\"]";
		assert!(Keymap::parse(camera_conflict, &registered_actions()).is_err());
	}
}
