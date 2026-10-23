use super::*;
use crate::wall_tool_active::ActiveTool;
use crate::{
	ground_pointer::PointerOnGround,
	snapping::{SnapSource, SnapTarget},
};

fn pointer_at(pixels: Vec2) -> GroundPointer {
	GroundPointer {
		hit: Some(PointerOnGround {
			viewport_position: pixels,
			graph_position: pixels / 100.0,
		}),
		..Default::default()
	}
}

fn snapped_at(position: Vec2) -> SnappedPointer {
	SnappedPointer {
		target: Some(SnapTarget {
			position,
			source: SnapSource::Free,
		}),
	}
}

fn pressed() -> ButtonInput<MouseButton> {
	let mut mouse = ButtonInput::default();
	mouse.press(MouseButton::Left);
	mouse
}

fn released() -> ButtonInput<MouseButton> {
	let mut mouse = ButtonInput::default();
	mouse.press(MouseButton::Left);
	mouse.clear();
	mouse.release(MouseButton::Left);
	mouse
}

#[test]
fn press_starts_a_draft_at_the_snapped_point() {
	let (draft, step) = advance_draft(
		WallDraft::Idle,
		&pressed(),
		&pointer_at(Vec2::new(100.0, 100.0)),
		&snapped_at(Vec2::new(1.0, 1.0)),
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
		WallDraft::Idle,
		&pressed(),
		&GroundPointer::default(),
		&SnappedPointer::default(),
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
		pressed_draft,
		&released(),
		&pointer_at(Vec2::new(102.0, 100.0)),
		&snapped_at(Vec2::ZERO),
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
		pressed_draft,
		&released(),
		&pointer_at(Vec2::new(200.0, 100.0)),
		&snapped_at(Vec2::new(1.0, 0.0)),
	);

	assert_eq!(step, DraftAction::Confirm);
}

#[test]
fn second_click_confirms_only_with_a_ground_hit() {
	let awaiting = WallDraft::AwaitingEnd { start: Vec2::ZERO };
	let snapped = snapped_at(Vec2::new(1.0, 0.0));

	let (_, over_ui) = advance_draft(awaiting, &pressed(), &GroundPointer::default(), &snapped);
	let (_, on_ground) = advance_draft(
		awaiting,
		&pressed(),
		&pointer_at(Vec2::new(100.0, 0.0)),
		&snapped,
	);

	assert_eq!(over_ui, DraftAction::Continue);
	assert_eq!(on_ground, DraftAction::Confirm);
}

#[test]
fn zero_length_proposal_is_an_invalid_preview() {
	let mut app = drawing_app();
	app.insert_resource(WallDraft::AwaitingEnd { start: Vec2::ONE })
		.insert_resource(snapped_at(Vec2::ONE));
	app.update();
	assert!(matches!(
		app.world().resource::<EditPreview>().state,
		Some(PreviewState {
			outcome: Err(_),
			..
		})
	));
}

fn drawing_app() -> App {
	let mut app = App::new();
	app.add_plugins((
		crate::editor_schedule::EditorSchedulePlugin,
		WallToolDrawingPlugin,
	))
	.insert_resource(ActiveTool(Tool::DrawWall))
	.init_resource::<House>()
	.init_resource::<GroundPointer>()
	.init_resource::<SnappedPointer>()
	.init_resource::<ButtonInput<MouseButton>>()
	.init_resource::<ButtonInput<KeyCode>>();
	app
}

#[test]
fn invalid_drag_release_waits_for_a_new_endpoint() {
	let mut app = drawing_app();
	app.insert_resource(WallDraft::Pressed {
		start: Vec2::ZERO,
		press_position: Vec2::ZERO,
		dragged: true,
	})
	.insert_resource(released())
	.insert_resource(snapped_at(Vec2::ZERO));
	app.update();
	assert_eq!(
		*app.world().resource::<WallDraft>(),
		WallDraft::AwaitingEnd { start: Vec2::ZERO }
	);
	assert_eq!(app.world().resource::<House>().revision(), 0);
	app.insert_resource(pressed())
		.insert_resource(pointer_at(Vec2::new(100.0, 0.0)))
		.insert_resource(snapped_at(Vec2::X));
	app.update();
	assert_eq!(*app.world().resource::<WallDraft>(), WallDraft::Idle);
	assert_eq!(app.world().resource::<House>().graph().walls().count(), 1);
}

