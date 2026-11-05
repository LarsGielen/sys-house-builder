use bevy::prelude::*;

mod app_state;
mod core;
mod editor;
mod ui;

fn main() {
	App::new()
		.add_plugins(DefaultPlugins.set(WindowPlugin {
			primary_window: Some(Window {
				name: Some("bevy-dev".to_string()),
				title: "House Editor".to_string(),
				resolution: (1440, 860).into(),
				resize_constraints: WindowResizeConstraints {
					min_width: 1100.0,
					min_height: 680.0,
					..default()
				},
				..default()
			}),
			..default()
		}))
		.init_state::<app_state::AppState>()
		.add_plugins((editor::EditorPlugin, ui::AppUiPlugin))
		.run();
}
