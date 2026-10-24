use super::*;
use crate::ground_pointer::GroundPointer;
use crate::tests::support::press_key as press;
use crate::wall_tool_selection::SelectionTarget;
use crate::{editor_schedule::EditorSchedulePlugin, opening_category::OpeningCategory};
use wall_graph::OpeningId;

fn house_with_door() -> (House, OpeningId) {
	let (house, _, opening) =
		crate::tests::support::house_with_opening(8.0, 3.0, OpeningCategory::Door);
	(house, opening)
}

/// An app with one focused field holding `text`.
fn fields_app(field: impl Component, text: &str) -> (App, OpeningId, Entity) {
	let (house, id) = house_with_door();
	let mut app = App::new();
	app.add_plugins((
		EditorSchedulePlugin,
		crate::text_fields::TextFieldsPlugin,
		OpeningFieldsPlugin,
	))
	.insert_resource(house)
	.insert_resource(SelectedTarget(Some(SelectionTarget::Opening(id))))
	.init_resource::<SnapSettings>()
	.init_resource::<GroundPointer>()
	.init_resource::<EditPreview>()
	.init_resource::<EditNotice>()
	.init_resource::<ButtonInput<MouseButton>>()
	.init_resource::<ButtonInput<KeyCode>>();
	let entity = app.world_mut().spawn((field, EditableText::new(text))).id();
	app.insert_resource(InputFocus::from_entity(entity));
	(app, id, entity)
}

#[test]
fn enter_applies_a_valid_value_as_one_commit_on_the_same_opening() {
	let (mut app, id, _) = fields_app(OpeningField::Width, "1.2");
	press(&mut app, KeyCode::Enter);
	app.update();

	let house = app.world().resource::<House>();
	let spec = house.graph().opening(id).unwrap().spec;
	assert!((spec.width - 1.2).abs() < 1e-6);
	assert_eq!(spec.center_distance, 3.0);
	assert_eq!(
		house.committed().categories().get(id),
		Some(OpeningCategory::Door)
	);
	assert!(app.world().resource::<EditPreview>().state.is_none());
}

#[test]
fn an_invalid_value_stays_a_draft_and_changes_nothing() {
	let (mut app, id, _) = fields_app(OpeningField::Width, "20");
	let revision = app.world().resource::<House>().revision();
	press(&mut app, KeyCode::Enter);
	app.update();

	let house = app.world().resource::<House>();
	assert_eq!(house.revision(), revision);
	assert!((house.graph().opening(id).unwrap().spec.width - 0.9).abs() < 1e-6);
	let Some(PreviewState {
		proposal: Proposal::OpeningEdit(draft),
		outcome: Err(_),
		..
	}) = app.world().resource::<EditPreview>().state
	else {
		panic!("expected a rejected draft");
	};
	assert_eq!(draft.spec.width, 20.0);
}

#[test]
fn the_pending_draft_is_the_base_of_the_next_edit() {
	let (mut app, id, field) = fields_app(OpeningField::Width, "20");
	press(&mut app, KeyCode::Enter);
	app.update();

	// Move the same draft's position; the rejected width is still part of it.
	app.world_mut()
		.entity_mut(field)
		.insert(OpeningField::CenterDistance);
	app.world_mut()
		.entity_mut(field)
		.insert(EditableText::new("4"));
	press(&mut app, KeyCode::Enter);
	app.update();

	let Some(PreviewState {
		proposal: Proposal::OpeningEdit(draft),
		..
	}) = app.world().resource::<EditPreview>().state
	else {
		panic!("expected a draft");
	};
	assert_eq!(draft.opening, id);
	assert_eq!((draft.spec.width, draft.spec.center_distance), (20.0, 4.0));
}

#[test]
fn text_that_is_not_a_number_is_reported_and_changes_nothing() {
	let (mut app, _, _) = fields_app(OpeningField::Height, "-");
	let revision = app.world().resource::<House>().revision();
	press(&mut app, KeyCode::Enter);
	app.update();

	assert_eq!(app.world().resource::<House>().revision(), revision);
	assert!(app.world().resource::<EditNotice>().message.is_some());
}

#[test]
fn escape_reverts_the_draft_and_leaves_the_field() {
	let (mut app, _, _) = fields_app(OpeningField::Width, "20");
	press(&mut app, KeyCode::Enter);
	app.update();
	assert!(app.world().resource::<EditPreview>().state.is_some());

	press(&mut app, KeyCode::Escape);
	app.update();

	assert!(app.world().resource::<EditPreview>().state.is_none());
	assert_eq!(app.world().resource::<InputFocus>().get(), None);
}

#[test]
fn the_snap_gap_field_sets_the_gap() {
	let (mut app, _, _) = fields_app(SnapGapField, "0.15");
	press(&mut app, KeyCode::Enter);
	app.update();

	assert!((app.world().resource::<SnapSettings>().opening_gap() - 0.15).abs() < 1e-6);
}

#[test]
fn unfocused_fields_show_the_selected_opening_but_the_focused_one_is_left_alone() {
	let (mut app, _, focused) = fields_app(OpeningField::Width, "typing");
	let position = app
		.world_mut()
		.spawn((OpeningField::CenterDistance, EditableText::new("")))
		.id();
	app.update();

	let text_of = |app: &App, entity| {
		app.world()
			.get::<EditableText>(entity)
			.unwrap()
			.value()
			.to_string()
	};
	assert_eq!(text_of(&app, position), "3.000");
	assert_eq!(text_of(&app, focused), "typing");
}

#[test]
fn invalid_snap_gaps_leave_the_previous_setting_unchanged_and_explain_why() {
	for text in [
		"-1",
		"NaN",
		"inf",
		"99999999999999999999999999999999999999999",
	] {
		let (mut app, _, _) = fields_app(SnapGapField, text);
		app.world_mut()
			.resource_mut::<SnapSettings>()
			.set_opening_gap(0.15)
			.unwrap();
		press(&mut app, KeyCode::Enter);
		app.update();
		assert_eq!(
			app.world().resource::<SnapSettings>().opening_gap(),
			0.15,
			"{text}"
		);
		assert!(
			app.world().resource::<EditNotice>().message.is_some(),
			"{text}"
		);
	}
}

#[test]
fn enter_includes_text_edits_queued_in_the_same_frame() {
	use bevy::text::{EditableTextSystems, TextEdit, TextPlugin};
	use bevy::ui::UiSystems;
	let (mut app, id, field) = fields_app(OpeningField::Width, "1.");
	app.add_plugins((
		MinimalPlugins,
		bevy::asset::AssetPlugin::default(),
		TextPlugin,
	));
	// UiPlugin puts pending text edits in this set in the full application.
	app.configure_sets(PostUpdate, EditableTextSystems.in_set(UiSystems::Content));
	app.world_mut()
		.get_mut::<EditableText>(field)
		.unwrap()
		.queue_edit(TextEdit::Insert("2".into()));
	press(&mut app, KeyCode::Enter);
	app.update();
	assert_eq!(
		app.world()
			.get::<EditableText>(field)
			.unwrap()
			.value()
			.to_string(),
		"1.2"
	);
	assert_eq!(
		app.world()
			.resource::<House>()
			.graph()
			.opening(id)
			.unwrap()
			.spec
			.width,
		1.2
	);
}
