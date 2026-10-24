use super::*;
use crate::tests::support::{house_with_wall, pressed_mouse, released_mouse, tool_test_app};
use crate::{
	edit::{EditError, PreviewState, WallProposal, propose_wall},
	editor_schedule::EditorSchedulePlugin,
	ground_pointer::PointerOnGround,
	snapping::{SnapSource, SnapTarget},
	wall_tool_active::ActiveTool,
	wall_tool_selection::{SelectionTarget, WallToolSelectionPlugin},
};

const WALL_HEIGHT_FOR_RAY: f32 = 10.0;

fn handles_app(house: House, wall: Wall) -> App {
	let mut app = tool_test_app();
	app.add_plugins((
		EditorSchedulePlugin,
		WallToolSelectionPlugin,
		WallToolHandlesPlugin,
	))
	.insert_resource(ActiveTool(Tool::Select))
	.insert_resource(house)
	.init_resource::<SnapSettings>()
	.init_resource::<GroundPointer>()
	.init_resource::<SnappedPointer>();
	// Inserting the house counts as a commit, which clears the selection; select afterwards.
	app.update();
	app.insert_resource(SelectedTarget(Some(SelectionTarget::Wall(wall))));
	app
}

fn wall_app() -> (App, Wall) {
	let (house, wall) = house_with_wall(Vec2::ZERO, Vec2::new(4.0, 0.0));
	(handles_app(house, wall), wall)
}

/// One frame with the pointer's ray straight down through graph position `at`.
fn frame(app: &mut App, at: Vec2, mouse: ButtonInput<MouseButton>) {
	app.insert_resource(GroundPointer {
		hit: Some(PointerOnGround {
			viewport_position: at * 100.0,
			graph_position: at,
		}),
		ray: Some(Ray3d::new(
			graph_to_bevy(at.extend(WALL_HEIGHT_FOR_RAY)),
			Dir3::NEG_Y,
		)),
	});
	app.insert_resource(SnappedPointer {
		target: Some(SnapTarget {
			position: at,
			source: SnapSource::Free,
		}),
	});
	app.insert_resource(mouse);
	app.update();
}

fn idle() -> ButtonInput<MouseButton> {
	ButtonInput::default()
}

fn snapping_off(app: &mut App) {
	let mut keyboard = ButtonInput::<KeyCode>::default();
	keyboard.press(KeyCode::ShiftLeft);
	app.insert_resource(keyboard);
}

fn undo_steps(app: &mut App) -> usize {
	let mut house = app.world_mut().resource_mut::<House>();
	std::iter::from_fn(|| house.undo().then_some(())).count()
}

#[test]
fn handles_under_the_pointer_are_found_on_the_selected_wall_only() {
	let (mut app, wall) = wall_app();
	frame(&mut app, Vec2::new(4.0, 0.0), idle());
	assert_eq!(
		app.world().resource::<HoveredHandle>().0,
		Some(WallHandle::Endpoint(wall.destination()))
	);
	frame(&mut app, Vec2::new(2.0, 0.0), idle());
	assert_eq!(
		app.world().resource::<HoveredHandle>().0,
		Some(WallHandle::Bend(wall))
	);
	frame(&mut app, Vec2::new(1.0, 0.0), idle());
	assert_eq!(app.world().resource::<HoveredHandle>().0, None);

	app.insert_resource(SelectedTarget(None));
	frame(&mut app, Vec2::new(4.0, 0.0), idle());
	assert_eq!(app.world().resource::<HoveredHandle>().0, None);
}