#[test]
fn unchanged_preview_does_not_trigger_change_detection() {
	#[derive(Resource, Default)]
	struct PreviewChanges(usize);
	fn count_changes(mut count: ResMut<PreviewChanges>) {
		count.0 += 1;
	}
	let mut app = drawing_app();
	app.init_resource::<PreviewChanges>()
		.insert_resource(WallDraft::AwaitingEnd { start: Vec2::ZERO })
		.insert_resource(snapped_at(Vec2::X))
		.add_systems(
			PostUpdate,
			count_changes
				.run_if(resource_changed::<EditPreview>)
				.after(WallToolDrawingSystems),
		);
	app.update();
	app.update();
	assert_eq!(app.world().resource::<PreviewChanges>().0, 1);
	app.insert_resource(snapped_at(Vec2::Y));
	app.update();
	assert_eq!(app.world().resource::<PreviewChanges>().0, 2);
}

#[test]
fn house_changes_refresh_an_identical_proposal_even_off_the_ground() {
	let mut app = drawing_app();
	app.insert_resource(WallDraft::AwaitingEnd { start: Vec2::ZERO })
		.insert_resource(snapped_at(Vec2::X));
	app.update();
	let candidate = propose_wall(
		app.world().resource::<House>().graph(),
		&WallProposal {
			start: Vec2::ZERO,
			end: Vec2::Y,
			dimensions: WallDimensions::default(),
		},
	)
	.unwrap();
	app.world_mut().resource_mut::<House>().commit(candidate);
	app.insert_resource(SnappedPointer::default());
	app.update();
	let state = app
		.world()
		.resource::<EditPreview>()
		.state
		.as_ref()
		.unwrap();
	assert_eq!(state.house_revision, 1);
	assert_eq!(state.outcome.as_ref().unwrap().graph().walls().count(), 2);
}

#[test]
fn confirmation_commits_the_exact_previewed_mesh() {
	let mut app = drawing_app();
	app.insert_resource(WallDraft::AwaitingEnd { start: Vec2::ZERO })
		.insert_resource(snapped_at(Vec2::X));
	app.update();
	let expected_mesh = app
		.world()
		.resource::<EditPreview>()
		.valid_candidate()
		.unwrap()
		.mesh()
		.clone();
	app.insert_resource(pressed())
		.insert_resource(pointer_at(Vec2::new(100.0, 0.0)));
	app.update();
	assert_eq!(app.world().resource::<House>().mesh(), &expected_mesh);
	assert!(app.world().resource::<EditPreview>().state.is_none());
	assert_eq!(*app.world().resource::<WallDraft>(), WallDraft::Idle);
}

#[test]
fn current_frame_pointer_eligibility_is_used_for_clicks() {
	let mut app = drawing_app();
	app.insert_resource(pressed())
		.insert_resource(pointer_at(Vec2::ZERO))
		.insert_resource(snapped_at(Vec2::ZERO))
		.add_systems(
			PostUpdate,
			(|mut pointer: ResMut<GroundPointer>| pointer.hit = None)
				.in_set(crate::ground_pointer::GroundPointerSystems::Pick),
		)
		.add_systems(
			PostUpdate,
			(|ground: Res<GroundPointer>, mut snapped: ResMut<SnappedPointer>| {
				if ground.hit.is_none() {
					snapped.target = None;
				}
			})
			.in_set(crate::snapping::SnappingSystems::Pick),
		);
	app.update();
	assert_eq!(*app.world().resource::<WallDraft>(), WallDraft::Idle);
}

#[test]
fn dimension_steps_respect_minimums() {
	let mut settings = WallDrawingSettings::default();
	for _ in 0..100 {
		settings.step_height(StepDirection::Decrease);
		settings.step_thickness(StepDirection::Decrease);
	}
	assert_eq!(settings.dimensions().height, MINIMUM_HEIGHT_METRES);
	assert_eq!(settings.dimensions().thickness, MINIMUM_THICKNESS_METRES);
	settings.step_height(StepDirection::Increase);
	settings.step_thickness(StepDirection::Increase);
	assert!(settings.dimensions().height > MINIMUM_HEIGHT_METRES);
	assert!(settings.dimensions().thickness > MINIMUM_THICKNESS_METRES);
}
