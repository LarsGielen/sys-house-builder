use bevy::prelude::*;

mod action_definitions;
mod action_handler;
mod camera_controls;
mod shortcuts;

pub use action_definitions::{ActionTrigger, BoundAction, CameraControl, EditorAction};
pub use shortcuts::Keymap;

/// All systems that turn user input into editor changes.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ControlsSystems;

pub struct ControlsPlugin;

impl Plugin for ControlsPlugin {
	fn build(&self, app: &mut App) {
		app.add_observer(action_handler::handle_action);
		app.insert_resource(Keymap::load());
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