#[test]
fn dragging_an_endpoint_previews_then_commits_one_undo_step_and_keeps_the_selection() {
	let (mut app, wall) = wall_app();
	let setup_revision = app.world().resource::<House>().revision();
	frame(&mut app, Vec2::new(4.0, 0.0), pressed_mouse());
	assert!(matches!(
		*app.world().resource::<HandleDrag>(),
		HandleDrag::Pressed { dragged: false, .. }
	));
	// The press on the handle did not change the selection.
	assert_eq!(app.world().resource::<SelectedTarget>().wall(), Some(wall));

	snapping_off(&mut app);
	for y in [1.0, 2.0, 3.0] {
		frame(&mut app, Vec2::new(4.0, y), ButtonInput::default());
	}
	assert!(matches!(
		app.world().resource::<EditPreview>().state,
		Some(PreviewState {
			proposal: Proposal::Move(_),
			outcome: Ok(_),
			..
		})
	));
	assert_eq!(
		app.world().resource::<House>().revision(),
		setup_revision,
		"nothing commits before release"
	);

	frame(&mut app, Vec2::new(4.0, 3.0), released_mouse());
	let house = app.world().resource::<House>();
	assert_eq!(
		house.graph().node_position(wall.destination()),
		Some(Vec2::new(4.0, 3.0))
	);
	assert!(app.world().resource::<EditPreview>().state.is_none());
	// The wall was rebuilt, yet its selection follows it once the commit is seen.
	app.update();
	let house = app.world().resource::<House>();
	let selected = app.world().resource::<SelectedTarget>().wall();
	assert!(selected.is_some_and(|wall| house.graph().wall_length(wall).is_some()));
	// One step for the wall the test started with, one for the whole drag.
	assert_eq!(undo_steps(&mut app), 2);
}

#[test]
fn escape_during_a_drag_restores_the_original_view_without_a_history_entry() {
	let (mut app, wall) = wall_app();
	frame(&mut app, Vec2::new(4.0, 0.0), pressed_mouse());
	snapping_off(&mut app);
	frame(&mut app, Vec2::new(4.0, 3.0), ButtonInput::default());
	assert!(app.world().resource::<EditPreview>().state.is_some());

	let mut keyboard = ButtonInput::<KeyCode>::default();
	keyboard.press(KeyCode::Escape);
	app.insert_resource(keyboard);
	frame(&mut app, Vec2::new(4.0, 3.0), ButtonInput::default());

	assert_eq!(*app.world().resource::<HandleDrag>(), HandleDrag::Idle);
	assert!(app.world().resource::<EditPreview>().state.is_none());
	assert_eq!(
		app.world()
			.resource::<House>()
			.graph()
			.node_position(wall.destination()),
		Some(Vec2::new(4.0, 0.0))
	);
	assert_eq!(undo_steps(&mut app), 1);
}

#[test]
fn a_rejected_drag_leaves_no_history_entry_and_explains_itself_while_it_lasts() {
	let (mut house, wall) = house_with_wall(Vec2::ZERO, Vec2::new(4.0, 0.0));
	let candidate = propose_wall(
		house.committed(),
		&WallProposal {
			start: Vec2::new(0.0, 2.0),
			end: Vec2::new(4.0, 2.0),
			dimensions: Default::default(),
		},
	)
	.unwrap();
	house.commit(candidate);
	let wall = house
		.graph()
		.walls()
		.find(|candidate| candidate.origin() == wall.origin())
		.unwrap();
	let mut app = handles_app(house, wall);
	let revision = app.world().resource::<House>().revision();

	frame(&mut app, Vec2::new(4.0, 0.0), pressed_mouse());
	snapping_off(&mut app);
	frame(&mut app, Vec2::new(4.0, 4.0), ButtonInput::default());
	assert!(matches!(
		app.world().resource::<EditPreview>().state,
		Some(PreviewState {
			outcome: Err(EditError::Graph(wall_graph::WallError::CrossesWall)),
			..
		})
	));
	frame(&mut app, Vec2::new(4.0, 4.0), released_mouse());

	assert_eq!(app.world().resource::<House>().revision(), revision);
	assert!(app.world().resource::<EditPreview>().state.is_none());
	assert_eq!(undo_steps(&mut app), 2, "only the two setup walls");
}

