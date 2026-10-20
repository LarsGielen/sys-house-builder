use bevy::{prelude::*, transform::helper::TransformHelper};
use wall_graph::{Wall, WallCurve, WallGraph, WallNodeId};

use crate::{
	coordinates::graph_to_bevy,
	ground_pointer::{GroundPointer, PointerOnGround},
	house::House,
};

const GRID_SPACINGS_METRES: [f32; 5] = [0.1, 0.25, 0.5, 1.0, 2.0];
const DEFAULT_GRID_SPACING_INDEX: usize = 3;
const SNAP_ACQUIRE_RADIUS_PIXELS: f32 = 12.0;
const SNAP_RELEASE_RADIUS_PIXELS: f32 = 20.0;
const SNAP_DISABLE_KEYS: [KeyCode; 2] = [KeyCode::ShiftLeft, KeyCode::ShiftRight];
const SNAP_MARKER_RADIUS: f32 = 0.08;

/// Chooses a snap target under the pointer, shows it, makes it available to editor tools.
pub struct SnappingPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SnappingSystems {
	Pick,
	Presentation,
}
impl Plugin for SnappingPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<SnapSettings>()
			.init_resource::<SnappedPointer>()
			.add_systems(Startup, spawn_snap_marker)
			.add_systems(
				PostUpdate,
				update_snapped_pointer.in_set(SnappingSystems::Pick),
			)
			.add_systems(
				PostUpdate,
				update_snap_marker.in_set(SnappingSystems::Presentation),
			);
	}
}

/// What a snap target was taken from. Priority is defined by `SnapSource::priority`.
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
	acquire_radius_pixels: f32,
	release_radius_pixels: f32,
}

impl Default for SnapSettings {
	fn default() -> Self {
		Self {
			grid_spacing_index: DEFAULT_GRID_SPACING_INDEX,
			acquire_radius_pixels: SNAP_ACQUIRE_RADIUS_PIXELS,
			release_radius_pixels: SNAP_RELEASE_RADIUS_PIXELS,
		}
	}
}

impl SnapSettings {
	pub fn grid_spacing(&self) -> f32 {
		GRID_SPACINGS_METRES[self.grid_spacing_index]
	}

	pub fn step_grid_spacing(&mut self, step: isize) {
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

pub fn snapping_disabled(keyboard: &ButtonInput<KeyCode>) -> bool {
	keyboard.any_pressed(SNAP_DISABLE_KEYS)
}

/// Picks the best selectable candidate: junctions first, then walls, then grid points,
/// and within a class the candidate nearest the pointer on screen.
/// Wall candidates use the closest point in graph space; under perspective this is
/// an approximation of the closest point on the projected wall.
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
				settings.release_radius_pixels
			} else {
				settings.acquire_radius_pixels
			};
			(distance <= radius).then_some((target, distance))
		})
		.min_by(|(first, first_distance), (second, second_distance)| {
			first
				.source
				.priority()
				.cmp(&second.source.priority())
				.then(first_distance.total_cmp(second_distance))
				.then_with(|| {
					let first_held = previous_source == Some(first.source);
					let second_held = previous_source == Some(second.source);
					second_held.cmp(&first_held)
				})
				.then_with(|| stable_source_order(graph, first.source, second.source))
		})
		.map(|(target, _)| target)
		.unwrap_or(SnapTarget::free(pointer.graph_position))
}

// Graph iteration order is unspecified, so equal candidates need a stable final order.
fn stable_source_order(
	graph: &WallGraph,
	first: SnapSource,
	second: SnapSource,
) -> std::cmp::Ordering {
	match (first, second) {
		(SnapSource::Junction(first), SnapSource::Junction(second)) => first.cmp(&second),
		(SnapSource::Wall(first), SnapSource::Wall(second)) => {
			// Different arcs can share both endpoints; their sweeps distinguish their paths.
			let sweep = |wall| match graph
				.wall_curve(wall)
				.expect("snap candidates are live walls")
			{
				WallCurve::Straight { .. } => 0.0,
				WallCurve::CircularArc { sweep, .. } => sweep,
			};
			(first.origin(), first.destination())
				.cmp(&(second.origin(), second.destination()))
				.then_with(|| sweep(first).total_cmp(&sweep(second)))
		}
		_ => first.priority().cmp(&second.priority()),
	}
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
	camera: Single<(Entity, &Camera), With<Camera3d>>,
	transforms: TransformHelper,
) {
	let (camera_entity, camera) = *camera;
	let camera_transform = transforms
		.compute_global_transform(camera_entity)
		.expect("the editor camera and its ancestors must have transforms");
	let snapping_disabled = snapping_disabled(&keyboard);
	let previous_source = snapped_pointer.target.map(|target| target.source);

	snapped_pointer.target = ground_pointer.hit.map(|pointer| {
		if snapping_disabled {
			return SnapTarget::free(pointer.graph_position);
		}
		choose_snap_target(
			house.graph(),
			pointer,
			&settings,
			previous_source,
			|graph_position| {
				camera
					.world_to_viewport(&camera_transform, graph_to_bevy(graph_position.extend(0.0)))
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

#[cfg(test)]
#[path = "tests/snapping.rs"]
mod tests;
