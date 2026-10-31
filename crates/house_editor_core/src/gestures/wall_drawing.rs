use glam::Vec2;
use house_document::WallProposal;
use wall_graph::WallDimensions;

use crate::interaction_settings::InteractionSettings;
use crate::{
	action::{Action, ActionState},
	pointing::pointer::PointerOnGround,
	pointing::snapping::SnapTarget,
};

/// Where the current gesture is. Positions are graph metres unless noted.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub enum WallDraft {
	#[default]
	Idle,
	/// The button is down. It is a click until the pointer moves past the drag threshold.
	Pressed {
		start: Vec2,
		/// Viewport pixels, to measure the drag threshold.
		press_position: Vec2,
		dragged: bool,
	},
	/// A click placed the start; the next click confirms the end.
	AwaitingEnd { start: Vec2 },
}

impl WallDraft {
	pub fn start(self) -> Option<Vec2> {
		match self {
			WallDraft::Idle => None,
			WallDraft::Pressed { start, .. } | WallDraft::AwaitingEnd { start } => Some(start),
		}
	}
}

/// Whether a draft step asks to commit the preview.
#[derive(Debug, PartialEq)]
pub enum DraftAction {
	Continue,
	Confirm,
}

/// The next draft after this frame's actions and pointer, and whether it asks to commit.
pub fn advance_draft(
	settings: &InteractionSettings,
	draft: WallDraft,
	actions: &ActionState,
	hit: Option<PointerOnGround>,
	target: Option<SnapTarget>,
) -> (WallDraft, DraftAction) {
	match draft {
		WallDraft::Idle => {
			let started = actions
				.just_pressed(Action::ToolPress)
				.then(|| Some((hit?, target?)))
				.flatten();
			match started {
				Some((hit, target)) => (
					WallDraft::Pressed {
						start: target.position,
						press_position: hit.viewport_position,
						dragged: false,
					},
					DraftAction::Continue,
				),
				None => (draft, DraftAction::Continue),
			}
		}
		WallDraft::Pressed {
			start,
			press_position,
			dragged,
		} => {
			let dragged = dragged
				|| hit.is_some_and(|hit| {
					hit.viewport_position.distance(press_position)
						> settings.drag_threshold_pixels()
				});
			// The release is handled even when the pointer is over the panel, so a drag
			// that ends there still finishes.
			if !actions.just_released(Action::ToolPress) {
				return (
					WallDraft::Pressed {
						start,
						press_position,
						dragged,
					},
					DraftAction::Continue,
				);
			}
			if dragged {
				(WallDraft::AwaitingEnd { start }, DraftAction::Confirm)
			} else {
				(WallDraft::AwaitingEnd { start }, DraftAction::Continue)
			}
		}
		WallDraft::AwaitingEnd { .. } => {
			let confirmed = actions.just_pressed(Action::ToolPress) && hit.is_some();
			let action = if confirmed {
				DraftAction::Confirm
			} else {
				DraftAction::Continue
			};
			(draft, action)
		}
	}
}

/// What the wall preview should do this frame.
#[derive(Debug, PartialEq)]
pub enum WallPreview {
	/// No wall is being drawn, or it is still a click.
	Clear,
	/// Off the ground with nothing to refresh.
	Keep,
	Propose(WallProposal),
}

/// The wall to preview for `draft`. The last endpoint is kept while the pointer is off the
/// ground, but refreshed with the current dimensions.
pub fn wall_preview(
	draft: WallDraft,
	target: Option<SnapTarget>,
	previous: Option<WallProposal>,
	dimensions: WallDimensions,
) -> WallPreview {
	let Some(start) = draft.start() else {
		return WallPreview::Clear;
	};
	// Until the pointer leaves the click threshold there is no end worth previewing.
	if matches!(draft, WallDraft::Pressed { dragged: false, .. }) {
		return WallPreview::Clear;
	}
	match (target, previous) {
		(Some(target), _) => WallPreview::Propose(WallProposal {
			start,
			end: target.position,
			dimensions,
		}),
		(None, Some(previous)) => WallPreview::Propose(WallProposal {
			start,
			dimensions,
			..previous
		}),
		(None, None) => WallPreview::Keep,
	}
}

#[cfg(test)]
#[path = "../tests/gestures/wall_drawing.rs"]
mod tests;
