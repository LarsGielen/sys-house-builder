use glam::Vec2;
use house_document::{ArcDefinition, ArcProposal};
use wall_graph::{WallDimensions, WallGraph, WallNodeId};

use crate::interaction_settings::InteractionSettings;
use crate::{
	action::{Action, ActionState},
	pointing::pointer::PointerOnGround,
	pointing::snapping::{SnapSource, SnapTarget},
};

/// How the arc is defined. Switching modes abandons the arc in progress.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArcMode {
	/// Endpoints, then a bend position.
	#[default]
	Free,
	/// Continue one wall end; the other endpoint is free.
	OneTangent,
	/// Continue a wall end at both ends.
	TwoTangents,
}

/// Where the current gesture is. Positions are graph metres unless noted.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub enum ArcDraft {
	#[default]
	Idle,
	/// A free arc's button is down. It is a click until the pointer passes the drag threshold.
	Pressed {
		start: Vec2,
		/// Viewport pixels, to measure the drag threshold.
		press_position: Vec2,
		dragged: bool,
	},
	/// The start is placed; the next click places the end of a free arc or confirms a
	/// tangent arc. `start_direction` is the direction of travel a tangent arc leaves in.
	AwaitingEnd {
		start: Vec2,
		start_direction: Option<Vec2>,
	},
	/// A free arc's endpoints are placed; the pointer picks the bend and a click confirms.
	AwaitingBend {
		start: Vec2,
		end: Vec2,
		bend: Vec2,
		/// A sweep typed into a field, which then wins over the pointer's bend.
		typed_sweep: Option<f32>,
	},
}

/// The direction a wall continuing the free wall end at `node` leaves in, or `None` when
/// `node` joins zero or several walls, which gives no single wall to continue.
pub fn free_end_continuation(graph: &WallGraph, node: WallNodeId) -> Option<Vec2> {
	let [wall] = graph.node_walls(node)?[..] else {
		return None;
	};
	let outgoing = if wall.origin() == node {
		graph.wall_tangent(wall, 0.0)?
	} else {
		-graph.wall_tangent(wall, 1.0)?
	};
	// A smooth join leaves in the opposite direction to the wall it continues.
	Some(-outgoing)
}

/// How an advancing draft changes the edit notice.
#[derive(Debug, PartialEq)]
pub enum NoticeChange {
	Keep,
	Clear,
	Set(String),
}

/// The result of one frame of the arc gesture.
#[derive(Debug, PartialEq)]
pub struct ArcStep {
	pub draft: ArcDraft,
	/// The gesture asks to commit the preview.
	pub confirmed: bool,
	pub notice: NoticeChange,
}

/// The next draft state, whether this click asks to commit the preview, and what the notice
/// should become.
pub fn advance_arc_draft(
	settings: &InteractionSettings,
	draft: ArcDraft,
	mode: ArcMode,
	actions: &ActionState,
	hit: Option<PointerOnGround>,
	target: Option<SnapTarget>,
	graph: &WallGraph,
) -> ArcStep {
	let pressed = actions.just_pressed(Action::ToolPress) && hit.is_some();
	let mut notice = NoticeChange::Keep;
	let (draft, confirmed) = advance(
		settings,
		draft,
		mode,
		actions,
		hit,
		target,
		pressed,
		graph,
		&mut notice,
	);
	ArcStep {
		draft,
		confirmed,
		notice,
	}
}

