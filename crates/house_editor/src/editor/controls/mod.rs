use bevy::prelude::*;

mod action_definitions;
mod action_handler;
mod camera_controls;
mod shortcuts;

pub use action_definitions::{ActionTrigger, EditorAction};
pub use shortcuts::Keymap;

pub struct ControlsPlugin;

impl Plugin for ControlsPlugin {
	fn build(&self, app: &mut App) {
		app.add_observer(action_handler::handle_action);
		app.add_plugins(shortcuts::ShortcutsPlugin);
		app.add_systems(Update, camera_controls::update_camera_controls);
	}
}
