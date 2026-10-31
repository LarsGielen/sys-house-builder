use bevy::prelude::*;
use house_editor::{HouseEditorPlugin, setup_scene};

fn main() {
	App::new()
		.add_plugins(DefaultPlugins.set(WindowPlugin {
			primary_window: Some(Window {
				name: Some("bevy-dev".to_string()),
				title: "House Editor".to_string(),
				..default()
			}),
			// Closing asks about unsaved changes first, so the project plugin closes the app.
			close_when_requested: false,
			..default()
		}))
		.add_plugins(HouseEditorPlugin)
		.add_systems(Startup, setup_scene)
		.run();
}
