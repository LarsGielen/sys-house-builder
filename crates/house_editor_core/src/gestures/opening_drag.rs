use std::ops::RangeInclusive;

use glam::Vec2;
use house_document::OpeningEditProposal;
use wall_graph::{OpeningId, OpeningSpec, Wall, WallGraph};

use crate::interaction_settings::InteractionSettings;
use crate::{
	action::{Action, ActionState},
	openings::area::closest_distance_along_wall,
	openings::snap::{OpeningSnapOptions, snap_opening},
	pointing::pointer::PointerOnGround,
};

/// Where the opening drag gesture is.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub enum OpeningDrag {
	#[default]
	Idle,
	/// The button is down. It is a click until the pointer moves past the drag threshold.
	Pressed {
		opening: OpeningId,
		original: OpeningSpec,
		/// Where along the wall the pointer grabbed the opening, so it does not jump.
		grab_distance: f64,
		/// Viewport pixels, to measure the drag threshold.
		press_position: Vec2,
		dragged: bool,
	},
}

/// Starts a drag on the selected, hovered opening, tracks the threshold, and reports the
/// release. A release off the ground still finishes the drag.
pub fn advance_opening_drag(
	settings: &InteractionSettings,
	drag: OpeningDrag,
	actions: &ActionState,
	hit: Option<PointerOnGround>,
	hovered_opening: Option<OpeningId>,
	selected_opening: Option<OpeningId>,
	graph: &WallGraph,
) -> (OpeningDrag, bool) {
	match drag {
		OpeningDrag::Idle => {
			let started = actions
				.just_pressed(Action::ToolPress)
				.then(|| {
					let opening = hovered_opening.filter(|&id| Some(id) == selected_opening)?;
					let hit = hit?;
					let current = graph.opening(opening)?;
					Some(OpeningDrag::Pressed {
						opening,
						original: current.spec,
						grab_distance: closest_distance_along_wall(
							graph,
							current.wall,
							hit.graph_position,
						)?,
						press_position: hit.viewport_position,
						dragged: false,
					})
				})
				.flatten();
			(started.unwrap_or(drag), false)
		}
		OpeningDrag::Pressed {
			opening,
			original,
			grab_distance,
			press_position,
			dragged,
		} => {
			let dragged = dragged
				|| hit.is_some_and(|hit| {
					hit.viewport_position.distance(press_position)
						> settings.drag_threshold_pixels()
				});
			(
				OpeningDrag::Pressed {
					opening,
					original,
					grab_distance,
					press_position,
					dragged,
				},
				actions.just_released(Action::ToolPress),
			)
		}
	}
}

/// The edit a drag asks for with the pointer at `hit`, snapped when `options` is given.
/// `bounds` yields the room available on the opening's wall and is only asked when snapping.
pub fn opening_drag_proposal(
	graph: &WallGraph,
	opening: OpeningId,
	original: OpeningSpec,
	grab_distance: f64,
	hit: PointerOnGround,
	options: Option<OpeningSnapOptions>,
	bounds: impl FnOnce(Wall) -> Option<RangeInclusive<f64>>,
) -> Option<OpeningEditProposal> {
	let wall = graph.opening(opening)?.wall;
	let distance = closest_distance_along_wall(graph, wall, hit.graph_position)?;
	let raw = OpeningSpec {
		center_distance: original.center_distance + distance - grab_distance,
		..original
	};
	Some(OpeningEditProposal {
		opening,
		spec: options.map_or(raw, |options| {
			snap_opening(graph, wall, bounds(wall), raw, options)
		}),
	})
}

#[cfg(test)]
#[path = "../tests/gestures/opening_drag.rs"]
mod tests;
