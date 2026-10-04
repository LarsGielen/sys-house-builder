mod coordinates;
mod edit;
mod ground_pointer;
mod orbit_camera;
mod snapping;
mod wall_render;

use bevy::{
	log::{DEFAULT_FILTER, LogPlugin},
	prelude::*,
	ui::FocusPolicy,
};

use edit::{WallProposal, propose_wall};
use ground_pointer::GroundPointerPlugin;
use orbit_camera::{OrbitCamera, OrbitCameraPlugin};
use snapping::{SnappingPlugin, snap_controls};
use wall_graph::{WallDimensions, WallGraph};
use wall_mesh::WallMesh;
use wall_render::WallRenderPlugin;

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
		.add_plugins((
			OrbitCameraPlugin,
			WallRenderPlugin,
			GroundPointerPlugin,
			SnappingPlugin,
		))
		.init_resource::<House>()
		.add_systems(Startup, (setup_scene, setup_editor_panel))
		.add_systems(Update, commit_test_wall)
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
				flex_direction: FlexDirection::Column,
				row_gap: Val::Px(12.0),
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
		))
		.with_child(snap_controls());
}

/// The committed house. Keeps geometry independent of Bevy entities; `mesh` is always
/// generated from `graph`, so replace both together.
#[derive(Resource, Default)]
struct House {
	graph: WallGraph,
	mesh: WallMesh,
}

/// Temporary driver until pointer drafting exists: each Enter press commits the next
/// side of a 3 m square through the same propose/commit path later tools will use.
fn commit_test_wall(
	keys: Res<ButtonInput<KeyCode>>,
	mut house: ResMut<House>,
	mut next_corner: Local<usize>,
) {
	const CORNERS: [Vec2; 5] = [
		Vec2::new(0.0, 0.0),
		Vec2::new(3.0, 0.0),
		Vec2::new(3.0, 3.0),
		Vec2::new(0.0, 3.0),
		Vec2::new(0.0, 0.0),
	];

	if !keys.just_pressed(KeyCode::Enter) || *next_corner + 1 >= CORNERS.len() {
		return;
	}

	let proposal = WallProposal {
		start: CORNERS[*next_corner],
		end: CORNERS[*next_corner + 1],
		dimensions: WallDimensions::default(),
	};
	match propose_wall(&house.graph, &proposal) {
		Ok(candidate) => {
			house.graph = candidate.graph;
			house.mesh = candidate.mesh;
			*next_corner += 1;
		}
		Err(error) => error!("wall rejected: {error}"),
	}
}
