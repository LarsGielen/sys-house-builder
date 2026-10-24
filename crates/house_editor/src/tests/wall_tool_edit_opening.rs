use super::*;
use crate::tests::support::pressed_mouse as pressed;
use crate::tests::support::released_mouse as released;
use crate::{
	editor_schedule::EditorSchedulePlugin,
	ground_pointer::PointerOnGround,
	opening_category::OpeningCategory,
	wall_tool_selection::{HoveredTarget, SelectedTarget, SelectionTarget},
};

/// A wall of `length` along X with one window, 0.9 m wide, centred at `center_distance`.
fn house_with_window(length: f32, center_distance: f64) -> (House, OpeningId) {
	let (house, _, opening) =
		crate::tests::support::house_with_opening(length, center_distance, OpeningCategory::Window);
	(house, opening)
}

fn edit_app(house: House, selected: Option<OpeningId>) -> App {
	let mut app = App::new();
	app.add_plugins((EditorSchedulePlugin, WallToolEditOpeningPlugin))
		.insert_resource(ActiveTool(Tool::Select))
		.insert_resource(house)
		.insert_resource(SelectedTarget(selected.map(SelectionTarget::Opening)))
		.insert_resource(HoveredTarget(selected.map(SelectionTarget::Opening)))
		.init_resource::<SnapSettings>()
		.init_resource::<GroundPointer>()
		.init_resource::<ButtonInput<MouseButton>>()
		.init_resource::<ButtonInput<KeyCode>>();
	app
}

fn pointer_at(pixels: Vec2, graph_position: Vec2) -> GroundPointer {
	GroundPointer {
		hit: Some(PointerOnGround {
			viewport_position: pixels,
			graph_position,
		}),
		..Default::default()
	}
}

fn snapping_off() -> ButtonInput<KeyCode> {
	let mut keyboard = ButtonInput::default();
	keyboard.press(KeyCode::ShiftLeft);
	keyboard
}

/// A drag already past the threshold, grabbed at 3 m along the wall.
fn dragging(opening: OpeningId, house: &House) -> OpeningDrag {
	OpeningDrag::Pressed {
		opening,
		original: house.graph().opening(opening).unwrap().spec,
		grab_distance: 3.0,
		press_position: Vec2::ZERO,
		dragged: true,
	}
}

#[test]
fn pressing_the_selected_opening_starts_a_drag_that_is_not_yet_a_move() {
	let (house, id) = house_with_window(6.0, 3.0);
	let mut app = edit_app(house, Some(id));
	app.insert_resource(pressed())
		.insert_resource(pointer_at(Vec2::new(300.0, 100.0), Vec2::new(3.0, 0.0)));
	app.update();

	assert!(matches!(
		*app.world().resource::<OpeningDrag>(),
		OpeningDrag::Pressed { dragged: false, .. }
	));
	assert!(app.world().resource::<EditPreview>().state.is_none());
}

#[test]
fn pressing_something_other_than_the_selected_opening_starts_no_drag() {
	let (house, id) = house_with_window(6.0, 3.0);
	let mut app = edit_app(house, Some(id));
	app.insert_resource(HoveredTarget(None))
		.insert_resource(pressed())
		.insert_resource(pointer_at(Vec2::new(300.0, 100.0), Vec2::new(3.0, 0.0)));
	app.update();

	assert_eq!(*app.world().resource::<OpeningDrag>(), OpeningDrag::Idle);
}

#[test]
fn dragging_previews_the_move_without_changing_the_house() {
	let (house, id) = house_with_window(8.0, 3.0);
	let drag = dragging(id, &house);
	let mut app = edit_app(house, Some(id));
	app.insert_resource(drag)
		.insert_resource(snapping_off())
		.insert_resource(pointer_at(Vec2::new(500.0, 100.0), Vec2::new(5.0, 0.0)));
	app.update();

	let Some(PreviewState {
		proposal: Proposal::OpeningEdit(edit),
		outcome: Ok(_),
		..
	}) = app.world().resource::<EditPreview>().state
	else {
		panic!("expected a valid edit preview");
	};
	assert_eq!(edit.opening, id);
	assert!((edit.spec.center_distance - 5.0).abs() < 1e-4);
	let house = app.world().resource::<House>();
	assert_eq!(house.graph().opening(id).unwrap().spec.center_distance, 3.0);
}

#[test]
fn releasing_a_valid_drag_moves_the_opening_and_keeps_its_identity_and_category() {
	let (house, id) = house_with_window(8.0, 3.0);
	let drag = dragging(id, &house);
	let mut app = edit_app(house, Some(id));
	app.insert_resource(drag)
		.insert_resource(snapping_off())
		.insert_resource(released())
		.insert_resource(pointer_at(Vec2::new(500.0, 100.0), Vec2::new(5.0, 0.0)));
	app.update();

	let house = app.world().resource::<House>();
	let moved = house.graph().opening(id).unwrap();
	assert!((moved.spec.center_distance - 5.0).abs() < 1e-4);
	assert_eq!(
		house.committed().categories().get(id),
		Some(OpeningCategory::Window)
	);
	assert_eq!(*app.world().resource::<OpeningDrag>(), OpeningDrag::Idle);
	assert!(app.world().resource::<EditPreview>().state.is_none());
}

