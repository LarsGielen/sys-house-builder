use bevy::prelude::*;
use house_editor::{HouseEditorPlugin, OrbitCamera};

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

fn setup_scene(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	commands.spawn((
		Name::new("editor-floor"),
		Mesh3d(meshes.add(Rectangle::new(4.0, 4.0))),
		MeshMaterial3d(materials.add(Color::WHITE)),
		Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
	));

	commands.spawn((
		PointLight {
			shadow_maps_enabled: true,
			..default()
		},
		Transform::from_xyz(4.0, 8.0, 4.0),
	));

	commands.spawn((Camera3d::default(), OrbitCamera::default()));
}
