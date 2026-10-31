use bevy::{ecs::system::SystemParam, prelude::*};
use house_editor_core::retained_selection;
use house_editor_core::{OpeningArea, opening_area};
pub use house_editor_core::{SelectionTarget, pick_target};
use wall_graph::{OpeningId, Wall, WallGraph};

use crate::{
	document::house::House,
	input::{Action, ActionState, ActionsPlugin},
	tools::active::{ActiveTool, Tool},
	tools::handles::HoveredHandle,
	viewport::coordinates::{bevy_to_graph, graph_to_bevy},
	viewport::ground_pointer::GroundPointer,
};

const HOVER_COLOR: Color = Color::srgb(1.0, 0.85, 0.2);
const SELECTED_COLOR: Color = Color::srgb(0.2, 0.6, 1.0);
const HANDLE_RADIUS_METRES: f32 = 0.1;
const SIZE_HANDLE_RADIUS_METRES: f32 = 0.06;
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
		ActionsPlugin::ensure(app);
		app.init_resource::<HoveredTarget>()
			.init_resource::<HoveredHandle>()
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
	let kept = retained_selection(house.graph(), carry_over.0.take(), selected.opening());
	selected.set_if_neq(SelectedTarget(kept));
}

#[derive(SystemParam)]
struct SelectionInput<'w> {
	actions: Res<'w, ActionState>,
	ground_pointer: Res<'w, GroundPointer>,
	/// A press on a handle of the selected wall starts a drag, not a new selection.
	hovered_handle: Res<'w, HoveredHandle>,
}

fn update_selection(
	active_tool: Res<ActiveTool>,
	input: SelectionInput,
	house: Res<House>,
	mut hovered_target: ResMut<HoveredTarget>,
	mut selected: ResMut<SelectedTarget>,
	mut last_picked: Local<Option<(Option<Ray3d>, u64)>>,
) {
	if active_tool.0 != Tool::Select {
		hovered_target.set_if_neq(HoveredTarget(None));
		selected.set_if_neq(SelectedTarget(None));
		*last_picked = None;
		return;
	}

	// Picking tests every triangle, so it only reruns when the ray or the house changed.
	let pick_inputs = (input.ground_pointer.ray, house.revision());
	if *last_picked != Some(pick_inputs) {
		let hovered = pick_inputs.0.and_then(|ray| {
			pick_target(
				house.graph(),
				house.mesh(),
				bevy_to_graph(ray.origin),
				bevy_to_graph(*ray.direction),
			)
		});
		hovered_target.set_if_neq(HoveredTarget(hovered));
		*last_picked = Some(pick_inputs);
	}

	// Escape wins over a click in the same frame, as cancelling does in the drawing tool.
	// While typing, Escape belongs to the text field.
	if input.actions.just_pressed(Action::Cancel) {
		selected.set_if_neq(SelectedTarget(None));
		return;
	}
	// A ray exists only when the viewport owns the pointer, so UI clicks never get here.
	if input.actions.just_pressed(Action::ToolPress)
		&& input.ground_pointer.ray.is_some()
		&& input.hovered_handle.0.is_none()
	{
		selected.set_if_neq(SelectedTarget(hovered_target.0));
	}
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
#[path = "../tests/tools/selection.rs"]
mod tests;
