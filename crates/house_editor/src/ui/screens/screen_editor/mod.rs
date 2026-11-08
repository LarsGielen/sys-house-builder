use crate::{
	app_state::AppState,
	editor::{ViewportCamera, setup_editor},
	ui::{
		icons::Icons,
		theme::{colors, spacing},
	},
};

use bevy::{input_focus::AutoFocus, prelude::*};

pub struct EditorScreenPlugin;

mod dock;
mod top_bar;
mod viewport;

use dock::dock;
use top_bar::top_bar;
use viewport::{sync_viewport_context, viewport};

impl Plugin for EditorScreenPlugin {
	fn build(&self, app: &mut App) {
		// The screen reads `ViewportCamera`, which the camera system inserts.
		app.add_systems(
			OnEnter(AppState::Editor),
			spawn_editor_screen.after(setup_editor),
		);
		app.add_systems(
			Update,
			sync_viewport_context.run_if(in_state(AppState::Editor)),
		);
	}
}

fn spawn_editor_screen(
	mut commands: Commands,
	viewport_camera: Res<ViewportCamera>,
	icons: Res<Icons>,
) {
	commands.spawn_scene(editor_screen(viewport_camera.0, &icons));
}

fn editor_screen(camera: Entity, icons: &Icons) -> impl Scene + use<> {
	bsn! {
		#EditorScreen
		DespawnOnExit::<AppState>(AppState::Editor)
		Node {
			width: percent(100),
			height: percent(100),
			flex_direction: FlexDirection::Column
		}
		BackgroundColor(colors::BACKGROUND)
		Children [
			@top_bar(icons)
			AutoFocus
			--
			Node {
				flex_grow: 1.0,
				min_height: px(0),
				column_gap: px(spacing::COMPACT),
				padding: UiRect::all(px(spacing::COMPACT)),
			}
			Children [
				@viewport(camera)
				--
				@dock()
			]
		]
	}
}
