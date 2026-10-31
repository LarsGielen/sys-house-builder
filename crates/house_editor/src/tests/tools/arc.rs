use super::*;
use crate::document::edit::{EditError, PreviewState};
use crate::tests::support::{house_with_wall, press_key, pressed_mouse};
use crate::viewport::snapping::SnapSource;
use crate::{
	editor_schedule::EditorSchedulePlugin, tools::active::ActiveTool,
	viewport::ground_pointer::PointerOnGround, viewport::snapping::SnapTarget,
};
use wall_graph::{ArcError, WallNodeId};

fn arc_app(house: House) -> App {
	// The tool draws guides, which needs the gizmo resources.
	let mut app = crate::tests::support::tool_test_app();
	app.add_plugins((EditorSchedulePlugin, WallToolArcPlugin))
		.insert_resource(ActiveTool(Tool::DrawArc))
		.insert_resource(house)
		.init_resource::<GroundPointer>()
		.init_resource::<SnappedPointer>()
		.init_resource::<ButtonInput<MouseButton>>()
		.init_resource::<ButtonInput<KeyCode>>();
	app
}

fn point_at(app: &mut App, position: Vec2, source: SnapSource) {
	app.insert_resource(GroundPointer {
		hit: Some(PointerOnGround {
			viewport_position: position * 100.0,
			graph_position: position,
		}),
		..Default::default()
	});
	app.insert_resource(SnappedPointer {
		target: Some(SnapTarget { position, source }),
	});
}

/// One frame with the pointer at `position`, optionally clicking.
fn step(app: &mut App, position: Vec2, source: SnapSource, click: bool) {
	point_at(app, position, source);
	app.insert_resource(if click {
		pressed_mouse()
	} else {
		ButtonInput::<MouseButton>::default()
	});
	app.insert_resource(ButtonInput::<KeyCode>::default());
	app.update();
}

fn click(app: &mut App, position: Vec2) {
	step(app, position, SnapSource::Free, true);
}

fn hover(app: &mut App, position: Vec2) {
	step(app, position, SnapSource::Free, false);
}

/// A click-click pair for a free arc's endpoints: press and release without moving.
fn place_endpoint(app: &mut App, position: Vec2) {
	click(app, position);
	let mut mouse = pressed_mouse();
	mouse.clear();
	mouse.release(MouseButton::Left);
	app.insert_resource(mouse);
	app.update();
}

fn draft(app: &App) -> ArcDraft {
	app.world().resource::<ArcDraftState>().0
}

fn undo_steps(app: &mut App) -> usize {
	let mut house = app.world_mut().resource_mut::<House>();
	std::iter::from_fn(|| house.undo().then_some(())).count()
}

fn previewed_sweep(app: &App) -> f32 {
	let Some(PreviewState {
		proposal: Proposal::Arc(arc),
		..
	}) = app.world().resource::<EditPreview>().state
	else {
		panic!("expected an arc preview");
	};
	arc.sweep().unwrap()
}

#[test]
fn a_free_arc_takes_two_endpoints_then_a_bend_and_commits_once() {
	let mut app = arc_app(House::default());
	place_endpoint(&mut app, Vec2::ZERO);
	assert!(matches!(draft(&app), ArcDraft::AwaitingEnd { .. }));
	click(&mut app, Vec2::new(4.0, 0.0));
	assert!(matches!(draft(&app), ArcDraft::AwaitingBend { .. }));

	hover(&mut app, Vec2::new(2.0, 1.0));
	assert!(
		app.world()
			.resource::<EditPreview>()
			.valid_candidate()
			.is_some()
	);
	click(&mut app, Vec2::new(2.0, 1.5));

	let house = app.world().resource::<House>();
	assert_eq!(house.graph().walls().count(), 1);
	assert_eq!(draft(&app), ArcDraft::Idle);
	assert!(app.world().resource::<EditPreview>().state.is_none());
	assert_eq!(undo_steps(&mut app), 1);
}