#[test]
fn releasing_an_invalid_drag_leaves_geometry_and_category_unchanged() {
	let (house, id) = house_with_window(8.0, 3.0);
	let before = house.graph().opening(id).unwrap();
	let drag = dragging(id, &house);
	let mut app = edit_app(house, Some(id));
	// Dragged beyond the wall's end, with snapping off so nothing pulls it back.
	app.insert_resource(drag)
		.insert_resource(snapping_off())
		.insert_resource(pointer_at(Vec2::new(900.0, 100.0), Vec2::new(8.0, 0.0)));
	app.update();
	assert!(matches!(
		app.world().resource::<EditPreview>().state,
		Some(PreviewState {
			outcome: Err(_),
			..
		})
	));

	app.insert_resource(released());
	app.update();

	let house = app.world().resource::<House>();
	assert_eq!(house.graph().opening(id), Some(before));
	assert_eq!(
		house.committed().categories().get(id),
		Some(OpeningCategory::Window)
	);
	assert_eq!(house.revision(), 2);
	assert!(app.world().resource::<EditPreview>().state.is_none());
}

#[test]
fn escape_cancels_a_drag() {
	let (house, id) = house_with_window(8.0, 3.0);
	let drag = dragging(id, &house);
	let mut app = edit_app(house, Some(id));
	app.insert_resource(drag)
		.insert_resource(snapping_off())
		.insert_resource(pointer_at(Vec2::new(500.0, 100.0), Vec2::new(5.0, 0.0)));
	app.update();
	assert!(app.world().resource::<EditPreview>().state.is_some());

	let mut keyboard = ButtonInput::<KeyCode>::default();
	keyboard.press(KeyCode::Escape);
	app.insert_resource(keyboard);
	app.update();

	assert_eq!(*app.world().resource::<OpeningDrag>(), OpeningDrag::Idle);
	assert!(app.world().resource::<EditPreview>().state.is_none());
}

#[test]
fn a_draft_is_dropped_when_the_selection_moves_away() {
	let (house, id) = house_with_window(8.0, 3.0);
	let drag = dragging(id, &house);
	let mut app = edit_app(house, Some(id));
	app.insert_resource(drag)
		.insert_resource(snapping_off())
		.insert_resource(pointer_at(Vec2::new(500.0, 100.0), Vec2::new(5.0, 0.0)));
	app.update();
	assert!(app.world().resource::<EditPreview>().state.is_some());

	app.insert_resource(SelectedTarget(None));
	app.update();

	assert!(app.world().resource::<EditPreview>().state.is_none());
	assert_eq!(*app.world().resource::<OpeningDrag>(), OpeningDrag::Idle);
}

#[test]
fn deleting_removes_the_opening_and_its_category_in_one_commit() {
	let (house, id) = house_with_window(8.0, 3.0);
	let revision = house.revision();
	let mut app = edit_app(house, Some(id));
	app.world_mut().write_message(DeleteSelectedOpening);
	app.update();

	let house = app.world().resource::<House>();
	assert!(house.graph().opening(id).is_none());
	assert_eq!(house.committed().categories().get(id), None);
	assert_eq!(house.revision(), revision + 1);
	assert_eq!(app.world().resource::<SelectedTarget>().opening(), None);
}

#[test]
fn duplicating_adds_a_copy_with_the_same_category_and_selects_it() {
	let (house, id) = house_with_window(8.0, 3.0);
	let mut app = edit_app(house, Some(id));
	app.world_mut().write_message(DuplicateSelectedOpening);
	app.update();

	let house = app.world().resource::<House>();
	assert_eq!(house.graph().openings().count(), 2);
	let copy = app.world().resource::<SelectedTarget>().opening().unwrap();
	assert_ne!(copy, id);
	assert_eq!(
		house.committed().categories().get(copy),
		Some(OpeningCategory::Window)
	);
	// The original stays where it was, and the copy sits beside it without overlapping.
	assert_eq!(house.graph().opening(id).unwrap().spec.center_distance, 3.0);
	let gap = (house.graph().opening(copy).unwrap().spec.center_distance - 3.0).abs() - 0.9;
	assert!(gap >= wall_graph::MIN_OPENING_GAP - 1e-6);
}

#[test]
fn duplicating_with_no_room_explains_why_and_changes_nothing() {
	let (house, id) = house_with_window(1.4, 0.7);
	let revision = house.revision();
	let mut app = edit_app(house, Some(id));
	app.world_mut().write_message(DuplicateSelectedOpening);
	app.update();

	let house = app.world().resource::<House>();
	assert_eq!(house.revision(), revision);
	assert_eq!(house.graph().openings().count(), 1);
	assert!(
		app.world()
			.resource::<EditNotice>()
			.message
			.as_ref()
			.is_some_and(|message| message.starts_with("Cannot duplicate"))
	);
	assert_eq!(app.world().resource::<SelectedTarget>().opening(), Some(id));
}
