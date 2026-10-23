use bevy::prelude::*;
use wall_graph::{Wall, WallGraph, WallNodeId};
use wall_mesh::{SurfaceSource, WallMesh};

use crate::{
	coordinates::{bevy_to_graph, graph_to_bevy},
	ground_pointer::GroundPointer,
	house::House,
	wall_tool_active::{ActiveTool, Tool},
};

const HOVER_COLOR: Color = Color::srgb(1.0, 0.85, 0.2);
const SELECTED_COLOR: Color = Color::srgb(0.2, 0.6, 1.0);
const HANDLE_RADIUS_METRES: f32 = 0.1;
// Lifts the highlight above the wall top so the wall mesh does not hide it.
const HIGHLIGHT_LIFT_METRES: f32 = 0.02;
const SAMPLE_DEVIATION_METRES: f32 = 0.01;

/// Hovers and selects committed walls with the selection tool.
pub struct WallToolSelectionPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WallToolSelectionSystems {
	Pick,
	Presentation,
}

impl Plugin for WallToolSelectionPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<HoveredWall>()
			.init_resource::<SelectedWall>()
			.add_systems(
				PostUpdate,
				(
					clear_selection_after_commit.run_if(resource_changed::<House>),
					update_selection,
				)
					.chain()
					.in_set(WallToolSelectionSystems::Pick),
			)
			.add_systems(
				PostUpdate,
				draw_wall_highlights.in_set(WallToolSelectionSystems::Presentation),
			);
	}
}

/// The wall under the pointer, recomputed every frame from the committed mesh.
#[derive(Resource, Default, PartialEq)]
pub struct HoveredWall(pub Option<Wall>);

/// The selected wall. Wall handles do not survive a commit, so every commit clears it.
#[derive(Resource, Default, PartialEq)]
pub struct SelectedWall(pub Option<Wall>);

fn clear_selection_after_commit(mut selected: ResMut<SelectedWall>) {
	selected.set_if_neq(SelectedWall(None));
}

fn update_selection(
	active_tool: Res<ActiveTool>,
	mouse: Res<ButtonInput<MouseButton>>,
	keyboard: Res<ButtonInput<KeyCode>>,
	ground_pointer: Res<GroundPointer>,
	house: Res<House>,
	mut hovered: ResMut<HoveredWall>,
	mut selected: ResMut<SelectedWall>,
) {
	if active_tool.0 != Tool::Select {
		hovered.set_if_neq(HoveredWall(None));
		selected.set_if_neq(SelectedWall(None));
		return;
	}

	hovered.set_if_neq(HoveredWall(ground_pointer.ray.and_then(|ray| {
		pick_wall(
			house.graph(),
			house.mesh(),
			bevy_to_graph(ray.origin),
			bevy_to_graph(*ray.direction),
		)
	})));

	// Escape wins over a click in the same frame, as cancelling does in the drawing tool.
	if keyboard.just_pressed(KeyCode::Escape) {
		selected.set_if_neq(SelectedWall(None));
		return;
	}
	// A ray exists only when the viewport owns the pointer, so UI clicks never get here.
	if mouse.just_pressed(MouseButton::Left) && ground_pointer.ray.is_some() {
		selected.set_if_neq(SelectedWall(hovered.0));
	}
}

/// The wall owning the nearest mesh surface along a ray, and where the ray meets it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WallHit {
	pub wall: Wall,
	/// Graph metres.
	pub position: Vec3,
}

/// Finds the wall owning the nearest mesh surface along a graph-space ray.
fn pick_wall(graph: &WallGraph, mesh: &WallMesh, origin: Vec3, direction: Vec3) -> Option<Wall> {
	pick_wall_hit(graph, mesh, origin, direction).map(|hit| hit.wall)
}

