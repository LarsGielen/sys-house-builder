use super::*;
use crate::edit::EditCandidate;
use crate::tests::support::{press_key, tool_test_app};
use crate::{editor_schedule::EditorSchedulePlugin, wall_tool_drawing::WallDrawingSettings};
use bevy::input_focus::InputFocus;
use wall_graph::WallDimensions;

fn bend_proposal() -> ArcProposal {
	ArcProposal {
		start: Vec2::ZERO,
		end: Vec2::new(4.0, 0.0),
		definition: ArcDefinition::Bend(Vec2::new(2.0, 1.0)),
		dimensions: WallDimensions::default(),
	}
}

/// An arc at its bend stage, previewed, with `field` focused and holding `text`.
fn fields_app(field: ArcField, text: &str) -> App {
	let mut app = tool_test_app();
	app.add_plugins((
		EditorSchedulePlugin,
		crate::text_fields::TextFieldsPlugin,
		ArcFieldsPlugin,
	))
	.init_resource::<House>()
	.init_resource::<crate::ground_pointer::GroundPointer>()
	.init_resource::<WallDrawingSettings>();
	let proposal = bend_proposal();
	app.insert_resource(ArcDraft::AwaitingBend {
		start: proposal.start,
		end: proposal.end,
		bend: Vec2::new(2.0, 1.0),
		typed_sweep: None,
	});
	{
		let world = app.world_mut();
		let revision = world.resource::<House>().revision();
		let mut preview = world.resource_mut::<EditPreview>();
		EditPreview::update(
			preview.reborrow(),
			Proposal::Arc(proposal),
			revision,
			&EditCandidate::default(),
		);
	}
	let entity = app.world_mut().spawn((field, EditableText::new(text))).id();
	app.insert_resource(InputFocus::from_entity(entity));
	app
}

fn typed_sweep(app: &App) -> Option<f32> {
	match *app.world().resource::<ArcDraft>() {
		ArcDraft::AwaitingBend { typed_sweep, .. } => typed_sweep,
		_ => None,
	}
}

fn previewed_arc(app: &App) -> ArcProposal {
	super::previewed_arc(app.world().resource::<EditPreview>()).unwrap()
}

#[test]
fn a_typed_angle_replaces_the_pointer_bend_and_previews() {
	let mut app = fields_app(ArcField::Angle, "90");
	app.update();

	let sweep = typed_sweep(&app).unwrap();
	assert!((sweep - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
	assert_eq!(previewed_arc(&app).definition, ArcDefinition::Sweep(sweep));
	assert!(
		app.world()
			.resource::<EditPreview>()
			.valid_candidate()
			.is_some()
	);
	assert_eq!(app.world().resource::<House>().revision(), 0);
}

#[test]
fn a_typed_negative_angle_bends_the_other_way() {
	let mut app = fields_app(ArcField::Angle, "-90");
	app.update();
	assert!(typed_sweep(&app).unwrap() < 0.0);
}

#[test]
fn a_typed_radius_keeps_the_side_the_bend_chose() {
	let mut app = fields_app(ArcField::Radius, "3");
	app.update();

	let bend_side = bend_proposal().sweep().unwrap().signum();
	let sweep = typed_sweep(&app).unwrap();
	assert_eq!(sweep.signum(), bend_side);
	assert!(
		(wall_graph::arc_radius(Vec2::ZERO, Vec2::new(4.0, 0.0), sweep).unwrap() - 3.0).abs()
			< 1e-4
	);
}

#[test]
fn a_radius_that_cannot_span_the_endpoints_is_reported_and_changes_nothing() {
	let mut app = fields_app(ArcField::Radius, "1");
	app.update();
	assert_eq!(typed_sweep(&app), None);
	assert!(app.world().resource::<EditNotice>().message.is_some());
}

#[test]
fn text_that_is_not_a_number_yet_leaves_the_preview_alone() {
	let mut app = fields_app(ArcField::Angle, "-");
	app.update();
	assert_eq!(typed_sweep(&app), None);
}

#[test]
fn enter_commits_the_typed_arc_once() {
	let mut app = fields_app(ArcField::Angle, "90");
	app.update();
	press_key(&mut app, KeyCode::Enter);
	app.update();

	let house = app.world().resource::<House>();
	assert_eq!(house.graph().walls().count(), 1);
	assert_eq!(house.revision(), 1);
	assert_eq!(*app.world().resource::<ArcDraft>(), ArcDraft::Idle);
	assert!(app.world().resource::<EditPreview>().state.is_none());
}

#[test]
fn an_invalid_typed_arc_stays_a_draft_with_its_reason() {
	let mut app = fields_app(ArcField::Angle, "400");
	app.update();
	press_key(&mut app, KeyCode::Enter);
	app.update();

	assert_eq!(app.world().resource::<House>().revision(), 0);
	assert!(app.world().resource::<EditPreview>().state.is_some());
	assert!(matches!(
		*app.world().resource::<ArcDraft>(),
		ArcDraft::AwaitingBend { .. }
	));
}

#[test]
fn escape_returns_the_arc_to_the_pointer_bend() {
	let mut app = fields_app(ArcField::Angle, "90");
	app.update();
	assert!(typed_sweep(&app).is_some());
	press_key(&mut app, KeyCode::Escape);
	app.update();
	assert_eq!(typed_sweep(&app), None);
}
