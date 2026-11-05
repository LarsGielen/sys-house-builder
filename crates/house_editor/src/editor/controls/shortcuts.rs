use bevy::prelude::*;

use crate::editor::{
	controls::EditorAction,
	input::{ActionGroup, ActionState},
};

// Embedded so there is always a valid keymap to fall back to, without duplicating the bindings in code.
pub(super) const BUILT_IN_KEYMAP: &str = include_str!("../../../assets/keymap.toml");

pub(super) fn trigger_editor_actions(state: Res<ActionState>, mut commands: Commands) {
	for &action in EditorAction::all() {
		if state.just_pressed(action) {
			commands.trigger(action);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::editor::{controls::CameraAction, input::Keymap};

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
		assert_eq!(keymap.axis_bindings().len(), 5);
	}
}
