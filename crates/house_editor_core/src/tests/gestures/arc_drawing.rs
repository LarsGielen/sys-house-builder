use super::*;
use crate::interaction_settings::InteractionSettings;
use crate::pointing::snapping::SnapSource;
use house_document::EditCandidate;

fn pressed() -> ActionState {
	let mut state = ActionState::default();
	state.press(Action::ToolPress);
	state
}

fn pointer_at(pixels: Vec2) -> Option<PointerOnGround> {
	Some(PointerOnGround {
		viewport_position: pixels,
		graph_position: pixels / 100.0,
	})
}

fn free_target(position: Vec2) -> Option<SnapTarget> {
	Some(SnapTarget {
		position,
		source: SnapSource::Free,
	})
}

fn advance(draft: ArcDraft, mode: ArcMode, hit: bool, target: Option<SnapTarget>) -> ArcStep {
	advance_arc_draft(
		&InteractionSettings::default(),
		draft,
		mode,
		&pressed(),
		hit.then(|| pointer_at(Vec2::ZERO)).flatten(),
		target,
		EditCandidate::default().graph(),
	)
}

#[test]
fn a_free_arc_press_waits_for_release_before_placing_its_start() {
	let step = advance(ArcDraft::Idle, ArcMode::Free, true, free_target(Vec2::X));
	assert!(matches!(step.draft, ArcDraft::Pressed { start, .. } if start == Vec2::X));
	assert!(!step.confirmed);
}

#[test]
fn a_press_off_the_ground_changes_nothing() {
	let step = advance(ArcDraft::Idle, ArcMode::Free, false, free_target(Vec2::X));
	assert_eq!(step.draft, ArcDraft::Idle);
}

#[test]
fn a_tangent_arc_cannot_start_away_from_a_free_wall_end() {
	let step = advance(
		ArcDraft::Idle,
		ArcMode::OneTangent,
		true,
		free_target(Vec2::X),
	);
	assert_eq!(step.draft, ArcDraft::Idle);
	assert!(matches!(step.notice, NoticeChange::Set(_)));
}

#[test]
fn at_the_bend_stage_a_click_confirms_and_the_pointer_moves_the_bend() {
	let draft = ArcDraft::AwaitingBend {
		start: Vec2::ZERO,
		end: Vec2::new(4.0, 0.0),
		bend: Vec2::new(2.0, 0.0),
		typed_sweep: None,
	};
	let step = advance(draft, ArcMode::Free, true, free_target(Vec2::new(2.0, 1.0)));
	assert!(step.confirmed);
	assert!(matches!(
		step.draft,
		ArcDraft::AwaitingBend { bend, .. } if bend == Vec2::new(2.0, 1.0)
	));
}

#[test]
fn a_typed_sweep_ignores_the_pointer() {
	let draft = ArcDraft::AwaitingBend {
		start: Vec2::ZERO,
		end: Vec2::new(4.0, 0.0),
		bend: Vec2::new(2.0, 0.0),
		typed_sweep: Some(1.0),
	};
	let step = advance(draft, ArcMode::Free, true, free_target(Vec2::new(2.0, 1.0)));
	assert_eq!(step.draft, draft);
}