#[test]
fn moving_the_bend_to_the_other_side_flips_the_arc() {
	let mut app = arc_app(House::default());
	place_endpoint(&mut app, Vec2::ZERO);
	click(&mut app, Vec2::new(4.0, 0.0));
	hover(&mut app, Vec2::new(2.0, 1.0));
	let above = previewed_sweep(&app);
	hover(&mut app, Vec2::new(2.0, -1.0));
	let below = previewed_sweep(&app);
	assert_eq!(above.signum(), -below.signum());
	hover(&mut app, Vec2::new(2.0, 2.0));
	// A higher bend is a deeper arc.
	assert!(previewed_sweep(&app).abs() > above.abs());
}

#[test]
fn a_bend_on_the_chord_previews_an_error_and_cannot_commit() {
	let mut app = arc_app(House::default());
	place_endpoint(&mut app, Vec2::ZERO);
	click(&mut app, Vec2::new(4.0, 0.0));
	click(&mut app, Vec2::new(2.0, 0.0));

	let preview = app.world().resource::<EditPreview>();
	assert!(matches!(
		&preview.state,
		Some(PreviewState {
			outcome: Err(EditError::Arc(ArcError::BendOnChord)),
			..
		})
	));
	assert_eq!(app.world().resource::<House>().revision(), 0);
	assert!(matches!(draft(&app), ArcDraft::AwaitingBend { .. }));
}

#[test]
fn escape_cancels_at_the_endpoint_stage_and_at_the_bend_stage() {
	for stage in [1, 2] {
		let mut app = arc_app(House::default());
		place_endpoint(&mut app, Vec2::ZERO);
		if stage == 2 {
			click(&mut app, Vec2::new(4.0, 0.0));
			hover(&mut app, Vec2::new(2.0, 1.0));
		}
		press_key(&mut app, KeyCode::Escape);
		app.update();
		assert_eq!(draft(&app), ArcDraft::Idle, "stage {stage}");
		assert!(app.world().resource::<EditPreview>().state.is_none());
		assert_eq!(app.world().resource::<House>().revision(), 0);
	}
}

fn node_at(house: &House, position: Vec2) -> WallNodeId {
	house
		.graph()
		.nodes()
		.find(|(_, node)| node.distance(position) < 1e-4)
		.unwrap()
		.0
}

fn app_with_free_end() -> App {
	let (house, _) = house_with_wall(Vec2::ZERO, Vec2::new(2.0, 0.0));
	let mut app = arc_app(house);
	app.insert_resource(ArcModeState(ArcMode::OneTangent));
	app.update();
	app
}

#[test]
fn a_tangent_arc_starts_on_a_free_wall_end_and_meets_it_smoothly() {
	let mut app = app_with_free_end();
	let end_node = node_at(app.world().resource::<House>(), Vec2::new(2.0, 0.0));
	step(
		&mut app,
		Vec2::new(2.0, 0.0),
		SnapSource::Junction(end_node),
		true,
	);
	assert!(matches!(
		draft(&app),
		ArcDraft::AwaitingEnd {
			start_direction: Some(_),
			..
		}
	));

	hover(&mut app, Vec2::new(4.0, 2.0));
	assert!(previewed_sweep(&app) > 0.0);
	click(&mut app, Vec2::new(4.0, 2.0));

	let house = app.world().resource::<House>();
	assert_eq!(house.graph().walls().count(), 2);
	assert_eq!(house.graph().smooth_joins().len(), 1);
	// The arc is one step on top of the wall the test started with.
	assert_eq!(undo_steps(&mut app), 2);
}

#[test]
fn a_tangent_arc_cannot_start_away_from_a_free_wall_end() {
	let mut app = app_with_free_end();
	click(&mut app, Vec2::new(5.0, 5.0));
	assert_eq!(draft(&app), ArcDraft::Idle);
	assert!(app.world().resource::<EditNotice>().message.is_some());

	// A junction of several walls gives no single wall to continue.
	let mut house = House::default();
	for end in [Vec2::new(2.0, 0.0), Vec2::new(0.0, 2.0)] {
		let candidate = crate::document::edit::propose_wall(
			house.committed(),
			&crate::document::edit::WallProposal {
				start: Vec2::ZERO,
				end,
				dimensions: Default::default(),
			},
		)
		.unwrap();
		house.commit(candidate);
	}
	let corner = node_at(&house, Vec2::ZERO);
	let mut app = arc_app(house);
	app.insert_resource(ArcModeState(ArcMode::OneTangent));
	app.update();
	step(&mut app, Vec2::ZERO, SnapSource::Junction(corner), true);
	assert_eq!(draft(&app), ArcDraft::Idle);
}

