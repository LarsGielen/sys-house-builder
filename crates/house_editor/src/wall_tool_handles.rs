use bevy::{ecs::system::SystemParam, prelude::*};
use wall_graph::{Wall, WallCurve, WallGraph, WallNodeId, arc_midpoint, arc_through_bend};

use crate::{
	coordinates::{bevy_to_graph, graph_to_bevy},
	edit::{BendProposal, EditPreview, MoveProposal, Proposal},
	ground_pointer::GroundPointer,
	history::DocumentRestored,
	house::House,
	snapping::{SnapSettings, SnappedPointer, snap_to_grid, snapping_disabled},
	text_fields::TextFocus,
	wall_tool_active::{Tool, tool_active},
	wall_tool_selection::{HoveredTarget, SelectedTarget, WallSelectionCarryOver},
};

const DRAG_THRESHOLD_PIXELS: f32 = 6.0;
// Larger than the drawn handle, so a handle is easy to grab from a distance.
const HANDLE_PICK_RADIUS_METRES: f32 = 0.2;
const HANDLE_DRAW_RADIUS_METRES: f32 = 0.1;
// Lifts handles above the wall top so the wall mesh does not hide them.
const HANDLE_LIFT_METRES: f32 = 0.02;
const BEND_COLOR: Color = Color::srgb(0.85, 0.4, 0.95);
const HOVER_COLOR: Color = Color::srgb(1.0, 0.85, 0.2);

/// Drags the handles of the selected wall: its ends and junctions, its bend, or the wall
/// as a whole. A drag is one draft proposal; releasing commits it as one history entry.
pub struct WallToolHandlesPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WallToolHandlesSystems {
	/// Finds the handle under the pointer. Runs before selection so that grabbing a handle
	/// is not also a click that changes the selection.
	Hover,
	Drag,
}

impl Plugin for WallToolHandlesPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<HoveredHandle>()
			.init_resource::<HandleDrag>()
			.init_resource::<EditPreview>()
			.init_resource::<WallSelectionCarryOver>()
			.add_message::<DocumentRestored>()
			.add_systems(
				PostUpdate,
				update_hovered_handle.in_set(WallToolHandlesSystems::Hover),
			)
			.add_systems(
				PostUpdate,
				(
					discard_drag_after_restore,
					update_handle_drag.run_if(tool_active(Tool::Select)),
					cancel_handle_drag.run_if(not(tool_active(Tool::Select))),
				)
					.chain()
					.in_set(WallToolHandlesSystems::Drag),
			)
			.add_systems(PostUpdate, draw_handles.after(WallToolHandlesSystems::Drag));
	}
}

/// What a drag moves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WallHandle {
	/// An end of the selected wall. A junction shared with other walls moves them all.
	Endpoint(WallNodeId),
	/// The middle of the selected wall, which bends it.
	Bend(Wall),
	/// The wall as a whole.
	Body(Wall),
}

/// The end or bend handle under the pointer. The body is not a visible handle, so it is
/// decided when a press arrives.
#[derive(Resource, Default, PartialEq)]
pub struct HoveredHandle(pub Option<WallHandle>);

#[derive(Resource, Default, Clone, Copy, Debug, PartialEq)]
enum HandleDrag {
	#[default]
	Idle,
	/// The button is down. It is a click until the pointer moves past the drag threshold.
	Pressed {
		handle: WallHandle,
		/// The selected wall, whose selection follows it to its rebuilt handle.
		wall: Wall,
		/// Graph metres where the pointer grabbed, so the wall does not jump.
		grab: Vec2,
		/// Viewport pixels, to measure the drag threshold.
		press_position: Vec2,
		dragged: bool,
	},
}

/// Where the handles of `wall` are, in graph metres, at the height they are drawn.
struct WallHandlePositions {
	origin: Vec3,
	destination: Vec3,
	bend: Vec3,
}

fn handle_positions(graph: &WallGraph, wall: Wall) -> Option<WallHandlePositions> {
	let height = graph.wall_dimensions(wall)?.height + HANDLE_LIFT_METRES;
	let origin = graph.node_position(wall.origin())?;
	let destination = graph.node_position(wall.destination())?;
	let sweep = match graph.wall_curve(wall)? {
		WallCurve::Straight { .. } => 0.0,
		WallCurve::CircularArc { sweep, .. } => sweep as f32,
	};
	Some(WallHandlePositions {
		origin: origin.extend(height),
		destination: destination.extend(height),
		bend: arc_midpoint(origin, destination, sweep).extend(height),
	})
}

