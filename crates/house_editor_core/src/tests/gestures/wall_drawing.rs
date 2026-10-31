use super::*;
use crate::interaction_settings::InteractionSettings;
use crate::pointing::snapping::SnapSource;

fn pressed() -> ActionState {
	let mut state = ActionState::default();
	state.press(Action::ToolPress);
	state
}

fn released() -> ActionState {
	let mut state = ActionState::default();
	state.release(Action::ToolPress);
	state
}

fn pointer_at(pixels: Vec2) -> Option<PointerOnGround> {
	Some(PointerOnGround {
		viewport_position: pixels,
		graph_position: pixels / 100.0,
	})
}

fn snapped_at(position: Vec2) -> Option<SnapTarget> {
	Some(SnapTarget {
		position,
		source: SnapSource::Free,
	})
}

#[test]
fn press_starts_a_draft_at_the_snapped_point() {
	let (draft, step) = advance_draft(
		&InteractionSettings::default(),
		WallDraft::Idle,
		&pressed(),
		pointer_at(Vec2::new(100.0, 100.0)),
		snapped_at(Vec2::new(1.0, 1.0)),
	);

	assert_eq!(step, DraftAction::Continue);
	assert_eq!(
		draft,
		WallDraft::Pressed {
			start: Vec2::new(1.0, 1.0),
			press_position: Vec2::new(100.0, 100.0),
			dragged: false,
		}
	);
}

#[test]
fn press_off_the_ground_does_not_start_a_draft() {
	let (draft, _) = advance_draft(
		&InteractionSettings::default(),
		WallDraft::Idle,
		&pressed(),
		None,
		None,
	);

	assert_eq!(draft, WallDraft::Idle);
}

#[test]
fn release_without_moving_waits_for_a_second_click() {
	let pressed_draft = WallDraft::Pressed {
		start: Vec2::ZERO,
		press_position: Vec2::new(100.0, 100.0),
		dragged: false,
	};
	let (draft, step) = advance_draft(
		&InteractionSettings::default(),
		pressed_draft,
		&released(),
		pointer_at(Vec2::new(102.0, 100.0)),
		snapped_at(Vec2::ZERO),
	);

	assert_eq!(draft, WallDraft::AwaitingEnd { start: Vec2::ZERO });
	assert_eq!(step, DraftAction::Continue);
}

#[test]
fn release_after_a_drag_confirms() {
	let pressed_draft = WallDraft::Pressed {
		start: Vec2::ZERO,
		press_position: Vec2::new(100.0, 100.0),
		dragged: false,
	};
	let (_, step) = advance_draft(
		&InteractionSettings::default(),
		pressed_draft,
		&released(),
		pointer_at(Vec2::new(200.0, 100.0)),
		snapped_at(Vec2::new(1.0, 0.0)),
	);

	assert_eq!(step, DraftAction::Confirm);
}

#[test]
fn second_click_confirms_only_with_a_ground_hit() {
	let awaiting = WallDraft::AwaitingEnd { start: Vec2::ZERO };
	let snapped = snapped_at(Vec2::new(1.0, 0.0));

	let (_, over_ui) = advance_draft(
		&InteractionSettings::default(),
		awaiting,
		&pressed(),
		None,
		snapped,
	);
	let (_, on_ground) = advance_draft(
		&InteractionSettings::default(),
		awaiting,
		&pressed(),
		pointer_at(Vec2::new(100.0, 0.0)),
		snapped,
	);

	assert_eq!(over_ui, DraftAction::Continue);
	assert_eq!(on_ground, DraftAction::Confirm);
}

#[test]
fn a_click_that_has_not_become_a_drag_previews_nothing() {
	let draft = WallDraft::Pressed {
		start: Vec2::ZERO,
		press_position: Vec2::ZERO,
		dragged: false,
	};
	assert_eq!(
		wall_preview(draft, snapped_at(Vec2::X), None, WallDimensions::default()),
		WallPreview::Clear
	);
}

#[test]
fn off_the_ground_the_last_end_is_kept_with_current_dimensions() {
	let draft = WallDraft::AwaitingEnd { start: Vec2::ZERO };
	let previous = WallProposal {
		start: Vec2::ZERO,
		end: Vec2::X,
		dimensions: WallDimensions::default(),
	};
	let thicker = WallDimensions {
		thickness: 0.4,
		..WallDimensions::default()
	};
	assert_eq!(
		wall_preview(draft, None, Some(previous), thicker),
		WallPreview::Propose(WallProposal {
			dimensions: thicker,
			..previous
		})
	);
	assert_eq!(wall_preview(draft, None, None, thicker), WallPreview::Keep);
}

#[test]
fn the_drag_threshold_comes_from_the_settings() {
	let draft = WallDraft::Pressed {
		start: Vec2::ZERO,
		press_position: Vec2::new(100.0, 100.0),
		dragged: false,
	};
	let travelled = pointer_at(Vec2::new(115.0, 100.0));
	let step = |threshold| {
		let settings = InteractionSettings::new(threshold, 12.0, 20.0).unwrap();
		advance_draft(
			&settings,
			draft,
			&released(),
			travelled,
			snapped_at(Vec2::X),
		)
		.1
	};
	assert_eq!(step(6.0), DraftAction::Confirm);
	assert_eq!(step(20.0), DraftAction::Continue);
}
