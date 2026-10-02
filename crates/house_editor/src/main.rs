mod orbit_camera;

use bevy::{
	log::{DEFAULT_FILTER, LogPlugin},
	prelude::*,
	ui::FocusPolicy,
};

use orbit_camera::{OrbitCamera, OrbitCameraPlugin};
use wall_graph::WallGraph;

fn main() {
	App::new()
		.add_plugins(
			DefaultPlugins
				.set(WindowPlugin {
					primary_window: Some(Window {
						name: Some("bevy-dev".to_string()),
						title: "House Editor".to_string(),
						..default()
					}),
					..default()
				})
				// rodio logs a harmless ALSA timestamp error every frame on some setups.
				.set(LogPlugin {
					filter: format!("{DEFAULT_FILTER},rodio=off"),
					..default()
				}),
		)
		.add_plugins(OrbitCameraPlugin)
		.init_resource::<House>()
		.add_systems(Startup, (setup_scene, setup_editor_panel))
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

fn setup_editor_panel(mut commands: Commands) {
	commands
		.spawn((
			Name::new("editor-panel"),
			Node {
				position_type: PositionType::Absolute,
				left: Val::Px(0.0),
				top: Val::Px(0.0),
				width: Val::Px(240.0),
				height: Val::Percent(100.0),
				padding: UiRect::all(Val::Px(12.0)),
				..default()
			},
			BackgroundColor(Color::srgba(0.1, 0.1, 0.12, 0.9)),
			// Without `Interaction` the node is never hover-tested, and `Block` stops
			// the pointer from also counting as over anything underneath.
			Interaction::default(),
			FocusPolicy::Block,
		))
		.with_child((
			Button,
			Node {
				padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
				..default()
			},
			BackgroundColor(Color::srgb(0.25, 0.25, 0.3)),
			children![Text::new("Test button")],
		));
}

/// Keeps house geometry independent of Bevy entities.
#[derive(Resource, Default)]
struct House(
	#[expect(dead_code, reason = "wall placement will use the graph in step 2")] WallGraph,
);
