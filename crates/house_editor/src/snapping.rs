use bevy::prelude::*;
use wall_graph::{Wall, WallGraph, WallNodeId};

use crate::{
	House,
	coordinates::graph_to_bevy,
	ground_pointer::{GroundPointer, PointerOnGround, update_ground_pointer},
};

const GRID_SPACINGS_METRES: [f32; 5] = [0.1, 0.25, 0.5, 1.0, 2.0];
const DEFAULT_GRID_SPACING_INDEX: usize = 3;
const SNAP_START_RADIUS_PIXELS: f32 = 12.0;
const SNAP_END_RADIUS_PIXELS: f32 = 20.0;
const SNAP_DISABLE_KEYS: [KeyCode; 2] = [KeyCode::ShiftLeft, KeyCode::ShiftRight];
const SNAP_MARKER_RADIUS: f32 = 0.08;

/// Chooses a snap target under the pointer, shows it, and exposes the grid controls.
pub struct SnappingPlugin;
impl Plugin for SnappingPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<SnapSettings>()
			.init_resource::<SnappedPointer>()
			.add_systems(Startup, spawn_snap_marker)
			.add_systems(
				Update,
				(
					step_grid_spacing,
					update_grid_spacing_label.run_if(resource_changed::<SnapSettings>),
					update_snap_readout,
					update_snap_marker,
				),
			)
			// Snapping compares screen distances, so it needs this frame's ground hit.
			.add_systems(
				PostUpdate,
				update_snapped_pointer
					.after(update_ground_pointer)
					.after(TransformSystems::Propagate),
			);
	}
}

/// What a snap target was taken from. The order of the variants is the snap priority.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SnapSource {
	Junction(WallNodeId),
	Wall(Wall),
	Grid,
	Free,
}

