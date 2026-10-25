use glam::{Vec2, Vec3};
use house_document::{BendProposal, MoveProposal, Proposal};
use wall_graph::{Wall, WallCurve, WallGraph, WallNodeId, arc_midpoint, arc_through_bend};

use crate::interaction_settings::InteractionSettings;
use crate::{
	action::{Action, ActionState},
	pointing::pointer::PointerOnGround,
	pointing::snapping::{SnapTarget, snap_to_grid},
};

// Larger than the drawn handle, so a handle is easy to grab from a distance.
const HANDLE_PICK_RADIUS_METRES: f32 = 0.2;
// Lifts handles above the wall top so the wall mesh does not hide them.
pub const HANDLE_LIFT_METRES: f32 = 0.02;

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

/// Where the handle gesture is.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub enum HandleDrag {
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

/// A drag that has passed the threshold, and so has an edit to preview.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActiveDrag {
	pub handle: WallHandle,
	pub wall: Wall,
	pub grab: Vec2,
}

/// The result of one frame of the handle gesture.
#[derive(Debug, PartialEq)]
pub struct HandleStep {
	pub drag: HandleDrag,
	pub active: Option<ActiveDrag>,
	/// The button was released this frame while dragging, which commits the preview.
	pub released: bool,
}

/// Starts a drag on a handle (or the selected wall's body) when pressed, tracks it past the
/// click threshold, and reports its release. The release counts even if the pointer has left
/// the ground, so a drag that ends over the panel still finishes.
pub fn advance_handle_drag(
	settings: &InteractionSettings,
	drag: HandleDrag,
	actions: &ActionState,
	hit: Option<PointerOnGround>,
	selected_wall: Option<Wall>,
	hovered_handle: Option<WallHandle>,
	hovered_wall: Option<Wall>,
) -> HandleStep {
	let idle = |drag| HandleStep {
		drag,
		active: None,
		released: false,
	};
	let HandleDrag::Pressed {
		handle,
		wall,
		grab,
		press_position,
		dragged,
	} = drag
	else {
		let pressed = actions.just_pressed(Action::ToolPress);
		let (Some(hit), Some(wall)) = (hit, selected_wall) else {
			return idle(drag);
		};
		let handle = hovered_handle
			.or_else(|| (hovered_wall == Some(wall)).then_some(WallHandle::Body(wall)));
		return match (pressed, handle) {
			(true, Some(handle)) => idle(HandleDrag::Pressed {
				handle,
				wall,
				grab: hit.graph_position,
				press_position: hit.viewport_position,
				dragged: false,
			}),
			_ => idle(drag),
		};
	};

	let dragged = dragged
		|| hit.is_some_and(|hit| {
			hit.viewport_position.distance(press_position) > settings.drag_threshold_pixels()
		});
	let released = actions.just_released(Action::ToolPress);
	if !dragged {
		return idle(if released {
			HandleDrag::Idle
		} else {
			HandleDrag::Pressed {
				handle,
				wall,
				grab,
				press_position,
				dragged,
			}
		});
	}
	HandleStep {
		drag: HandleDrag::Pressed {
			handle,
			wall,
			grab,
			press_position,
			dragged,
		},
		active: Some(ActiveDrag { handle, wall, grab }),
		released,
	}
}

/// Where the handles of a wall are, in graph metres, at the height they are drawn.
pub struct WallHandlePositions {
	pub origin: Vec3,
	pub destination: Vec3,
	pub bend: Vec3,
}

pub fn handle_positions(graph: &WallGraph, wall: Wall) -> Option<WallHandlePositions> {
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

/// The end or bend handle of `wall` that a graph-space ray reaches first.
pub fn handle_under_ray(
	graph: &WallGraph,
	wall: Wall,
	origin: Vec3,
	direction: Vec3,
) -> Option<WallHandle> {
	let positions = handle_positions(graph, wall)?;
	[
		(positions.origin, WallHandle::Endpoint(wall.origin())),
		(
			positions.destination,
			WallHandle::Endpoint(wall.destination()),
		),
		(positions.bend, WallHandle::Bend(wall)),
	]
	.into_iter()
	.filter_map(|(center, handle)| Some((ray_sphere_distance(origin, direction, center)?, handle)))
	.min_by(|(first, _), (second, _)| first.total_cmp(second))
	.map(|(_, handle)| handle)
}

/// The distance along a ray to the sphere's near surface, if the ray reaches it.
fn ray_sphere_distance(origin: Vec3, direction: Vec3, center: Vec3) -> Option<f32> {
	let to_center = center - origin;
	let along = to_center.dot(direction);
	let off_axis_squared = to_center.length_squared() - along * along;
	(along >= 0.0 && off_axis_squared <= HANDLE_PICK_RADIUS_METRES.powi(2)).then_some(along)
}

/// How the pointer maps to a snapped or free position while dragging.
pub struct DragPointer {
	pub hit: PointerOnGround,
	pub snap_target: Option<SnapTarget>,
	pub snapping_disabled: bool,
	pub grid_spacing: f32,
}

/// The edit the pointer's position asks for, or `None` while it has nothing to go on.
pub fn drag_proposal(
	active: ActiveDrag,
	pointer: &DragPointer,
	graph: &WallGraph,
) -> Option<Proposal> {
	let ActiveDrag { handle, wall, grab } = active;
	match handle {
		WallHandle::Endpoint(node) => Some(Proposal::Move(MoveProposal {
			first: (node, pointer.snap_target?.position),
			second: None,
			grabbed: Some(wall),
		})),
		WallHandle::Body(wall) => {
			let origin = graph.node_position(wall.origin())?;
			let destination = graph.node_position(wall.destination())?;
			let mut delta = pointer.hit.graph_position - grab;
			// The wall's own start sits on the grid, so the whole wall moves in grid steps.
			if !pointer.snapping_disabled {
				delta = snap_to_grid(origin + delta, pointer.grid_spacing) - origin;
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
			let sweep = match arc_through_bend(origin, pointer.hit.graph_position, destination) {
				Ok(sweep) => sweep,
				Err(wall_graph::ArcError::BendOnChord) => 0.0,
				Err(_) => return None,
			};
			Some(Proposal::Bend(BendProposal { wall, sweep }))
		}
	}
}

#[cfg(test)]
#[path = "../tests/gestures/handle_drag.rs"]
mod tests;
