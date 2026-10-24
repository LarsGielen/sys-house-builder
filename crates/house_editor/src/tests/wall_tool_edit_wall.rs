use super::*;
use crate::tests::support::press_key as press;
use crate::wall_tool_selection::SelectionTarget;
use crate::{
	edit::{OpeningProposal, WallProposal, propose_opening, propose_wall},
	editor_schedule::EditorSchedulePlugin,
	opening_category::OpeningCategory,
};
use wall_graph::{OpeningSpec, WallDimensions};

/// One 6 m wall with a door and a window.
fn house_with_two_openings() -> (House, wall_graph::Wall) {
	let mut house = House::default();
	house.commit(
		propose_wall(
			house.committed(),
			&WallProposal {
				start: Vec2::ZERO,
				end: Vec2::new(6.0, 0.0),
				dimensions: WallDimensions::default(),
			},
		)
		.unwrap(),
	);
	let wall = house.graph().walls().next().unwrap();
	for (center_distance, category) in
		[(1.5, OpeningCategory::Door), (4.5, OpeningCategory::Window)]
	{
		let proposal = OpeningProposal {
			wall,
			spec: OpeningSpec {
				center_distance,
				width: 0.9,
				bottom: 0.0,
				height: 2.1,
			},
			category,
		};
		house.commit(propose_opening(house.committed(), &proposal).unwrap());
	}
	(house, wall)
}

fn edit_wall_app(house: House, selected: Option<wall_graph::Wall>) -> App {
	let mut app = App::new();
	app.add_plugins((EditorSchedulePlugin, WallToolEditWallPlugin))
		.insert_resource(ActiveTool(Tool::Select))
		.insert_resource(house)
		.insert_resource(SelectedTarget(selected.map(SelectionTarget::Wall)))
		.init_resource::<ButtonInput<KeyCode>>();
	app
}

#[test]
fn requesting_deletion_previews_it_without_changing_the_house() {
	let (house, wall) = house_with_two_openings();
	let revision = house.revision();
	let mut app = edit_wall_app(house, Some(wall));
	app.world_mut().write_message(DeleteSelectedWall);
	app.update();

	assert!(app.world().resource::<EditPreview>().holds_wall_removal());
	assert!(
		app.world()
			.resource::<EditPreview>()
			.valid_candidate()
			.is_some()
	);
	let house = app.world().resource::<House>();
	assert_eq!(house.revision(), revision);
	assert_eq!(house.graph().openings().count(), 2);
}

#[test]
fn confirming_removes_the_wall_its_openings_and_their_categories_in_one_commit() {
	let (house, wall) = house_with_two_openings();
	let revision = house.revision();
	let mut app = edit_wall_app(house, Some(wall));
	app.world_mut().write_message(DeleteSelectedWall);
	app.update();
	app.world_mut().write_message(ConfirmWallDeletion);
	app.update();

	let house = app.world().resource::<House>();
	assert_eq!(house.graph().walls().count(), 0);
	assert_eq!(house.graph().openings().count(), 0);
	assert_eq!(
		house.committed().categories(),
		&crate::opening_category::OpeningCategories::default()
	);
	assert_eq!(house.revision(), revision + 1);
	assert!(app.world().resource::<EditPreview>().state.is_none());
}

#[test]
fn enter_confirms_and_escape_cancels() {
	let (house, wall) = house_with_two_openings();
	let mut app = edit_wall_app(house, Some(wall));
	app.world_mut().write_message(DeleteSelectedWall);
	app.update();
	press(&mut app, KeyCode::Escape);
	app.update();
	assert!(app.world().resource::<EditPreview>().state.is_none());
	assert_eq!(app.world().resource::<House>().graph().walls().count(), 1);

	app.insert_resource(ButtonInput::<KeyCode>::default());
	app.world_mut().write_message(DeleteSelectedWall);
	app.update();
	press(&mut app, KeyCode::Enter);
	app.update();
	assert_eq!(app.world().resource::<House>().graph().walls().count(), 0);
}

#[test]
fn confirming_with_nothing_previewed_does_nothing() {
	let (house, wall) = house_with_two_openings();
	let revision = house.revision();
	let mut app = edit_wall_app(house, Some(wall));
	app.world_mut().write_message(ConfirmWallDeletion);
	app.update();

	assert_eq!(app.world().resource::<House>().revision(), revision);
}

#[test]
fn a_deletion_preview_is_dropped_when_the_selection_moves_away() {
	let (house, wall) = house_with_two_openings();
	let mut app = edit_wall_app(house, Some(wall));
	app.world_mut().write_message(DeleteSelectedWall);
	app.update();
	assert!(app.world().resource::<EditPreview>().holds_wall_removal());

	app.insert_resource(SelectedTarget(None));
	app.update();

	assert!(app.world().resource::<EditPreview>().state.is_none());
}
