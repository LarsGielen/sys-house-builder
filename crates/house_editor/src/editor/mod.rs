use bevy::{camera::RenderTarget, prelude::*, render::render_resource::TextureFormat};

use crate::{
	app_state::AppState,
	core::input::{ActionState, InputPlugin, Keymap},
	editor::{
		camera::{OrbitCamera, OrbitCameraPlugin},
		inputs::EditorInputPlugin,
	},
};

mod camera;
pub mod inputs;

pub struct EditorPlugin;
impl Plugin for EditorPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(InputPlugin);

		app.add_plugins(EditorInputPlugin);
		app.add_plugins(OrbitCameraPlugin);

		app.add_systems(OnEnter(AppState::Editor), setup_editor);
		app.add_systems(OnExit(AppState::Editor), remove_viewport_camera);

		app.insert_resource(Keymap::load(
			app.world().resource::<ActionState>(),
			include_str!("../../assets/keymap.toml"),
		));
	}
}

#[derive(Resource)]
pub struct ViewportCamera(pub Entity);

pub(crate) fn setup_editor(
	mut commands: Commands,
	mut images: ResMut<Assets<Image>>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
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

	// Temporary scene for checking the camera; remove once real geometry is rendered.
	commands.spawn((
		Mesh3d(meshes.add(Cuboid::new(2.0, 2.0, 2.0))),
		MeshMaterial3d(materials.add(Color::srgb(0.8, 0.4, 0.3))),
		Transform::from_xyz(0.0, 1.0, 0.0),
		DespawnOnExit::<AppState>(AppState::Editor),
	));
	commands.spawn((
		DirectionalLight::default(),
		Transform::from_xyz(4.0, 8.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
		DespawnOnExit::<AppState>(AppState::Editor),
	));
}

// The camera entity itself is despawned by `DespawnOnExit`; this drops the dangling handle to it.
fn remove_viewport_camera(mut commands: Commands) {
	commands.remove_resource::<ViewportCamera>();
}
