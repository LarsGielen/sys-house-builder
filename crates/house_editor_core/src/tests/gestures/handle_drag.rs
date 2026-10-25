use super::*;
use crate::interaction_settings::InteractionSettings;
use house_document::{EditCandidate, WallProposal, propose_wall};
use wall_graph::WallDimensions;

fn wall_graph_with_one_wall() -> (EditCandidate, Wall) {
	let candidate = propose_wall(
		&EditCandidate::default(),
		&WallProposal {
			start: Vec2::ZERO,
			end: Vec2::new(4.0, 0.0),
			dimensions: WallDimensions::default(),
		},
	)
	.unwrap();
	let wall = candidate.graph().walls().next().unwrap();
	(candidate, wall)
}

fn hit(pixels: Vec2) -> Option<PointerOnGround> {
	Some(PointerOnGround {
		viewport_position: pixels,
		graph_position: pixels / 100.0,
	})
}

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

#[test]
fn pressing_on_a_hovered_handle_starts_a_drag_that_is_still_a_click() {
	let (_, wall) = wall_graph_with_one_wall();
	let handle = WallHandle::Endpoint(wall.origin());
	let step = advance_handle_drag(
		&InteractionSettings::default(),
		HandleDrag::Idle,
		&pressed(),
		hit(Vec2::new(100.0, 0.0)),
		Some(wall),
		Some(handle),
		None,
	);
	assert!(matches!(
		step.drag,
		HandleDrag::Pressed { handle: pressed, dragged: false, .. } if pressed == handle
	));
	assert_eq!(step.active, None);
}

#[test]
fn pressing_on_the_selected_wall_without_a_handle_grabs_its_body() {
	let (_, wall) = wall_graph_with_one_wall();
	let step = advance_handle_drag(
		&InteractionSettings::default(),
		HandleDrag::Idle,
		&pressed(),
		hit(Vec2::ZERO),
		Some(wall),
		None,
		Some(wall),
	);
	assert!(matches!(
		step.drag,
		HandleDrag::Pressed { handle: WallHandle::Body(grabbed), .. } if grabbed == wall
	));
}

#[test]
fn a_press_with_no_selection_or_no_ground_hit_does_nothing() {
	let (_, wall) = wall_graph_with_one_wall();
	for (pointer, selected) in [(None, Some(wall)), (hit(Vec2::ZERO), None)] {
		let step = advance_handle_drag(
			&InteractionSettings::default(),
			HandleDrag::Idle,
			&pressed(),
			pointer,
			selected,
			Some(WallHandle::Bend(wall)),
			Some(wall),
		);
		assert_eq!(step.drag, HandleDrag::Idle);
	}
}

#[test]
fn releasing_before_the_threshold_ends_the_gesture_without_an_edit() {
	let (_, wall) = wall_graph_with_one_wall();
	let drag = HandleDrag::Pressed {
		handle: WallHandle::Body(wall),
		wall,
		grab: Vec2::ZERO,
		press_position: Vec2::ZERO,
		dragged: false,
	};
	let step = advance_handle_drag(
		&InteractionSettings::default(),
		drag,
		&released(),
		hit(Vec2::new(2.0, 0.0)),
		None,
		None,
		None,
	);
	assert_eq!(step.drag, HandleDrag::Idle);
	assert_eq!(step.active, None);
}

#[test]
fn a_release_off_the_ground_still_ends_a_drag_that_passed_the_threshold() {
	let (_, wall) = wall_graph_with_one_wall();
	let drag = HandleDrag::Pressed {
		handle: WallHandle::Body(wall),
		wall,
		grab: Vec2::ZERO,
		press_position: Vec2::ZERO,
		dragged: true,
	};
	let step = advance_handle_drag(
		&InteractionSettings::default(),
		drag,
		&released(),
		None,
		None,
		None,
		None,
	);
	assert!(step.released);
	assert!(step.active.is_some());
}

#[test]
fn a_body_drag_moves_both_ends_in_grid_steps_unless_snapping_is_off() {
	let (candidate, wall) = wall_graph_with_one_wall();
	let active = ActiveDrag {
		handle: WallHandle::Body(wall),
		wall,
		grab: Vec2::ZERO,
	};
	let pointer = |snapping_disabled| DragPointer {
		hit: PointerOnGround {
			viewport_position: Vec2::ZERO,
			graph_position: Vec2::new(0.3, 0.0),
		},
		snap_target: None,
		snapping_disabled,
		grid_spacing: 1.0,
	};
	let moved = |snapping_disabled| match drag_proposal(
		active,
		&pointer(snapping_disabled),
		candidate.graph(),
	) {
		Some(Proposal::Move(edit)) => edit.first.1.x,
		other => panic!("expected a move, got {other:?}"),
	};
	assert_eq!(moved(false), 0.0);
	assert!((moved(true) - 0.3).abs() < 1e-6);
}

#[test]
fn a_ray_through_a_handle_finds_it_and_a_miss_finds_nothing() {
	let (candidate, wall) = wall_graph_with_one_wall();
	let positions = handle_positions(candidate.graph(), wall).unwrap();
	let above = |at: Vec3| at + Vec3::Z * 5.0;
	assert_eq!(
		handle_under_ray(candidate.graph(), wall, above(positions.bend), -Vec3::Z),
		Some(WallHandle::Bend(wall))
	);
	assert_eq!(
		handle_under_ray(
			candidate.graph(),
			wall,
			above(positions.bend + Vec3::Y * 3.0),
			-Vec3::Z
		),
		None
	);
}