impl SnapSource {
	fn priority(self) -> u8 {
		match self {
			SnapSource::Junction(_) => 0,
			SnapSource::Wall(_) => 1,
			SnapSource::Grid => 2,
			SnapSource::Free => 3,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnapTarget {
	pub position: Vec2,
	pub source: SnapSource,
}

impl SnapTarget {
	fn free(position: Vec2) -> Self {
		Self {
			position,
			source: SnapSource::Free,
		}
	}
}

#[derive(Resource)]
pub struct SnapSettings {
	grid_spacing_index: usize,
	start_radius_pixels: f32,
	end_radius_pixels: f32,
}

impl Default for SnapSettings {
	fn default() -> Self {
		Self {
			grid_spacing_index: DEFAULT_GRID_SPACING_INDEX,
			start_radius_pixels: SNAP_START_RADIUS_PIXELS,
			end_radius_pixels: SNAP_END_RADIUS_PIXELS,
		}
	}
}

impl SnapSettings {
	pub fn grid_spacing(&self) -> f32 {
		GRID_SPACINGS_METRES[self.grid_spacing_index]
	}

	fn step_grid_spacing(&mut self, step: isize) {
		let last_index = GRID_SPACINGS_METRES.len() - 1;
		self.grid_spacing_index = self
			.grid_spacing_index
			.saturating_add_signed(step)
			.min(last_index);
	}
}

/// The snap target under the pointer; `None` when the pointer has no ground hit.
#[derive(Resource, Default)]
pub struct SnappedPointer {
	pub target: Option<SnapTarget>,
}

fn snapping_disabled(keyboard: &ButtonInput<KeyCode>) -> bool {
	keyboard.any_pressed(SNAP_DISABLE_KEYS)
}

/// Picks the best selectable candidate: junctions first, then walls, then grid points,
/// and within a class the one nearest the pointer on screen.
fn choose_snap_target(
	graph: &WallGraph,
	pointer: PointerOnGround,
	settings: &SnapSettings,
	previous_source: Option<SnapSource>,
	graph_to_viewport: impl Fn(Vec2) -> Option<Vec2>,
) -> SnapTarget {
	let junctions = graph.nodes().map(|(node, position)| SnapTarget {
		position,
		source: SnapSource::Junction(node),
	});
	let walls = graph.walls().filter_map(|wall| {
		let (_, position) = graph.wall_closest_point(wall, pointer.graph_position)?;
		Some(SnapTarget {
			position,
			source: SnapSource::Wall(wall),
		})
	});
	let grid = std::iter::once(SnapTarget {
		position: snap_to_grid(pointer.graph_position, settings.grid_spacing()),
		source: SnapSource::Grid,
	});

	junctions
		.chain(walls)
		.chain(grid)
		.filter_map(|target| {
			let distance = graph_to_viewport(target.position)?.distance(pointer.viewport_position);
			let radius = if previous_source == Some(target.source) {
				settings.end_radius_pixels
			} else {
				settings.start_radius_pixels
			};
			(distance <= radius).then_some((target, distance))
		})
		.min_by(|(first, first_distance), (second, second_distance)| {
			first
				.source
				.priority()
				.cmp(&second.source.priority())
				.then(first_distance.total_cmp(second_distance))
		})
		.map(|(target, _)| target)
		.unwrap_or(SnapTarget::free(pointer.graph_position))
}

fn snap_to_grid(position: Vec2, spacing: f32) -> Vec2 {
	(position / spacing).round() * spacing
}

fn update_snapped_pointer(
	mut snapped_pointer: ResMut<SnappedPointer>,
	ground_pointer: Res<GroundPointer>,
	house: Res<House>,
	settings: Res<SnapSettings>,
	keyboard: Res<ButtonInput<KeyCode>>,
	camera: Single<(&Camera, &GlobalTransform), With<Camera3d>>,
) {
	let (camera, camera_transform) = *camera;
	let snapping_disabled = snapping_disabled(&keyboard);
	let previous_source = snapped_pointer.target.map(|target| target.source);

	snapped_pointer.target = ground_pointer.hit.map(|pointer| {
		if snapping_disabled {
			return SnapTarget::free(pointer.graph_position);
		}
		choose_snap_target(
			&house.graph,
			pointer,
			&settings,
			previous_source,
			|graph_position| {
				camera
					.world_to_viewport(camera_transform, graph_to_bevy(graph_position.extend(0.0)))
					.ok()
			},
		)
	});
}

#[derive(Component)]
struct SnapMarker;

#[derive(Resource)]
struct SnapMarkerMaterials {
	junction: Handle<StandardMaterial>,
	wall: Handle<StandardMaterial>,
	grid: Handle<StandardMaterial>,
}

fn spawn_snap_marker(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let mut unlit = |color: Color| {
		materials.add(StandardMaterial {
			base_color: color,
			unlit: true,
			..default()
		})
	};
	let marker_materials = SnapMarkerMaterials {
		junction: unlit(Color::srgb(0.2, 0.85, 0.3)),
		wall: unlit(Color::srgb(0.95, 0.8, 0.2)),
		grid: unlit(Color::srgb(0.25, 0.5, 0.95)),
	};

	commands.spawn((
		Name::new("snap-marker"),
		SnapMarker,
		Mesh3d(meshes.add(Sphere::new(SNAP_MARKER_RADIUS))),
		MeshMaterial3d(marker_materials.grid.clone()),
		Visibility::Hidden,
	));
	commands.insert_resource(marker_materials);
}

fn update_snap_marker(
	snapped_pointer: Res<SnappedPointer>,
	marker_materials: Res<SnapMarkerMaterials>,
	marker: Single<
		(
			&mut Transform,
			&mut Visibility,
			&mut MeshMaterial3d<StandardMaterial>,
		),
		With<SnapMarker>,
	>,
) {
	let (mut transform, mut visibility, mut material) = marker.into_inner();

	let snapped = snapped_pointer.target.and_then(|target| {
		let material = match target.source {
			SnapSource::Junction(_) => &marker_materials.junction,
			SnapSource::Wall(_) => &marker_materials.wall,
			SnapSource::Grid => &marker_materials.grid,
			SnapSource::Free => return None,
		};
		Some((target.position, material))
	});

	let Some((position, wanted_material)) = snapped else {
		*visibility = Visibility::Hidden;
		return;
	};
	transform.translation = graph_to_bevy(position.extend(0.0));
	*visibility = Visibility::Inherited;
	if material.0 != *wanted_material {
		material.0 = wanted_material.clone();
	}
}

#[derive(Component)]
struct GridSpacingLabel;

#[derive(Component)]
struct SnapReadout;

#[derive(Component)]
struct GridSpacingStep(isize);

pub fn snap_controls() -> impl Bundle {
	(
		Node {
			flex_direction: FlexDirection::Column,
			row_gap: Val::Px(8.0),
			..default()
		},
		children![
			Text::new("Snap priority: junction, wall, grid. Hold Shift to disable."),
			(
				Node {
					flex_direction: FlexDirection::Row,
					align_items: AlignItems::Center,
					column_gap: Val::Px(8.0),
					..default()
				},
				children![
					(Text::new(""), GridSpacingLabel),
					grid_spacing_button("-", -1),
					grid_spacing_button("+", 1),
				],
			),
			(Text::new(""), SnapReadout),
		],
	)
}

fn grid_spacing_button(label: &str, step: isize) -> impl Bundle {
	(
		Button,
		GridSpacingStep(step),
		Node {
			padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)),
			..default()
		},
		BackgroundColor(Color::srgb(0.25, 0.25, 0.3)),
		children![Text::new(label)],
	)
}

fn step_grid_spacing(
	mut settings: ResMut<SnapSettings>,
	buttons: Query<(&Interaction, &GridSpacingStep), Changed<Interaction>>,
) {
	for (interaction, step) in &buttons {
		if *interaction == Interaction::Pressed {
			settings.step_grid_spacing(step.0);
		}
	}
}

fn update_grid_spacing_label(
	settings: Res<SnapSettings>,
	mut label: Single<&mut Text, With<GridSpacingLabel>>,
) {
	label.0 = format!("Grid: {} m", settings.grid_spacing());
}

fn update_snap_readout(
	snapped_pointer: Res<SnappedPointer>,
	settings: Res<SnapSettings>,
	keyboard: Res<ButtonInput<KeyCode>>,
	mut readout: Single<&mut Text, With<SnapReadout>>,
) {
	let description = match snapped_pointer.target {
		None => "Snap: pointer is not on the ground".to_string(),
		Some(SnapTarget { position, source }) => {
			let source = match source {
				SnapSource::Junction(_) => "junction".to_string(),
				SnapSource::Wall(_) => "wall".to_string(),
				SnapSource::Grid => format!("grid ({} m)", settings.grid_spacing()),
				SnapSource::Free if snapping_disabled(&keyboard) => {
					"free (snapping off)".to_string()
				}
				SnapSource::Free => "free".to_string(),
			};
			format!(
				"Snap: {source}\nx {:.3} m, y {:.3} m",
				position.x, position.y
			)
		}
	};

	// Writing identical text would still trigger a text layout every frame.
	if readout.0 != description {
		readout.0 = description;
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const PIXELS_PER_METRE: f32 = 100.0;

	fn project(graph_position: Vec2) -> Option<Vec2> {
		Some(graph_position * PIXELS_PER_METRE)
	}

	fn pointer_at(graph_position: Vec2) -> PointerOnGround {
		PointerOnGround {
			viewport_position: graph_position * PIXELS_PER_METRE,
			graph_position,
		}
	}

	fn settings_with_spacing(spacing: f32) -> SnapSettings {
		let grid_spacing_index = GRID_SPACINGS_METRES
			.iter()
			.position(|&candidate| candidate == spacing)
			.expect("spacing is one of the supported grid spacings");
		SnapSettings {
			grid_spacing_index,
			..default()
		}
	}

	fn graph_with_one_wall() -> WallGraph {
		let mut graph = WallGraph::new();
		graph
			.add_wall(Vec2::new(0.0, 0.0), Vec2::new(4.0, 0.0))
			.unwrap();
		graph
	}

	#[test]
	fn junction_beats_a_nearby_wall_and_grid_point() {
		let graph = graph_with_one_wall();
		// Near the wall end: the wall point and a grid point are also within the radius.
		let target = choose_snap_target(
			&graph,
			pointer_at(Vec2::new(3.95, 0.04)),
			&settings_with_spacing(0.5),
			None,
			project,
		);

		assert!(matches!(target.source, SnapSource::Junction(_)));
		assert_eq!(target.position, Vec2::new(4.0, 0.0));
	}

	#[test]
	fn wall_beats_a_grid_point() {
		let graph = graph_with_one_wall();
		let target = choose_snap_target(
			&graph,
			pointer_at(Vec2::new(2.0, 0.05)),
			&settings_with_spacing(0.5),
			None,
			project,
		);

		assert!(matches!(target.source, SnapSource::Wall(_)));
		assert!(target.position.distance(Vec2::new(2.0, 0.0)) < 1e-5);
	}

	#[test]
	fn grid_is_used_when_nothing_else_is_selectable() {
		let target = choose_snap_target(
			&graph_with_one_wall(),
			pointer_at(Vec2::new(1.03, 3.0)),
			&settings_with_spacing(0.5),
			None,
			project,
		);

		assert_eq!(target.source, SnapSource::Grid);
		assert_eq!(target.position, Vec2::new(1.0, 3.0));
	}

	#[test]
	fn nearer_junction_wins_within_a_class() {
		let mut graph = WallGraph::new();
		graph
			.add_wall(Vec2::new(0.0, 0.0), Vec2::new(0.1, 0.0))
			.unwrap();
		// Both ends are within the radius, 7 px and 3 px away.
		let target = choose_snap_target(
			&graph,
			pointer_at(Vec2::new(0.07, 0.0)),
			&settings_with_spacing(2.0),
			None,
			project,
		);

		assert!(matches!(target.source, SnapSource::Junction(_)));
		assert_eq!(target.position, Vec2::new(0.1, 0.0));
	}

	#[test]
	fn nothing_within_the_radius_gives_the_unsnapped_point() {
		let pointer = pointer_at(Vec2::new(1.3, 3.3));
		let target = choose_snap_target(
			&graph_with_one_wall(),
			pointer,
			&settings_with_spacing(2.0),
			None,
			project,
		);

		assert_eq!(target, SnapTarget::free(pointer.graph_position));
	}

	#[test]
	fn grid_spacing_changes_only_the_grid_candidate() {
		let graph = graph_with_one_wall();
		let near_junction = pointer_at(Vec2::new(3.95, 0.04));
		let near_grid_only = pointer_at(Vec2::new(1.1, 3.0));

		let junction_with_fine_grid = choose_snap_target(
			&graph,
			near_junction,
			&settings_with_spacing(0.1),
			None,
			project,
		);
		let junction_with_coarse_grid = choose_snap_target(
			&graph,
			near_junction,
			&settings_with_spacing(2.0),
			None,
			project,
		);
		assert_eq!(junction_with_fine_grid, junction_with_coarse_grid);

		let fine_grid = choose_snap_target(
			&graph,
			near_grid_only,
			&settings_with_spacing(0.1),
			None,
			project,
		);
		let coarse_grid = choose_snap_target(
			&graph,
			near_grid_only,
			&settings_with_spacing(0.5),
			None,
			project,
		);
		assert_eq!(fine_grid.source, SnapSource::Grid);
		assert_eq!(coarse_grid.source, SnapSource::Grid);
		assert!(fine_grid.position.distance(Vec2::new(1.1, 3.0)) < 1e-5);
		assert!(coarse_grid.position.distance(Vec2::new(1.0, 3.0)) < 1e-5);
	}

	#[test]
	fn snap_starts_at_the_start_radius_but_holds_until_the_end_radius() {
		let graph = graph_with_one_wall();
		let settings = settings_with_spacing(2.0);
		let junction = graph.nodes().find(|(_, position)| *position == Vec2::ZERO);
		let junction_source = SnapSource::Junction(junction.unwrap().0);

		// 15 px from the junction: outside the 12 px start radius, inside the 20 px end radius.
		let pointer = pointer_at(Vec2::new(-0.15, 0.0));
		let not_yet_snapped = choose_snap_target(&graph, pointer, &settings, None, project);
		assert_eq!(not_yet_snapped, SnapTarget::free(pointer.graph_position));

		let already_snapped =
			choose_snap_target(&graph, pointer, &settings, Some(junction_source), project);
		assert_eq!(already_snapped.source, junction_source);

		// 25 px away: past the end radius, so the snap lets go.
		let far_pointer = pointer_at(Vec2::new(-0.25, 0.0));
		let released = choose_snap_target(
			&graph,
			far_pointer,
			&settings,
			Some(junction_source),
			project,
		);
		assert_eq!(released, SnapTarget::free(far_pointer.graph_position));
	}

	#[test]
	fn holding_a_lower_priority_snap_does_not_block_a_junction() {
		let graph = graph_with_one_wall();
		let settings = settings_with_spacing(2.0);
		let wall_source = graph
			.walls()
			.next()
			.map(SnapSource::Wall)
			.expect("the graph has one wall");

		// Snapped to the wall, the pointer comes within the start radius of its end junction.
		let target = choose_snap_target(
			&graph,
			pointer_at(Vec2::new(3.95, 0.1)),
			&settings,
			Some(wall_source),
			project,
		);

		assert!(matches!(target.source, SnapSource::Junction(_)));
	}

	#[test]
	fn grid_stays_selectable_up_to_the_end_radius() {
		let settings = settings_with_spacing(1.0);
		// 15 px from the grid point (1, 3): too far to start, close enough to hold.
		let pointer = pointer_at(Vec2::new(1.15, 3.0));

		let not_yet_snapped =
			choose_snap_target(&WallGraph::new(), pointer, &settings, None, project);
		assert_eq!(not_yet_snapped.source, SnapSource::Free);

		let held = choose_snap_target(
			&WallGraph::new(),
			pointer,
			&settings,
			Some(SnapSource::Grid),
			project,
		);
		assert_eq!(held.source, SnapSource::Grid);
		assert_eq!(held.position, Vec2::new(1.0, 3.0));
	}

	#[test]
	fn grid_spacing_stays_within_the_supported_list() {
		let mut settings = SnapSettings::default();
		for _ in 0..10 {
			settings.step_grid_spacing(1);
		}
		assert_eq!(settings.grid_spacing(), 2.0);
		for _ in 0..10 {
			settings.step_grid_spacing(-1);
		}
		assert_eq!(settings.grid_spacing(), 0.1);
	}
}