fn update_hovered_handle(
	tool: Res<crate::wall_tool_active::ActiveTool>,
	house: Res<House>,
	selected: Res<SelectedTarget>,
	ground_pointer: Res<GroundPointer>,
	mut hovered: ResMut<HoveredHandle>,
) {
	let handle = (tool.0 == Tool::Select)
		.then(|| {
			let (wall, ray) = (selected.wall()?, ground_pointer.ray?);
			let positions = handle_positions(house.graph(), wall)?;
			let origin = bevy_to_graph(ray.origin);
			let direction = bevy_to_graph(*ray.direction);
			[
				(positions.origin, WallHandle::Endpoint(wall.origin())),
				(
					positions.destination,
					WallHandle::Endpoint(wall.destination()),
				),
				(positions.bend, WallHandle::Bend(wall)),
			]
			.into_iter()
			.filter_map(|(center, handle)| {
				Some((ray_sphere_distance(origin, direction, center)?, handle))
			})
			.min_by(|(first, _), (second, _)| first.total_cmp(second))
			.map(|(_, handle)| handle)
		})
		.flatten();
	hovered.set_if_neq(HoveredHandle(handle));
}

/// The distance along a ray to the sphere's near surface, if the ray reaches it.
fn ray_sphere_distance(origin: Vec3, direction: Vec3, center: Vec3) -> Option<f32> {
	let to_center = center - origin;
	let along = to_center.dot(direction);
	let off_axis_squared = to_center.length_squared() - along * along;
	(along >= 0.0 && off_axis_squared <= HANDLE_PICK_RADIUS_METRES.powi(2)).then_some(along)
}

#[derive(SystemParam)]
struct HandleInput<'w, 's> {
	mouse: Res<'w, ButtonInput<MouseButton>>,
	keyboard: Res<'w, ButtonInput<KeyCode>>,
	ground_pointer: Res<'w, GroundPointer>,
	snapped_pointer: Res<'w, SnappedPointer>,
	snap_settings: Res<'w, SnapSettings>,
	hovered_handle: Res<'w, HoveredHandle>,
	hovered_target: Res<'w, HoveredTarget>,
	selected: Res<'w, SelectedTarget>,
	typing: TextFocus<'w, 's>,
}

fn update_handle_drag(
	input: HandleInput,
	mut house: ResMut<House>,
	mut drag: ResMut<HandleDrag>,
	mut preview: ResMut<EditPreview>,
	mut carry_over: ResMut<WallSelectionCarryOver>,
) {
	// Escape belongs to a focused text field, which reverts its own edit.
	if input.keyboard.just_pressed(KeyCode::Escape) && !input.typing.active() {
		drag.set_if_neq(HandleDrag::Idle);
		if preview.holds_handle_drag() {
			preview.state = None;
		}
		return;
	}

	if *drag == HandleDrag::Idle {
		let pressed = input.mouse.just_pressed(MouseButton::Left);
		let (Some(hit), Some(wall)) = (input.ground_pointer.hit, input.selected.wall()) else {
			return;
		};
		let handle = input.hovered_handle.0.or_else(|| {
			(input.hovered_target.wall() == Some(wall)).then_some(WallHandle::Body(wall))
		});
		if let (true, Some(handle)) = (pressed, handle) {
			*drag = HandleDrag::Pressed {
				handle,
				wall,
				grab: hit.graph_position,
				press_position: hit.viewport_position,
				dragged: false,
			};
		}
		return;
	}

	let HandleDrag::Pressed {
		handle,
		wall,
		grab,
		press_position,
		dragged,
	} = *drag
	else {
		return;
	};
	let dragged = dragged
		|| input.ground_pointer.hit.is_some_and(|hit| {
			hit.viewport_position.distance(press_position) > DRAG_THRESHOLD_PIXELS
		});
	let released = input.mouse.just_released(MouseButton::Left);
	if !dragged {
		*drag = if released {
			HandleDrag::Idle
		} else {
			HandleDrag::Pressed {
				handle,
				wall,
				grab,
				press_position,
				dragged,
			}
		};
		return;
	}
	*drag = HandleDrag::Pressed {
		handle,
		wall,
		grab,
		press_position,
		dragged,
	};

	let proposal = drag_proposal(handle, wall, grab, &input, house.graph());
	// Reborrowing retains Bevy's change tracking until the helper actually writes.
	if let Some(proposal) = proposal {
		EditPreview::update(
			preview.reborrow(),
			proposal,
			house.revision(),
			house.committed(),
		);
	}

	if released {
		// A rejected drag is abandoned, leaving the layout as it was.
		if let Some(proposal) = preview
			.state
			.as_ref()
			.map(|state| state.proposal)
			.filter(|_| preview.holds_handle_drag())
			&& let Some(candidate) = preview.take_valid(proposal, house.revision())
		{
			carry_over.0 = candidate
				.report()
				.rebuilt_walls
				.iter()
				.find(|&&(old, _)| old == wall)
				.map(|&(_, new)| new);
			house.commit(candidate);
		}
		if preview.holds_handle_drag() {
			preview.state = None;
		}
		*drag = HandleDrag::Idle;
	}
}

