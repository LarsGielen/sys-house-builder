use bevy::{ecs::system::SystemParam, prelude::*};
use wall_graph::{OpeningId, Wall, WallGraph, WallNodeId};
use wall_mesh::{SurfaceSource, WallMesh};

use crate::{
	coordinates::{bevy_to_graph, graph_to_bevy},
	ground_pointer::GroundPointer,
	house::House,
	opening_area::{OpeningArea, opening_area},
	text_fields::TextFocus,
	wall_tool_active::{ActiveTool, Tool},
};

const HOVER_COLOR: Color = Color::srgb(1.0, 0.85, 0.2);
const SELECTED_COLOR: Color = Color::srgb(0.2, 0.6, 1.0);
const HANDLE_RADIUS_METRES: f32 = 0.1;
const SIZE_HANDLE_RADIUS_METRES: f32 = 0.06;
// A jamb lies just inside the opening's area, so the area wins ties this close.
const OCCLUSION_TOLERANCE_METRES: f32 = 1e-3;
// Lifts the highlight above the wall top so the wall mesh does not hide it.
const HIGHLIGHT_LIFT_METRES: f32 = 0.02;
const SAMPLE_DEVIATION_METRES: f32 = 0.01;

/// Hovers and selects committed walls and openings with the selection tool.
pub struct WallToolSelectionPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WallToolSelectionSystems {
	Pick,
	Presentation,
}

impl Plugin for WallToolSelectionPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<HoveredTarget>()
			.init_resource::<SelectedTarget>()
			.init_resource::<WallSelectionCarryOver>()
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
				(draw_wall_highlights, draw_opening_highlights)
					.in_set(WallToolSelectionSystems::Presentation),
			);
	}
}

/// The target under the pointer, recomputed from committed geometry every frame.
#[derive(Resource, Default, PartialEq)]
pub struct HoveredTarget(pub Option<SelectionTarget>);

/// The selected target. Opening IDs survive ordinary edits. Wall handles are cleared on
/// commits unless the committing edit explicitly carries the wall selection over.
#[derive(Resource, Default, PartialEq)]
pub struct SelectedTarget(pub Option<SelectionTarget>);

/// What a pointer ray picked. A selection can refer to exactly one wall or opening.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SelectionTarget {
	Wall(Wall),
	Opening(OpeningId),
}

impl SelectionTarget {
	pub fn wall(self) -> Option<Wall> {
		match self {
			Self::Wall(wall) => Some(wall),
			Self::Opening(_) => None,
		}
	}

	pub fn opening(self) -> Option<OpeningId> {
		match self {
			Self::Opening(opening) => Some(opening),
			Self::Wall(_) => None,
		}
	}
}

impl SelectedTarget {
	pub fn wall(&self) -> Option<Wall> {
		self.0.and_then(SelectionTarget::wall)
	}

	pub fn opening(&self) -> Option<OpeningId> {
		self.0.and_then(SelectionTarget::opening)
	}
}

impl HoveredTarget {
	pub fn wall(&self) -> Option<Wall> {
		self.0.and_then(SelectionTarget::wall)
	}

	pub fn opening(&self) -> Option<OpeningId> {
		self.0.and_then(SelectionTarget::opening)
	}
}

/// A wall that a commit leaves valid, such as one whose dimensions changed. The committing
/// edit sets it, since only that edit knows the handle still means the same wall.
#[derive(Resource, Default)]
pub struct WallSelectionCarryOver(pub Option<Wall>);

fn clear_selection_after_commit(
	house: Res<House>,
	mut carry_over: ResMut<WallSelectionCarryOver>,
	mut selected: ResMut<SelectedTarget>,
) {
	let kept = carry_over
		.0
		.take()
		.filter(|&wall| house.graph().wall_dimensions(wall).is_some())
		.map(SelectionTarget::Wall)
		.or_else(|| {
			selected
				.opening()
				.filter(|&opening| house.graph().opening(opening).is_some())
				.map(SelectionTarget::Opening)
		});
	selected.set_if_neq(SelectedTarget(kept));
}

#[derive(SystemParam)]
struct SelectionInput<'w, 's> {
	mouse: Res<'w, ButtonInput<MouseButton>>,
	keyboard: Res<'w, ButtonInput<KeyCode>>,
	ground_pointer: Res<'w, GroundPointer>,
	typing: TextFocus<'w, 's>,
}

fn update_selection(
	active_tool: Res<ActiveTool>,
	input: SelectionInput,
	house: Res<House>,
	mut hovered_target: ResMut<HoveredTarget>,
	mut selected: ResMut<SelectedTarget>,
) {
	if active_tool.0 != Tool::Select {
		hovered_target.set_if_neq(HoveredTarget(None));
		selected.set_if_neq(SelectedTarget(None));
		return;
	}

	let hovered = input.ground_pointer.ray.and_then(|ray| {
		pick_target(
			house.graph(),
			house.mesh(),
			bevy_to_graph(ray.origin),
			bevy_to_graph(*ray.direction),
		)
	});
	hovered_target.set_if_neq(HoveredTarget(hovered));

	// Escape wins over a click in the same frame, as cancelling does in the drawing tool.
	// While typing, Escape belongs to the text field.
	if input.keyboard.just_pressed(KeyCode::Escape) && !input.typing.active() {
		selected.set_if_neq(SelectedTarget(None));
		return;
	}
	// A ray exists only when the viewport owns the pointer, so UI clicks never get here.
	if input.mouse.just_pressed(MouseButton::Left) && input.ground_pointer.ray.is_some() {
		selected.set_if_neq(SelectedTarget(hovered));
	}
}