#[allow(clippy::too_many_arguments)]
fn advance(
	settings: &InteractionSettings,
	draft: ArcDraft,
	mode: ArcMode,
	actions: &ActionState,
	hit: Option<PointerOnGround>,
	target: Option<SnapTarget>,
	pressed: bool,
	graph: &WallGraph,
	notice: &mut NoticeChange,
) -> (ArcDraft, bool) {
	match (draft, mode) {
		(ArcDraft::Idle, ArcMode::Free) => {
			let started = actions
				.just_pressed(Action::ToolPress)
				.then(|| Some((hit?, target?)))
				.flatten();
			match started {
				Some((hit, target)) => (
					ArcDraft::Pressed {
						start: target.position,
						press_position: hit.viewport_position,
						dragged: false,
					},
					false,
				),
				None => (draft, false),
			}
		}
		(ArcDraft::Idle, _) => {
			let (true, Some(target)) = (pressed, target) else {
				return (draft, false);
			};
			let direction = match target.source {
				SnapSource::Junction(node) => free_end_continuation(graph, node),
				_ => None,
			};
			match direction {
				Some(direction) => {
					*notice = NoticeChange::Clear;
					(
						ArcDraft::AwaitingEnd {
							start: target.position,
							start_direction: Some(direction),
						},
						false,
					)
				}
				None => {
					*notice =
						NoticeChange::Set("Start on a free wall end to continue it".to_string());
					(draft, false)
				}
			}
		}
		(
			ArcDraft::Pressed {
				start,
				press_position,
				dragged,
			},
			_,
		) => {
			let dragged = dragged
				|| hit.is_some_and(|hit| {
					hit.viewport_position.distance(press_position)
						> settings.drag_threshold_pixels()
				});
			// The release is handled even when the pointer is over the panel, so a drag
			// that ends there still finishes.
			if !actions.just_released(Action::ToolPress) {
				return (
					ArcDraft::Pressed {
						start,
						press_position,
						dragged,
					},
					false,
				);
			}
			match (dragged, target) {
				(true, Some(target)) => (bend_stage(start, target.position), false),
				_ => (
					ArcDraft::AwaitingEnd {
						start,
						start_direction: None,
					},
					false,
				),
			}
		}
		(ArcDraft::AwaitingEnd { start, .. }, ArcMode::Free) => match (pressed, target) {
			(true, Some(target)) => (bend_stage(start, target.position), false),
			_ => (draft, false),
		},
		(ArcDraft::AwaitingEnd { .. }, _) => (draft, pressed),
		(
			ArcDraft::AwaitingBend {
				start,
				end,
				bend,
				typed_sweep,
			},
			_,
		) => {
			// A typed sweep ignores the pointer, so moving it must not refresh the draft.
			let bend = match (typed_sweep, target) {
				(None, Some(target)) => target.position,
				_ => bend,
			};
			(
				ArcDraft::AwaitingBend {
					start,
					end,
					bend,
					typed_sweep,
				},
				pressed,
			)
		}
	}
}

/// Endpoints chosen; until the pointer moves the bend sits on the chord's midpoint.
fn bend_stage(start: Vec2, end: Vec2) -> ArcDraft {
	ArcDraft::AwaitingBend {
		start,
		end,
		bend: (start + end) / 2.0,
		typed_sweep: None,
	}
}

/// The arc the draft would draw, or `None` while it has too little to propose.
pub fn arc_proposal(
	draft: ArcDraft,
	mode: ArcMode,
	target: Option<SnapTarget>,
	graph: &WallGraph,
	dimensions: WallDimensions,
) -> Option<ArcProposal> {
	match draft {
		ArcDraft::AwaitingBend {
			start,
			end,
			bend,
			typed_sweep,
		} => Some(ArcProposal {
			start,
			end,
			definition: typed_sweep.map_or(ArcDefinition::Bend(bend), ArcDefinition::Sweep),
			dimensions,
		}),
		ArcDraft::AwaitingEnd {
			start,
			start_direction: Some(start_direction),
		} => {
			let target = target?;
			let definition = match mode {
				ArcMode::TwoTangents => ArcDefinition::TwoTangents {
					start_direction,
					end_direction: match target.source {
						SnapSource::Junction(node) => {
							free_end_continuation(graph, node).map(|direction| -direction)
						}
						_ => None,
					},
				},
				_ => ArcDefinition::OneTangent { start_direction },
			};
			Some(ArcProposal {
				start,
				end: target.position,
				definition,
				dimensions,
			})
		}
		_ => None,
	}
}

#[cfg(test)]
#[path = "../tests/gestures/arc_drawing.rs"]
mod tests;
