use super::*;
use crate::edit::PreviewState;
use crate::tests::support::pressed_mouse as clicked;
use crate::{
	coordinates::graph_to_bevy,
	editor_schedule::EditorSchedulePlugin,
	opening_category::{OpeningCategories, OpeningCategory},
};
use wall_graph::OpeningId;

/// A 6 m wall with a window 0.9 m wide centred 3 m along it.
fn house_with_window() -> (House, OpeningId) {
	let (house, _, opening) =
		crate::tests::support::house_with_opening(6.0, 3.0, OpeningCategory::Window);
	(house, opening)
}

fn delete_app(house: House) -> App {
	let mut app = App::new();
	// The wall-edit plugin discards wall-deletion previews it considers stale, so it must run
	// alongside the delete tool for these tests to mean anything.
	app.add_plugins((
		EditorSchedulePlugin,
		WallToolDeletePlugin,
		crate::wall_tool_edit_wall::WallToolEditWallPlugin,
	))
	.insert_resource(ActiveTool(Tool::Delete))
	.insert_resource(house)
	.init_resource::<GroundPointer>()
	.init_resource::<crate::wall_tool_selection::SelectedTarget>()
	.init_resource::<ButtonInput<MouseButton>>()
	.init_resource::<ButtonInput<KeyCode>>();
	app
}

/// A ray along graph Y through the wall at `x` and `elevation`, from the south.
fn ray_across(x: f32, elevation: f32) -> GroundPointer {
	GroundPointer {
		ray: Some(Ray3d::new(
			graph_to_bevy(Vec3::new(x, -5.0, elevation)),
			Dir3::new(graph_to_bevy(Vec3::Y)).unwrap(),
		)),
		..Default::default()
	}
}

#[test]
fn hovering_a_doorway_previews_deleting_the_opening_only() {
	let (house, id) = house_with_window();
	let mut app = delete_app(house);
	app.insert_resource(ray_across(3.0, 1.0));
	app.update();

	let preview = app.world().resource::<EditPreview>();
	assert!(matches!(
		preview.state,
		Some(PreviewState {
			proposal: Proposal::OpeningRemoval(opening),
			..
		}) if opening == id
	));
	assert!(preview.valid_candidate().is_some());
	assert_eq!(
		app.world().resource::<House>().graph().openings().count(),
		1
	);
}

#[test]
fn clicking_an_opening_deletes_it_and_its_category_but_not_the_wall() {
	let (house, id) = house_with_window();
	let mut app = delete_app(house);
	app.insert_resource(ray_across(3.0, 1.0))
		.insert_resource(clicked());
	app.update();

	let house = app.world().resource::<House>();
	assert!(house.graph().opening(id).is_none());
	assert_eq!(
		house.committed().categories(),
		&OpeningCategories::default()
	);
	assert_eq!(house.graph().walls().count(), 1);
	assert!(app.world().resource::<EditPreview>().state.is_none());
}

#[test]
fn clicking_solid_wall_deletes_the_wall_with_its_openings() {
	let (house, _) = house_with_window();
	let mut app = delete_app(house);
	app.insert_resource(ray_across(5.0, 1.0));
	app.update();
	assert!(app.world().resource::<EditPreview>().holds_wall_removal());

	app.insert_resource(clicked());
	app.update();

	let house = app.world().resource::<House>();
	assert_eq!(house.graph().walls().count(), 0);
	assert_eq!(house.graph().openings().count(), 0);
	assert_eq!(
		house.committed().categories(),
		&OpeningCategories::default()
	);
}

#[test]
fn clicking_empty_space_deletes_nothing() {
	let (house, _) = house_with_window();
	let revision = house.revision();
	let mut app = delete_app(house);
	app.insert_resource(ray_across(9.0, 1.0))
		.insert_resource(clicked());
	app.update();

	assert_eq!(app.world().resource::<House>().revision(), revision);
	assert!(app.world().resource::<EditPreview>().state.is_none());
}

#[test]
fn moving_off_the_target_or_leaving_the_tool_clears_the_preview() {
	let (house, _) = house_with_window();
	let mut app = delete_app(house);
	app.insert_resource(ray_across(5.0, 1.0));
	app.update();
	app.insert_resource(ray_across(9.0, 1.0));
	app.update();
	assert!(app.world().resource::<EditPreview>().state.is_none());

	app.insert_resource(ray_across(5.0, 1.0));
	app.update();
	assert!(app.world().resource::<EditPreview>().holds_removal());
	app.insert_resource(ActiveTool(Tool::DrawWall));
	app.update();
	assert!(app.world().resource::<EditPreview>().state.is_none());
}

#[test]
fn a_wall_deletion_preview_survives_across_frames() {
	let (house, _) = house_with_window();
	let mut app = delete_app(house);
	app.insert_resource(ray_across(5.0, 1.0));
	for _ in 0..3 {
		app.update();
		assert!(app.world().resource::<EditPreview>().holds_wall_removal());
	}
}