#[test]
fn a_tangent_arc_behind_the_wall_explains_why_it_cannot_commit() {
	let mut app = app_with_free_end();
	let end_node = node_at(app.world().resource::<House>(), Vec2::new(2.0, 0.0));
	step(
		&mut app,
		Vec2::new(2.0, 0.0),
		SnapSource::Junction(end_node),
		true,
	);
	click(&mut app, Vec2::new(-4.0, 0.0));

	assert!(matches!(
		&app.world().resource::<EditPreview>().state,
		Some(PreviewState {
			outcome: Err(_),
			..
		})
	));
	assert_eq!(app.world().resource::<House>().graph().walls().count(), 1);
	assert_eq!(app.world().resource::<House>().revision(), 1);
}

#[test]
fn two_connection_mode_needs_a_second_free_wall_end_and_never_falls_back_to_a_free_arc() {
	// Two free wall ends facing each other with an offset: an S-bend joins them.
	let mut house = House::default();
	for (start, end) in [
		(Vec2::ZERO, Vec2::new(2.0, 0.0)),
		(Vec2::new(8.0, 2.0), Vec2::new(10.0, 2.0)),
	] {
		let candidate = crate::document::edit::propose_wall(
			house.committed(),
			&crate::document::edit::WallProposal {
				start,
				end,
				dimensions: Default::default(),
			},
		)
		.unwrap();
		house.commit(candidate);
	}
	let first = node_at(&house, Vec2::new(2.0, 0.0));
	let second = node_at(&house, Vec2::new(8.0, 2.0));
	let mut app = arc_app(house);
	app.insert_resource(ArcModeState(ArcMode::TwoTangents));
	app.update();
	step(
		&mut app,
		Vec2::new(2.0, 0.0),
		SnapSource::Junction(first),
		true,
	);

	// Not on a wall end: invalid, with the reason.
	hover(&mut app, Vec2::new(8.0, 2.0));
	assert!(matches!(
		&app.world().resource::<EditPreview>().state,
		Some(PreviewState {
			outcome: Err(EditError::Arc(ArcError::MissingConnection)),
			..
		})
	));
	click(&mut app, Vec2::new(8.0, 2.0));
	assert_eq!(app.world().resource::<House>().revision(), 2);

	// On the second wall end: the tangents are inconsistent for this offset only if the
	// directions disagree, which a mirrored S-bend avoids.
	step(
		&mut app,
		Vec2::new(8.0, 2.0),
		SnapSource::Junction(second),
		false,
	);
	let outcome = &app
		.world()
		.resource::<EditPreview>()
		.state
		.as_ref()
		.unwrap()
		.outcome;
	assert!(
		!matches!(outcome, Err(EditError::Arc(ArcError::MissingConnection))),
		"{:?}",
		outcome.as_ref().err()
	);
}

#[test]
fn switching_modes_abandons_the_arc_in_progress() {
	let mut app = arc_app(House::default());
	place_endpoint(&mut app, Vec2::ZERO);
	click(&mut app, Vec2::new(4.0, 0.0));
	hover(&mut app, Vec2::new(2.0, 1.0));
	assert!(app.world().resource::<EditPreview>().state.is_some());

	app.insert_resource(ArcModeState(ArcMode::OneTangent));
	app.update();
	assert_eq!(draft(&app), ArcDraft::Idle);
	assert!(app.world().resource::<EditPreview>().state.is_none());
}

#[test]
fn leaving_the_tool_and_undoing_discard_the_draft() {
	let mut app = arc_app(House::default());
	place_endpoint(&mut app, Vec2::ZERO);
	app.insert_resource(ActiveTool(Tool::Select));
	app.update();
	assert_eq!(draft(&app), ArcDraft::Idle);

	let (house, _) = house_with_wall(Vec2::ZERO, Vec2::new(2.0, 0.0));
	let mut app = arc_app(house);
	app.add_plugins(crate::document::history::HistoryPlugin);
	place_endpoint(&mut app, Vec2::new(5.0, 5.0));
	app.world_mut()
		.write_message(crate::actions::EditorAction::from(
			crate::document::history::HistoryRequest::Undo,
		));
	app.update();
	assert_eq!(draft(&app), ArcDraft::Idle);
}
