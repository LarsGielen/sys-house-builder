use super::*;
use crate::{
	edit::{PreviewState, Proposal, WallProposal, propose_wall},
	tests::support::editor_app,
};
use wall_graph::WallDimensions;

fn app_with_two_walls() -> App {
	let mut app = editor_app();
	let mut house = House::default();
	for end in [Vec2::new(4.0, 0.0), Vec2::new(0.0, 3.0)] {
		let candidate = propose_wall(
			house.committed(),
			&WallProposal {
				start: Vec2::ZERO,
				end,
				dimensions: WallDimensions::default(),
			},
		)
		.unwrap();
		house.commit(candidate);
	}
	app.insert_resource(house);
	app.update();
	app
}

fn wall_count(app: &App) -> usize {
	app.world().resource::<House>().graph().walls().count()
}

fn press_with_control(app: &mut App, keys: &[KeyCode]) {
	let mut keyboard = ButtonInput::<KeyCode>::default();
	keyboard.press(KeyCode::ControlLeft);
	for &key in keys {
		keyboard.press(key);
	}
	app.insert_resource(keyboard);
}

#[test]
fn requests_undo_and_redo_the_house() {
	let mut app = app_with_two_walls();
	let before = wall_count(&app);

	app.world_mut().write_message(HistoryRequest::Undo);
	app.update();
	assert!(wall_count(&app) < before);

	app.world_mut().write_message(HistoryRequest::Redo);
	app.update();
	assert_eq!(wall_count(&app), before);
}

#[test]
fn an_unavailable_request_is_ignored() {
	let mut app = editor_app();
	app.update();
	app.world_mut().write_message(HistoryRequest::Undo);
	app.world_mut().write_message(HistoryRequest::Redo);
	app.update();
	assert_eq!(app.world().resource::<House>().revision(), 0);
}

#[test]
fn control_z_undoes_and_control_shift_z_or_y_redo() {
	let mut app = app_with_two_walls();
	let before = wall_count(&app);

	press_with_control(&mut app, &[KeyCode::KeyZ]);
	app.update();
	assert!(wall_count(&app) < before);

	press_with_control(&mut app, &[KeyCode::ShiftLeft, KeyCode::KeyZ]);
	app.update();
	assert_eq!(wall_count(&app), before);

	press_with_control(&mut app, &[KeyCode::KeyZ]);
	app.update();
	press_with_control(&mut app, &[KeyCode::KeyY]);
	app.update();
	assert_eq!(wall_count(&app), before);
}

#[test]
fn z_without_control_does_nothing() {
	let mut app = app_with_two_walls();
	let revision = app.world().resource::<House>().revision();
	crate::tests::support::press_key(&mut app, KeyCode::KeyZ);
	app.update();
	assert_eq!(app.world().resource::<House>().revision(), revision);
}

#[test]
fn restoring_discards_the_preview_and_notice() {
	let mut app = app_with_two_walls();
	let (wall, revision, outcome) = {
		let house = app.world().resource::<House>();
		let outcome = propose_wall(
			house.committed(),
			&WallProposal {
				start: Vec2::ZERO,
				end: Vec2::new(-3.0, 0.0),
				dimensions: WallDimensions::default(),
			},
		);
		(
			house.graph().walls().next().unwrap(),
			house.revision(),
			outcome,
		)
	};
	app.world_mut().resource_mut::<EditPreview>().state = Some(PreviewState {
		house_revision: revision,
		proposal: Proposal::WallRemoval(wall),
		outcome,
	});
	app.world_mut().resource_mut::<EditNotice>().message = Some("old".into());

	app.world_mut().write_message(HistoryRequest::Undo);
	app.update();

	assert!(app.world().resource::<EditPreview>().state.is_none());
	assert!(app.world().resource::<EditNotice>().message.is_none());
}