#[test]
fn dragging_the_bend_handle_bends_the_wall_and_the_chord_straightens_it() {
	let (mut app, wall) = wall_app();
	frame(&mut app, Vec2::new(2.0, 0.0), pressed_mouse());
	snapping_off(&mut app);
	frame(&mut app, Vec2::new(2.0, 1.0), ButtonInput::default());
	let Some(PreviewState {
		proposal: Proposal::Bend(bend),
		outcome: Ok(_),
		..
	}) = app.world().resource::<EditPreview>().state
	else {
		panic!("expected a valid bend preview");
	};
	assert_eq!(bend.wall, wall);
	assert!(bend.sweep != 0.0);

	frame(&mut app, Vec2::new(2.0, 0.0), ButtonInput::default());
	let Some(PreviewState {
		proposal: Proposal::Bend(bend),
		..
	}) = app.world().resource::<EditPreview>().state
	else {
		panic!("expected a bend preview");
	};
	assert_eq!(bend.sweep, 0.0);

	frame(&mut app, Vec2::new(2.0, 1.0), ButtonInput::default());
	frame(&mut app, Vec2::new(2.0, 1.0), released_mouse());
	let house = app.world().resource::<House>();
	let walls: Vec<_> = house.graph().walls().collect();
	assert_eq!(walls.len(), 1);
	assert!(matches!(
		house.graph().wall_curve(walls[0]),
		Some(wall_graph::WallCurve::CircularArc { .. })
	));
}

#[test]
fn dragging_the_wall_body_moves_both_ends_together() {
	let (mut app, wall) = wall_app();
	frame(&mut app, Vec2::new(1.0, 0.0), pressed_mouse());
	assert!(matches!(
		*app.world().resource::<HandleDrag>(),
		HandleDrag::Pressed {
			handle: WallHandle::Body(_),
			..
		}
	));
	snapping_off(&mut app);
	frame(&mut app, Vec2::new(1.0, 2.0), ButtonInput::default());
	frame(&mut app, Vec2::new(1.0, 2.0), released_mouse());

	let graph = app.world().resource::<House>().graph();
	assert_eq!(
		graph.node_position(wall.origin()),
		Some(Vec2::new(0.0, 2.0))
	);
	assert_eq!(
		graph.node_position(wall.destination()),
		Some(Vec2::new(4.0, 2.0))
	);
}

#[test]
fn a_click_on_a_handle_without_dragging_changes_nothing() {
	let (mut app, wall) = wall_app();
	let revision = app.world().resource::<House>().revision();
	frame(&mut app, Vec2::new(4.0, 0.0), pressed_mouse());
	frame(&mut app, Vec2::new(4.0, 0.0), released_mouse());
	assert_eq!(app.world().resource::<House>().revision(), revision);
	assert_eq!(*app.world().resource::<HandleDrag>(), HandleDrag::Idle);
	assert_eq!(app.world().resource::<SelectedTarget>().wall(), Some(wall));
}

#[test]
fn switching_tool_or_undoing_abandons_the_drag() {
	let (mut app, _) = wall_app();
	frame(&mut app, Vec2::new(4.0, 0.0), pressed_mouse());
	snapping_off(&mut app);
	frame(&mut app, Vec2::new(4.0, 3.0), ButtonInput::default());
	app.insert_resource(ActiveTool(Tool::DrawWall));
	app.update();
	assert_eq!(*app.world().resource::<HandleDrag>(), HandleDrag::Idle);
	assert!(app.world().resource::<EditPreview>().state.is_none());

	let (mut app, _) = wall_app();
	app.add_plugins(crate::history::HistoryPlugin);
	frame(&mut app, Vec2::new(4.0, 0.0), pressed_mouse());
	app.world_mut()
		.write_message(crate::history::HistoryRequest::Undo);
	app.update();
	assert_eq!(*app.world().resource::<HandleDrag>(), HandleDrag::Idle);
}
