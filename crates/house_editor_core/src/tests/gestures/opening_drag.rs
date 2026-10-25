use super::*;
use crate::interaction_settings::InteractionSettings;
use house_document::{
	EditCandidate, OpeningCategory, OpeningProposal, WallProposal, propose_opening, propose_wall,
};
use wall_graph::WallDimensions;

fn document_with_door() -> (EditCandidate, OpeningId) {
	let wall_candidate = propose_wall(
		&EditCandidate::default(),
		&WallProposal {
			start: Vec2::ZERO,
			end: Vec2::new(6.0, 0.0),
			dimensions: WallDimensions::default(),
		},
	)
	.unwrap();
	let wall = wall_candidate.graph().walls().next().unwrap();
	let candidate = propose_opening(
		&wall_candidate,
		&OpeningProposal {
			wall,
			spec: OpeningSpec {
				center_distance: 3.0,
				width: 1.0,
				bottom: 0.0,
				height: 2.1,
			},
			category: OpeningCategory::Door,
		},
	)
	.unwrap();
	let opening = candidate.graph().openings().next().unwrap().0;
	(candidate, opening)
}

fn hit_at(graph_position: Vec2) -> Option<PointerOnGround> {
	Some(PointerOnGround {
		viewport_position: graph_position * 100.0,
		graph_position,
	})
}

fn pressed() -> ActionState {
	let mut state = ActionState::default();
	state.press(Action::ToolPress);
	state
}

#[test]
fn only_the_selected_opening_under_the_pointer_can_be_grabbed() {
	let (candidate, opening) = document_with_door();
	let graph = candidate.graph();
	let start = |hovered, selected| {
		advance_opening_drag(
			&InteractionSettings::default(),
			OpeningDrag::Idle,
			&pressed(),
			hit_at(Vec2::new(3.0, 0.0)),
			hovered,
			selected,
			graph,
		)
		.0
	};
	assert!(matches!(
		start(Some(opening), Some(opening)),
		OpeningDrag::Pressed { dragged: false, .. }
	));
	assert_eq!(start(Some(opening), None), OpeningDrag::Idle);
	assert_eq!(start(None, Some(opening)), OpeningDrag::Idle);
}

#[test]
fn dragging_past_the_threshold_marks_the_drag_and_release_is_reported_off_the_ground() {
	let (candidate, opening) = document_with_door();
	let original = candidate.graph().opening(opening).unwrap().spec;
	let drag = OpeningDrag::Pressed {
		opening,
		original,
		grab_distance: 3.0,
		press_position: Vec2::ZERO,
		dragged: false,
	};
	let (moved, released) = advance_opening_drag(
		&InteractionSettings::default(),
		drag,
		&ActionState::default(),
		hit_at(Vec2::new(3.5, 0.0)),
		None,
		None,
		candidate.graph(),
	);
	assert!(matches!(moved, OpeningDrag::Pressed { dragged: true, .. }));
	assert!(!released);

	let mut release = ActionState::default();
	release.release(Action::ToolPress);
	let (_, released) = advance_opening_drag(
		&InteractionSettings::default(),
		moved,
		&release,
		None,
		None,
		None,
		candidate.graph(),
	);
	assert!(released);
}

#[test]
fn an_unsnapped_drag_moves_the_opening_by_the_pointer_travel() {
	let (candidate, opening) = document_with_door();
	let original = candidate.graph().opening(opening).unwrap().spec;
	let proposal = opening_drag_proposal(
		candidate.graph(),
		opening,
		original,
		3.0,
		hit_at(Vec2::new(4.2, 0.0)).unwrap(),
		None,
		|_| unreachable!("bounds are only needed when snapping"),
	)
	.unwrap();
	assert!((proposal.spec.center_distance - 4.2).abs() < 1e-6);
}
