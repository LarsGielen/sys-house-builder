use super::*;
use crate::tests::support::press_key as press;
use crate::tools::selection::SelectionTarget;
use crate::{editor_schedule::EditorSchedulePlugin, viewport::ground_pointer::GroundPointer};
use bevy::input_focus::InputFocus;

fn house_with_wall() -> (House, Wall) {
	crate::tests::support::house_with_wall(Vec2::ZERO, Vec2::new(4.0, 0.0))
}

/// An app with the wall selected and `field` focused, holding `text`.
fn fields_app(field: WallField, text: &str) -> (App, Wall, Entity) {
	let (house, wall) = house_with_wall();
	let mut app = App::new();
	app.add_plugins((
		EditorSchedulePlugin,
		crate::ui::fields::TextFieldsPlugin,
		WallFieldsPlugin,
	))
	.insert_resource(house)
	.insert_resource(SelectedTarget(Some(SelectionTarget::Wall(wall))))
	.init_resource::<WallSelectionCarryOver>()
	.init_resource::<EditNotice>()
	.init_resource::<GroundPointer>()
	.init_resource::<ButtonInput<MouseButton>>()
	.init_resource::<ButtonInput<KeyCode>>();
	let entity = app.world_mut().spawn((field, EditableText::new(text))).id();
	app.insert_resource(InputFocus::from_entity(entity));
	(app, wall, entity)
}

#[test]
fn typing_previews_the_new_mesh_without_changing_the_house() {
	let (mut app, wall, _) = fields_app(WallField::Height, "3.2");
	let revision = app.world().resource::<House>().revision();
	app.update();

	let Some(PreviewState {
		proposal: Proposal::WallDimensions(edit),
		outcome: Ok(candidate),
		..
	}) = &app.world().resource::<EditPreview>().state
	else {
		panic!("expected a valid dimensions preview");
	};
	assert_eq!(edit.wall, wall);
	assert!((edit.dimensions.height - 3.2).abs() < 1e-6);
	assert_ne!(candidate.mesh(), app.world().resource::<House>().mesh());
	assert_eq!(app.world().resource::<House>().revision(), revision);
}

#[test]
fn typing_the_current_value_previews_nothing() {
	let (mut app, _, _) = fields_app(WallField::Height, "2.5");
	app.update();

	assert!(app.world().resource::<EditPreview>().state.is_none());
}

#[test]
fn a_rejected_value_stays_previewed_with_its_reason_and_enter_does_not_apply_it() {
	let (mut app, wall, _) = fields_app(WallField::Height, "0");
	app.update();
	let revision = app.world().resource::<House>().revision();
	press(&mut app, KeyCode::Enter);
	app.update();

	assert!(matches!(
		app.world().resource::<EditPreview>().state,
		Some(PreviewState {
			outcome: Err(_),
			..
		})
	));
	let house = app.world().resource::<House>();
	assert_eq!(house.revision(), revision);
	assert_eq!(
		house.graph().wall_dimensions(wall),
		Some(WallDimensions::default())
	);
}

#[test]
fn escape_drops_the_preview() {
	let (mut app, _, _) = fields_app(WallField::Height, "3.2");
	app.update();
	assert!(
		app.world()
			.resource::<EditPreview>()
			.holds_wall_dimensions()
	);

	press(&mut app, KeyCode::Escape);
	app.update();

	assert!(app.world().resource::<EditPreview>().state.is_none());
}

#[test]
fn unfocused_fields_show_the_selected_wall() {
	let (mut app, _, _) = fields_app(WallField::Height, "typing");
	let thickness = app
		.world_mut()
		.spawn((WallField::Thickness, EditableText::new("")))
		.id();
	app.update();

	let shown = app
		.world()
		.get::<EditableText>(thickness)
		.unwrap()
		.value()
		.to_string();
	assert_eq!(shown, "0.200");
}

#[test]
fn enter_with_unparseable_text_does_not_commit_an_older_preview() {
	let (mut app, wall, field) = fields_app(WallField::Height, "3.2");
	app.update();
	let revision = app.world().resource::<House>().revision();
	app.world_mut()
		.entity_mut(field)
		.insert(EditableText::new("-"));
	press(&mut app, KeyCode::Enter);
	app.update();
	let house = app.world().resource::<House>();
	assert_eq!(house.revision(), revision);
	assert_eq!(
		house.graph().wall_dimensions(wall),
		Some(WallDimensions::default())
	);
	assert!(app.world().resource::<EditNotice>().message.is_some());
}