/// Picks what a graph-space ray points at. An opening's area takes precedence over its host
/// wall, unless something nearer hides it, and its jambs, head, and sill select the opening.
pub fn pick_target(
	graph: &WallGraph,
	mesh: &WallMesh,
	origin: Vec3,
	direction: Vec3,
) -> Option<SelectionTarget> {
	let surface = nearest_surface(mesh, origin, direction);
	if let Some((area_distance, opening)) = nearest_opening_area(graph, origin, direction)
		&& surface.is_none_or(|(surface_distance, _)| {
			area_distance <= surface_distance + OCCLUSION_TOLERANCE_METRES
		}) {
		return Some(SelectionTarget::Opening(opening));
	}

	let (distance, source) = surface?;
	match source {
		SurfaceSource::Opening(opening) => Some(SelectionTarget::Opening(opening)),
		_ => wall_of_surface(graph, source, (origin + direction * distance).truncate())
			.map(SelectionTarget::Wall),
	}
}

/// The nearest opening whose area the ray crosses, with the distance to it.
fn nearest_opening_area(
	graph: &WallGraph,
	origin: Vec3,
	direction: Vec3,
) -> Option<(f32, OpeningId)> {
	graph
		.openings()
		.filter_map(|(id, opening)| {
			let [a, b, c, d] = opening_area(graph, opening.wall, &opening.spec)?.corners();
			let distance = ray_triangle_distance(origin, direction, [a, b, c])
				.or_else(|| ray_triangle_distance(origin, direction, [a, c, d]))?;
			Some((distance, id))
		})
		.min_by(|(a, _), (b, _)| a.total_cmp(b))
}

/// The wall owning the nearest mesh surface along a ray, and where the ray meets it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WallHit {
	pub wall: Wall,
	/// Graph metres.
	pub position: Vec3,
}

pub fn pick_wall_hit(
	graph: &WallGraph,
	mesh: &WallMesh,
	origin: Vec3,
	direction: Vec3,
) -> Option<WallHit> {
	let (distance, source) = nearest_surface(mesh, origin, direction)?;
	let position = origin + direction * distance;
	let wall = wall_of_surface(graph, source, position.truncate())?;
	Some(WallHit { wall, position })
}

/// The nearest mesh triangle along a graph-space ray, with its distance and what it belongs to.
fn nearest_surface(mesh: &WallMesh, origin: Vec3, direction: Vec3) -> Option<(f32, SurfaceSource)> {
	let positions = mesh.positions();
	let indices = mesh.indices();
	mesh.sections()
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
		.min_by(|(a, _), (b, _)| a.total_cmp(b))
}

/// The wall a surface belongs to. A hit on a junction picks the wall nearest `point`.
fn wall_of_surface(graph: &WallGraph, source: SurfaceSource, point: Vec2) -> Option<Wall> {
	match source {
		SurfaceSource::Wall(wall) => Some(wall),
		SurfaceSource::Opening(opening) => Some(graph.opening(opening)?.wall),
		SurfaceSource::Junction(node) => nearest_wall_at_junction(graph, node, point),
	}
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

fn draw_opening_highlights(
	house: Res<House>,
	hovered: Res<HoveredTarget>,
	selected: Res<SelectedTarget>,
	mut gizmos: Gizmos,
) {
	let graph = house.graph();
	if let Some(area) = hovered
		.opening()
		.filter(|&opening| Some(opening) != selected.opening())
		.and_then(|opening| committed_opening_area(graph, opening))
	{
		draw_outline(&area, HOVER_COLOR, &mut gizmos);
	}
	if let Some(area) = selected
		.opening()
		.and_then(|opening| committed_opening_area(graph, opening))
	{
		draw_outline(&area, SELECTED_COLOR, &mut gizmos);
		draw_opening_handles(&area, &mut gizmos);
	}
}

/// `None` for an opening the graph no longer has.
fn committed_opening_area(graph: &WallGraph, opening: OpeningId) -> Option<OpeningArea> {
	let opening = graph.opening(opening)?;
	opening_area(graph, opening.wall, &opening.spec)
}

fn draw_outline(area: &OpeningArea, color: Color, gizmos: &mut Gizmos) {
	let [first, second, third, fourth] = area.corners().map(graph_to_bevy);
	gizmos.linestrip([first, second, third, fourth, first], color);
}

/// The opening can be dragged to move it. The size markers are decorative; dimensions
/// are edited in the panel.
fn draw_opening_handles(area: &OpeningArea, gizmos: &mut Gizmos) {
	let [bottom_start, bottom_end, top_end, top_start] = area.corners();
	gizmos.sphere(
		Isometry3d::from_translation(graph_to_bevy(area.center())),
		HANDLE_RADIUS_METRES,
		SELECTED_COLOR,
	);
	let middle = |a: Vec3, b: Vec3| graph_to_bevy((a + b) / 2.0);
	for size_handle in [
		middle(bottom_start, top_start),
		middle(bottom_end, top_end),
		middle(top_start, top_end),
	] {
		gizmos.sphere(
			Isometry3d::from_translation(size_handle),
			SIZE_HANDLE_RADIUS_METRES,
			SELECTED_COLOR,
		);
	}
}

fn draw_wall_highlights(
	house: Res<House>,
	hovered: Res<HoveredTarget>,
	selected: Res<SelectedTarget>,
	mut gizmos: Gizmos,
) {
	let graph = house.graph();
	if let Some(wall) = hovered.wall().filter(|&wall| Some(wall) != selected.wall())
		&& let Some(line) = wall_top_line(graph, wall)
	{
		gizmos.linestrip(line, HOVER_COLOR);
	}
	if let Some(line) = selected.wall().and_then(|wall| wall_top_line(graph, wall)) {
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