pub fn pick_wall_hit(
	graph: &WallGraph,
	mesh: &WallMesh,
	origin: Vec3,
	direction: Vec3,
) -> Option<WallHit> {
	let positions = mesh.positions();
	let indices = mesh.indices();
	let (distance, source) = mesh
		.sections()
		.iter()
		.flat_map(|section| {
			indices[section.indices.clone()]
				.as_chunks::<3>()
				.0
				.iter()
				.map(move |triangle| (section.tag.source, triangle))
		})
		.filter_map(|(source, triangle)| {
			let corners = triangle.map(|index| positions[index as usize]);
			Some((ray_triangle_distance(origin, direction, corners)?, source))
		})
		.min_by(|(a, _), (b, _)| a.total_cmp(b))?;

	let position = origin + direction * distance;
	let wall = match source {
		SurfaceSource::Wall(wall) => Some(wall),
		SurfaceSource::Opening(opening) => Some(graph.opening(opening)?.wall),
		SurfaceSource::Junction(node) => nearest_wall_at_junction(graph, node, position.truncate()),
	}?;
	Some(WallHit { wall, position })
}

/// A junction surface belongs to several walls, so it selects the one closest to the hit.
fn nearest_wall_at_junction(graph: &WallGraph, node: WallNodeId, point: Vec2) -> Option<Wall> {
	graph
		.node_walls(node)?
		.into_iter()
		.filter_map(|wall| {
			let (_, closest) = graph.wall_closest_point(wall, point)?;
			Some((wall, closest.distance(point)))
		})
		.min_by(|(_, a), (_, b)| a.total_cmp(b))
		.map(|(wall, _)| wall)
}

/// Möller–Trumbore, hitting both faces so a ray from inside a wall still finds it.
fn ray_triangle_distance(origin: Vec3, direction: Vec3, [a, b, c]: [Vec3; 3]) -> Option<f32> {
	let edge_ab = b - a;
	let edge_ac = c - a;
	let direction_cross_edge = direction.cross(edge_ac);
	let determinant = edge_ab.dot(direction_cross_edge);
	// A ray parallel to the triangle plane never crosses it.
	if determinant.abs() < f32::EPSILON {
		return None;
	}

	let to_origin = origin - a;
	let origin_cross_edge = to_origin.cross(edge_ab);
	let u = to_origin.dot(direction_cross_edge) / determinant;
	let v = direction.dot(origin_cross_edge) / determinant;
	let distance = edge_ac.dot(origin_cross_edge) / determinant;
	(u >= 0.0 && v >= 0.0 && u + v <= 1.0 && distance >= 0.0).then_some(distance)
}

fn draw_wall_highlights(
	house: Res<House>,
	hovered: Res<HoveredWall>,
	selected: Res<SelectedWall>,
	mut gizmos: Gizmos,
) {
	let graph = house.graph();
	if let Some(wall) = hovered.0.filter(|&wall| Some(wall) != selected.0)
		&& let Some(line) = wall_top_line(graph, wall)
	{
		gizmos.linestrip(line, HOVER_COLOR);
	}
	if let Some(line) = selected.0.and_then(|wall| wall_top_line(graph, wall)) {
		// The line runs from one node to the other, so its ends are the wall's handles.
		for end in [line.first(), line.last()].into_iter().flatten() {
			gizmos.sphere(
				Isometry3d::from_translation(*end),
				HANDLE_RADIUS_METRES,
				SELECTED_COLOR,
			);
		}
		gizmos.linestrip(line, SELECTED_COLOR);
	}
}

/// The wall's centre line along its top in Bevy space, or `None` for a stale handle.
fn wall_top_line(graph: &WallGraph, wall: Wall) -> Option<Vec<Vec3>> {
	let top = graph.wall_dimensions(wall)?.height + HIGHLIGHT_LIFT_METRES;
	let points = graph.sample_wall(wall, SAMPLE_DEVIATION_METRES).ok()?;
	Some(
		points
			.into_iter()
			.map(|point| graph_to_bevy(point.extend(top)))
			.collect(),
	)
}

#[cfg(test)]
#[path = "tests/wall_tool_selection.rs"]
mod tests;
