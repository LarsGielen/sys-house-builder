use bevy::prelude::*;

mod action_definitions;
mod action_handler;
mod camera_controls;
mod shortcuts;

pub use action_definitions::{ActionTrigger, CameraControl, EditorAction};

use crate::editor::input::{ActionState, InputAppExt, Keymap};

/// All systems that turn user input into editor changes.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ControlsSystems;

pub struct ControlsPlugin;

impl Plugin for ControlsPlugin {
	fn build(&self, app: &mut App) {
		// The keymap binds shortcuts by action name, so the actions must be registered before it loads.
		app.add_input_actions::<EditorAction>()
			.add_input_actions::<CameraControl>();

		app.add_observer(action_handler::handle_action);
		let keymap = Keymap::load(
			app.world().resource::<ActionState>(),
			shortcuts::BUILT_IN_KEYMAP,
		);
		app.insert_resource(keymap);
		app.init_resource::<camera_controls::ActiveCameraDrag>();
		app.add_systems(
			Update,
			(
				shortcuts::trigger_shortcuts,
				camera_controls::update_camera_controls,
				camera_controls::update_cursor_lock
					.after(camera_controls::update_camera_controls)
					.run_if(resource_changed::<camera_controls::ActiveCameraDrag>),
			)
				.in_set(ControlsSystems),
		);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::editor::input::InputPlugin;

	// The built-in keymap is parsed while the plugin builds, so this fails (panics) if the actions
	// are registered after the keymap loads.
	#[test]
	fn the_keymap_loads_after_the_actions_are_registered() {
		let mut app = App::new();
		app.add_plugins((InputPlugin, ControlsPlugin));

		assert!(app.world().contains_resource::<Keymap>());
	}
}
