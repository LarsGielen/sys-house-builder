use bevy::prelude::*;

use crate::editor::{
	controls::{CameraAction, EditorAction},
	input::{ActionGroup, Keymap},
};

// Embedded so there is always a valid keymap to fall back to, without duplicating the bindings in code.
pub(super) const BUILT_IN_KEYMAP: &str = include_str!("../../../assets/keymap.toml");

// Temporary lookups for the camera, which still polls raw input. They go away once the camera
// reads the action state.
impl Keymap {
	/// The camera control whose shortcut was pressed this frame, if any.
	pub fn camera_control_pressed(
		&self,
		keys: &ButtonInput<KeyCode>,
		mouse_buttons: &ButtonInput<MouseButton>,
	) -> Option<CameraAction> {
		self.button_bindings().iter().find_map(|(shortcut, name)| {
			CameraAction::from_name(name).filter(|_| shortcut.just_pressed(keys, mouse_buttons))
		})
	}

	/// Whether the input of any shortcut of the control is still down, regardless of modifiers.
	pub fn camera_control_held(
		&self,
		control: CameraAction,
		keys: &ButtonInput<KeyCode>,
		mouse_buttons: &ButtonInput<MouseButton>,
	) -> bool {
		self.shortcuts_for(control)
			.any(|shortcut| shortcut.input_held(keys, mouse_buttons))
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
	let action = keymap
		.button_bindings()
		.iter()
		.find_map(|(shortcut, name)| {
			EditorAction::from_name(name).filter(|_| shortcut.just_pressed(&keys, &mouse_buttons))
		});

	if let Some(action) = action {
		commands.trigger(action);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::editor::input::ActionState;

	fn registered_actions() -> ActionState {
		let mut actions = ActionState::default();
		actions.register_group::<EditorAction>();
		actions.register_group::<CameraAction>();
		actions
	}

	#[test]
	fn built_in_keymap_is_valid() {
		let keymap = Keymap::parse(BUILT_IN_KEYMAP, &registered_actions()).unwrap();
		assert_eq!(keymap.button_bindings().len(), 7);
		assert_eq!(keymap.axis_bindings().len(), 2);
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
		assert!(keymap.camera_control_held(CameraAction::Pan, &keys, &mouse_buttons));
		assert!(keymap.camera_control_held(CameraAction::Orbit, &keys, &mouse_buttons));
	}

	#[test]
	fn finds_the_pressed_camera_control() {
		let keymap = Keymap::parse(CAMERA_KEYMAP, &registered_actions()).unwrap();
		let mut keys = ButtonInput::<KeyCode>::default();
		let mut mouse_buttons = ButtonInput::<MouseButton>::default();
		mouse_buttons.press(MouseButton::Middle);
		assert_eq!(
			keymap.camera_control_pressed(&keys, &mouse_buttons),
			Some(CameraAction::Orbit)
		);

		keys.press(KeyCode::ShiftLeft);
		assert_eq!(
			keymap.camera_control_pressed(&keys, &mouse_buttons),
			Some(CameraAction::Pan)
		);
	}
}