/// The edit the pointer's position asks for, or `None` while it has nothing to go on.
fn drag_proposal(
	handle: WallHandle,
	wall: Wall,
	grab: Vec2,
	input: &HandleInput,
	graph: &WallGraph,
) -> Option<Proposal> {
	let hit = input.ground_pointer.hit?;
	match handle {
		WallHandle::Endpoint(node) => Some(Proposal::Move(MoveProposal {
			first: (node, input.snapped_pointer.target?.position),
			second: None,
			grabbed: Some(wall),
		})),
		WallHandle::Body(wall) => {
			let origin = graph.node_position(wall.origin())?;
			let destination = graph.node_position(wall.destination())?;
			let mut delta = hit.graph_position - grab;
			// The wall's own start sits on the grid, so the whole wall moves in grid steps.
			if !snapping_disabled(&input.keyboard) {
				delta = snap_to_grid(origin + delta, input.snap_settings.grid_spacing()) - origin;
			}
			Some(Proposal::Move(MoveProposal {
				first: (wall.origin(), origin + delta),
				second: Some((wall.destination(), destination + delta)),
				grabbed: Some(wall),
			}))
		}
		WallHandle::Bend(wall) => {
			let origin = graph.node_position(wall.origin())?;
			let destination = graph.node_position(wall.destination())?;
			// A bend on the chord is a straight wall.
			let sweep = match arc_through_bend(origin, hit.graph_position, destination) {
				Ok(sweep) => sweep,
				Err(wall_graph::ArcError::BendOnChord) => 0.0,
				Err(_) => return None,
			};
			Some(Proposal::Bend(BendProposal { wall, sweep }))
		}
	}
}

/// Leaving the tool abandons a drag instead of resuming it on return.
fn cancel_handle_drag(mut drag: ResMut<HandleDrag>, mut preview: ResMut<EditPreview>) {
	drag.set_if_neq(HandleDrag::Idle);
	if preview.holds_handle_drag() {
		preview.state = None;
	}
}

/// A drag remembers positions from before an undo or redo, which would then be committed
/// over the restored house.
fn discard_drag_after_restore(
	mut restored: MessageReader<DocumentRestored>,
	mut drag: ResMut<HandleDrag>,
) {
	if restored.read().count() > 0 {
		drag.set_if_neq(HandleDrag::Idle);
	}
}

/// The bend handle, which the selection's own highlight does not draw, and an emphasis on
/// whichever handle the pointer is over.
fn draw_handles(
	tool: Res<crate::wall_tool_active::ActiveTool>,
	house: Res<House>,
	selected: Res<SelectedTarget>,
	hovered: Res<HoveredHandle>,
	mut gizmos: Gizmos,
) {
	let Some(wall) = selected.wall().filter(|_| tool.0 == Tool::Select) else {
		return;
	};
	let Some(positions) = handle_positions(house.graph(), wall) else {
		return;
	};
	let mut sphere = |position: Vec3, color: Color| {
		gizmos.sphere(
			Isometry3d::from_translation(graph_to_bevy(position)),
			HANDLE_DRAW_RADIUS_METRES,
			color,
		);
	};
	sphere(positions.bend, BEND_COLOR);
	if let Some(handle) = hovered.0 {
		let (position, color) = match handle {
			WallHandle::Endpoint(node) if node == wall.origin() => (positions.origin, HOVER_COLOR),
			WallHandle::Endpoint(_) => (positions.destination, HOVER_COLOR),
			WallHandle::Bend(_) | WallHandle::Body(_) => (positions.bend, HOVER_COLOR),
		};
		sphere(position + Vec3::Z * HANDLE_LIFT_METRES, color);
	}
}

#[cfg(test)]
#[path = "tests/wall_tool_handles.rs"]
mod tests;
