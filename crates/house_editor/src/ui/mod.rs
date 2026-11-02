use bevy::prelude::*;

mod screens;
mod theme;
mod widgets;

pub struct AppUiPlugin;

impl Plugin for AppUiPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Startup, spawn_ui_camera)
			.add_plugins(screens::screen_editor::EditorScreenPlugin);
	}
}

fn spawn_ui_camera(mut commands: Commands) {
	commands.spawn((Name::new("UI camera"), Camera2d, IsDefaultUiCamera));
}
