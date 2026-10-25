use super::*;
use crate::document::edit::WallProposal;
use crate::document::edit::propose_wall;
use crate::input::Action;
use crate::tests::support::{pressed_mouse, released_mouse};
use crate::tools::active::ActiveTool;
use crate::{
	viewport::ground_pointer::PointerOnGround,
	viewport::snapping::{SnapSource, SnapTarget},
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

#[test]
fn zero_length_proposal_is_an_invalid_preview() {
	let mut app = drawing_app();
	app.insert_resource(WallDraftState(WallDraft::AwaitingEnd { start: Vec2::ONE }))
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
	app.insert_resource(WallDraftState(WallDraft::Pressed {
		start: Vec2::ZERO,
		press_position: Vec2::ZERO,
		dragged: true,
	}))
	.insert_resource(released_mouse())
	.insert_resource(snapped_at(Vec2::ZERO));
	app.update();
	assert_eq!(
		app.world().resource::<WallDraftState>().0,
		WallDraft::AwaitingEnd { start: Vec2::ZERO }
	);
	assert_eq!(app.world().resource::<House>().revision(), 0);
	app.insert_resource(pressed_mouse())
		.insert_resource(pointer_at(Vec2::new(100.0, 0.0)))
		.insert_resource(snapped_at(Vec2::X));
	app.update();
	assert_eq!(app.world().resource::<WallDraftState>().0, WallDraft::Idle);
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
		.insert_resource(WallDraftState(WallDraft::AwaitingEnd { start: Vec2::ZERO }))
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
	app.insert_resource(WallDraftState(WallDraft::AwaitingEnd { start: Vec2::ZERO }))
		.insert_resource(snapped_at(Vec2::X));
	app.update();
	let candidate = propose_wall(
		app.world().resource::<House>().committed(),
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
	app.insert_resource(WallDraftState(WallDraft::AwaitingEnd { start: Vec2::ZERO }))
		.insert_resource(snapped_at(Vec2::X));
	app.update();
	let expected_mesh = app
		.world()
		.resource::<EditPreview>()
		.valid_candidate()
		.unwrap()
		.mesh()
		.clone();
	app.insert_resource(pressed_mouse())
		.insert_resource(pointer_at(Vec2::new(100.0, 0.0)));
	app.update();
	assert_eq!(app.world().resource::<House>().mesh(), &expected_mesh);
	assert!(app.world().resource::<EditPreview>().state.is_none());
	assert_eq!(app.world().resource::<WallDraftState>().0, WallDraft::Idle);
}

#[test]
fn current_frame_pointer_eligibility_is_used_for_clicks() {
	let mut app = drawing_app();
	app.insert_resource(pressed_mouse())
		.insert_resource(pointer_at(Vec2::ZERO))
		.insert_resource(snapped_at(Vec2::ZERO))
		.add_systems(
			PostUpdate,
			(|mut pointer: ResMut<GroundPointer>| pointer.hit = None)
				.in_set(crate::viewport::ground_pointer::GroundPointerSystems::Pick),
		)
		.add_systems(
			PostUpdate,
			(|ground: Res<GroundPointer>, mut snapped: ResMut<SnappedPointer>| {
				if ground.hit.is_none() {
					snapped.target = None;
				}
			})
			.in_set(crate::viewport::snapping::SnappingSystems::Pick),
		);
	app.update();
	assert_eq!(app.world().resource::<WallDraftState>().0, WallDraft::Idle);
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

#[test]
fn a_rebound_cancel_action_cancels_the_draft_and_the_old_key_does_not() {
	use crate::input::{Binding, Bindings, Context, ModifierRule, Trigger};

	let mut app = drawing_app();
	app.insert_resource(WallDraftState(WallDraft::AwaitingEnd { start: Vec2::ZERO }));
	{
		let mut bindings = app.world_mut().resource_mut::<Bindings>();
		bindings
			.0
			.retain(|binding| binding.action != Action::Cancel);
		bindings.0.push(Binding {
			trigger: Trigger::Key(KeyCode::KeyQ),
			control: ModifierRule::Ignored,
			shift: ModifierRule::Ignored,
			context: Context::Viewport,
			action: Action::Cancel,
		});
	}

	crate::tests::support::press_key(&mut app, KeyCode::Escape);
	app.update();
	assert_eq!(
		app.world().resource::<WallDraftState>().0,
		WallDraft::AwaitingEnd { start: Vec2::ZERO }
	);

	crate::tests::support::press_key(&mut app, KeyCode::KeyQ);
	app.update();
	assert_eq!(app.world().resource::<WallDraftState>().0, WallDraft::Idle);
}

#[test]
fn a_settings_action_steps_the_wall_height_like_the_button_does() {
	use crate::actions::settings::SettingsAction;

	let mut app = drawing_app();
	let before = app
		.world()
		.resource::<WallDrawingSettings>()
		.dimensions()
		.height;
	app.world_mut()
		.write_message(crate::actions::EditorAction::from(
			SettingsAction::StepWallHeight(StepDirection::Increase),
		));
	app.update();
	assert!(
		app.world()
			.resource::<WallDrawingSettings>()
			.dimensions()
			.height > before
	);
}
