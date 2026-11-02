use bevy::{camera::RenderTarget, prelude::*, render::render_resource::TextureFormat};

use crate::{
	app_state::AppState,
	editor::camera::{OrbitCamera, OrbitCameraPlugin},
};

mod action_handler;
mod actions;
mod camera;
mod shortcuts;

pub use actions::{ActionTrigger, EditorAction};

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((OrbitCameraPlugin, shortcuts::ShortcutsPlugin));
		app.add_systems(OnEnter(AppState::Editor), setup_editor);
		app.add_systems(OnExit(AppState::Editor), remove_viewport_camera);
		app.add_observer(action_handler::handle_action);
	}
}

#[derive(Resource)]
pub struct ViewportCamera(pub Entity);

pub(crate) fn setup_editor(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
	let render_target = images.add(Image::new_target_texture(
		1,
		1,
		TextureFormat::Rgba8UnormSrgb,
		None,
	));

	let camera = commands
		.spawn((
			Camera3d::default(),
			Camera {
				order: -1,
				..default()
			},
			RenderTarget::Image(render_target.into()),
			OrbitCamera::default(),
			DespawnOnExit::<AppState>(AppState::Editor),
		))
		.id();
	commands.insert_resource(ViewportCamera(camera));
}

// The camera entity itself is despawned by `DespawnOnExit`; this drops the dangling handle to it.
fn remove_viewport_camera(mut commands: Commands) {
	commands.remove_resource::<ViewportCamera>();
}