#[test]
fn field_commits_apply_dimensions_and_keep_the_wall_selected_after_the_next_frame() {
	for (field_kind, text, expected) in [
		(
			WallField::Height,
			"3.2",
			WallDimensions {
				height: 3.2,
				..WallDimensions::default()
			},
		),
		(
			WallField::Thickness,
			"0.3",
			WallDimensions {
				thickness: 0.3,
				..WallDimensions::default()
			},
		),
	] {
		let (house, wall) = house_with_wall();
		let mut app = crate::tests::support::editor_app();
		app.insert_resource(house);
		app.update();
		app.insert_resource(SelectedTarget(Some(SelectionTarget::Wall(wall))));
		let field = app
			.world_mut()
			.spawn((field_kind, EditableText::new(text)))
			.id();
		app.insert_resource(InputFocus::from_entity(field));
		app.update();
		app.world_mut().resource_mut::<EditNotice>().message = Some("An earlier failure".into());
		let revision = app.world().resource::<House>().revision();
		press(&mut app, KeyCode::Enter);
		app.update();
		app.insert_resource(ButtonInput::<KeyCode>::default());
		app.update();
		let house = app.world().resource::<House>();
		assert_eq!(house.graph().wall_dimensions(wall), Some(expected));
		assert_eq!(house.revision(), revision + 1);
		assert!(app.world().resource::<EditPreview>().state.is_none());
		assert_eq!(app.world().resource::<SelectedTarget>().wall(), Some(wall));
		assert_eq!(app.world().resource::<WallSelectionCarryOver>().0, None);
		assert!(app.world().resource::<EditNotice>().message.is_none());
	}
}

#[test]
fn property_actions_edit_a_wall_without_any_widget() {
	let (mut app, wall, entity) = fields_app(WallField::Height, "");
	app.world_mut().entity_mut(entity).despawn();
	app.insert_resource(InputFocus::default());
	let revision = app.world().resource::<House>().revision();
	app.world_mut()
		.write_message(crate::actions::EditorAction::from(
			PropertyAction::PreviewWall {
				revision,
				wall,
				property: WallProperty::Height,
				value: 3.0,
			},
		));
	app.world_mut()
		.write_message(crate::actions::EditorAction::from(
			PropertyAction::SubmitWall {
				revision,
				wall,
				property: WallProperty::Height,
				value: 3.0,
			},
		));
	app.update();

	let house = app.world().resource::<House>();
	assert_eq!(house.revision(), revision + 1);
	assert!((house.graph().wall_dimensions(wall).unwrap().height - 3.0).abs() < 1e-6);
}

#[test]
fn a_submit_that_cannot_be_represented_is_refused_with_a_notice() {
	let (mut app, wall, entity) = fields_app(WallField::Height, "");
	app.world_mut().entity_mut(entity).despawn();
	app.insert_resource(InputFocus::default());
	let revision = app.world().resource::<House>().revision();
	app.world_mut()
		.write_message(crate::actions::EditorAction::from(
			PropertyAction::SubmitWall {
				revision,
				wall,
				property: WallProperty::Height,
				value: 1e300,
			},
		));
	app.update();

	assert_eq!(app.world().resource::<House>().revision(), revision);
	assert!(app.world().resource::<EditNotice>().message.is_some());
}

#[test]
fn an_action_made_for_an_older_document_is_refused() {
	let (mut app, wall, entity) = fields_app(WallField::Height, "");
	app.world_mut().entity_mut(entity).despawn();
	app.insert_resource(InputFocus::default());
	let revision = app.world().resource::<House>().revision();
	app.world_mut()
		.write_message(crate::actions::EditorAction::from(
			PropertyAction::SubmitWall {
				revision: revision - 1,
				wall,
				property: WallProperty::Height,
				value: 3.0,
			},
		));
	app.update();

	assert_eq!(app.world().resource::<House>().revision(), revision);
	assert!(app.world().resource::<EditNotice>().message.is_some());
}
