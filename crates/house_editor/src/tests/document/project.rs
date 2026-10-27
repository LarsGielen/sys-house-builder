use super::*;
use crate::{
	document::edit::{WallProposal, propose_wall},
	editor_schedule::EditorSchedulePlugin,
	tools::selection::SelectionTarget,
};
use house_document::HouseLibrary;
use std::fs;

/// A directory for one test's files, removed afterwards.
struct TestDirectory(PathBuf);

impl TestDirectory {
	fn new(name: &str) -> Self {
		let path = std::env::temp_dir().join(format!(
			"house_editor_project_{name}_{}",
			std::process::id()
		));
		let _ = fs::remove_dir_all(&path);
		fs::create_dir_all(&path).unwrap();
		Self(path)
	}

	fn path(&self, file: &str) -> PathBuf {
		self.0.join(file)
	}
}

impl Drop for TestDirectory {
	fn drop(&mut self) {
		let _ = fs::remove_dir_all(&self.0);
	}
}

fn house_with_walls(count: usize) -> House {
	let mut house = House::default();
	for index in 0..count {
		let y = index as f32 * 3.0;
		let candidate = propose_wall(
			house.committed(),
			&WallProposal {
				start: Vec2::new(0.0, y),
				end: Vec2::new(4.0, y),
				dimensions: Default::default(),
			},
		)
		.unwrap();
		house.commit(candidate);
	}
	house
}

fn project_app(house: House) -> App {
	let mut app = App::new();
	app.add_plugins((EditorSchedulePlugin, ProjectPlugin))
		.insert_resource(house)
		.init_resource::<ButtonInput<KeyCode>>();
	app
}

fn send(app: &mut App, request: ProjectRequest) {
	app.world_mut()
		.write_message(crate::actions::EditorAction::from(request));
	app.update();
}

fn decide(app: &mut App, choice: UnsavedChoice) {
	app.world_mut()
		.write_message(crate::actions::EditorAction::from(UnsavedDecision(choice)));
	app.update();
}

fn wall_count(app: &App) -> usize {
	app.world().resource::<House>().graph().walls().count()
}

fn status(app: &App) -> String {
	app.world().resource::<ProjectStatus>().0.clone()
}

fn pending(app: &App) -> Option<ProjectRequest> {
	app.world()
		.resource::<PendingReplacement>()
		.pending_request()
}

fn open_house(app: &mut App, path: &std::path::Path) {
	app.world_mut().write_message(OpenHouse(path.to_path_buf()));
	app.update();
}

fn library_app(house: House, directory: &TestDirectory) -> App {
	let mut app = project_app(house);
	app.insert_resource(Houses(Some(HouseLibrary::new(directory.path("houses")))));
	app
}

/// A saved project with `count` walls, to open from another app.
fn saved_project(directory: &TestDirectory, name: &str, count: usize) -> PathBuf {
	let path = directory.path(name);
	save_project(&path, house_with_walls(count).committed()).unwrap();
	path
}

#[test]
fn saving_writes_the_file_and_marks_the_document_clean() {
	let directory = TestDirectory::new("save");
	let path = directory.path("a.project");
	let mut app = project_app(house_with_walls(2));
	app.world_mut().resource_mut::<ProjectPath>().0 = Some(path.clone());
	assert!(app.world().resource::<House>().is_modified());

	send(&mut app, ProjectRequest::Save);

	assert!(!app.world().resource::<House>().is_modified());
	assert!(fs::read_to_string(&path).unwrap().contains("wall w2"));
	assert_eq!(
		app.world().resource::<ProjectPath>().0.as_deref(),
		Some(path.as_path())
	);
	assert!(status(&app).starts_with("Saved"));
	// Saving is not an edit: history stays.
	assert!(app.world().resource::<House>().can_undo());
}

#[test]
fn a_failed_save_keeps_the_document_modified_and_says_why() {
	let directory = TestDirectory::new("failed_save");
	let path = directory.path("a.project");
	fs::create_dir(&path).unwrap();
	let mut app = project_app(house_with_walls(1));
	app.world_mut().resource_mut::<ProjectPath>().0 = Some(path.clone());

	send(&mut app, ProjectRequest::Save);

	assert!(app.world().resource::<House>().is_modified());
	assert!(status(&app).starts_with("Cannot save"), "{}", status(&app));
	assert!(path.is_dir());
}

#[test]
fn saving_without_an_open_house_says_so_and_changes_nothing() {
	let mut app = project_app(house_with_walls(1));
	send(&mut app, ProjectRequest::Save);
	assert!(app.world().resource::<House>().is_modified());
	assert!(status(&app).contains("No house"));
}

#[test]
fn control_s_saves_to_the_project_path() {
	let directory = TestDirectory::new("shortcut");
	let path = directory.path("a.project");
	let mut app = project_app(house_with_walls(1));
	app.world_mut().resource_mut::<ProjectPath>().0 = Some(path.clone());
	let mut keyboard = ButtonInput::<KeyCode>::default();
	keyboard.press(KeyCode::ControlLeft);
	keyboard.press(KeyCode::KeyS);
	app.insert_resource(keyboard);
	app.update();
	assert!(path.is_file());
	assert!(!app.world().resource::<House>().is_modified());
}

#[test]
fn opening_replaces_the_document_history_selection_and_modified_state() {
	let directory = TestDirectory::new("open");
	let path = saved_project(&directory, "other.project", 3);
	let mut app = project_app(house_with_walls(1));
	app.update();
	let wall = app
		.world()
		.resource::<House>()
		.graph()
		.walls()
		.next()
		.unwrap();
	app.insert_resource(SelectedTarget(Some(SelectionTarget::Wall(wall))));
	// Saved first so that opening needs no question.
	app.world_mut().resource_mut::<House>().mark_saved();

	open_house(&mut app, &path);

	assert_eq!(wall_count(&app), 3);
	let house = app.world().resource::<House>();
	assert!(!house.is_modified());
	assert!(!house.can_undo() && !house.can_redo());
	assert_eq!(app.world().resource::<SelectedTarget>().0, None);
	assert_eq!(
		app.world().resource::<ProjectPath>().0.as_deref(),
		Some(path.as_path())
	);
}

#[test]
fn a_rejected_import_preserves_geometry_history_and_modified_state() {
	let directory = TestDirectory::new("rejected");
	let broken = directory.path("broken.project");
	fs::write(
		&broken,
		"house-editor-project 1\nwall a: from 0 0 to nope 0 height 2 thickness 1\n",
	)
	.unwrap();
	let mut app = project_app(house_with_walls(2));
	app.world_mut().resource_mut::<House>().mark_saved();
	app.world_mut().resource_mut::<House>().undo();
	app.world_mut().resource_mut::<House>().redo();
	let revision = app.world().resource::<House>().revision();
	let modified = app.world().resource::<House>().is_modified();

	open_house(&mut app, &broken);

	let house = app.world().resource::<House>();
	assert_eq!(house.revision(), revision);
	assert_eq!(house.is_modified(), modified);
	assert!(house.can_undo());
	assert_eq!(wall_count(&app), 2);
	assert!(status(&app).contains("line 2"), "{}", status(&app));
}

#[test]
fn opening_a_modified_project_asks_first_and_cancel_keeps_everything() {
	let directory = TestDirectory::new("cancel");
	let path = saved_project(&directory, "other.project", 3);
	let mut app = project_app(house_with_walls(2));
	app.update();
	let wall = app
		.world()
		.resource::<House>()
		.graph()
		.walls()
		.next()
		.unwrap();
	app.insert_resource(SelectedTarget(Some(SelectionTarget::Wall(wall))));
	let revision = app.world().resource::<House>().revision();

	open_house(&mut app, &path);
	assert_eq!(pending(&app), Some(ProjectRequest::Open));
	assert_eq!(wall_count(&app), 2, "nothing is replaced before the answer");

	decide(&mut app, UnsavedChoice::Cancel);

	assert_eq!(pending(&app), None);
	let house = app.world().resource::<House>();
	assert_eq!(house.revision(), revision);
	assert!(house.is_modified());
	assert!(house.can_undo());
	assert_eq!(wall_count(&app), 2);
	assert_eq!(app.world().resource::<SelectedTarget>().wall(), Some(wall));
}

#[test]
fn discarding_opens_the_other_project() {
	let directory = TestDirectory::new("discard");
	let path = saved_project(&directory, "other.project", 3);
	let mut app = project_app(house_with_walls(2));
	open_house(&mut app, &path);
	decide(&mut app, UnsavedChoice::Discard);
	assert_eq!(wall_count(&app), 3);
	assert_eq!(pending(&app), None);
	assert!(!app.world().resource::<House>().is_modified());
}

#[test]
fn saving_first_writes_the_old_project_then_opens_the_other() {
	let directory = TestDirectory::new("save_then_open");
	let other = saved_project(&directory, "other.project", 3);
	let current = directory.path("current.project");
	let mut app = project_app(house_with_walls(2));
	app.world_mut().resource_mut::<ProjectPath>().0 = Some(current.clone());
	open_house(&mut app, &other);

	decide(&mut app, UnsavedChoice::Save);

	assert!(fs::read_to_string(&current).unwrap().contains("wall w2"));
	assert_eq!(wall_count(&app), 3, "the requested file is the one opened");
	assert_eq!(
		app.world().resource::<ProjectPath>().0.as_deref(),
		Some(other.as_path())
	);
	assert_eq!(pending(&app), None);
}

#[test]
fn a_failed_save_does_not_replace_the_document() {
	let directory = TestDirectory::new("save_fails");
	let blocked = directory.path("blocked.project");
	fs::create_dir(&blocked).unwrap();
	let mut app = project_app(house_with_walls(2));
	app.world_mut().resource_mut::<ProjectPath>().0 = Some(blocked);
	send(&mut app, ProjectRequest::New);
	assert_eq!(pending(&app), Some(ProjectRequest::New));

	decide(&mut app, UnsavedChoice::Save);

	assert_eq!(wall_count(&app), 2);
	assert!(app.world().resource::<House>().is_modified());
	assert_eq!(pending(&app), None);
	assert!(status(&app).starts_with("Cannot save"));
}

#[test]
fn a_new_house_is_saved_to_the_library_and_opened_empty_and_clean_after_the_choice() {
	let directory = TestDirectory::new("new_house");
	let mut app = library_app(house_with_walls(2), &directory);
	send(&mut app, ProjectRequest::New);
	assert_eq!(wall_count(&app), 2);
	decide(&mut app, UnsavedChoice::Discard);
	assert_eq!(wall_count(&app), 0);
	let house = app.world().resource::<House>();
	assert!(!house.is_modified() && !house.can_undo());
	let path = app.world().resource::<ProjectPath>().0.clone().unwrap();
	assert!(path.is_file());
	assert_eq!(path.file_stem().unwrap(), "House 1");
}

#[test]
fn a_new_house_needs_somewhere_to_keep_it() {
	let mut app = project_app(House::default());
	send(&mut app, ProjectRequest::New);
	assert_eq!(app.world().resource::<ProjectPath>().0, None);
	assert!(status(&app).contains("nowhere"));
}

#[test]
fn a_clean_document_is_replaced_without_asking() {
	let mut app = project_app(House::default());
	send(&mut app, ProjectRequest::New);
	assert_eq!(pending(&app), None);
}

#[test]
fn closing_a_modified_project_asks_and_a_clean_one_exits() {
	let mut app = project_app(house_with_walls(1));
	send(&mut app, ProjectRequest::Close);
	assert_eq!(pending(&app), Some(ProjectRequest::Close));
	assert_eq!(
		app.world_mut()
			.resource_mut::<Messages<AppExit>>()
			.drain()
			.count(),
		0
	);

	decide(&mut app, UnsavedChoice::Cancel);
	assert_eq!(pending(&app), None);

	send(&mut app, ProjectRequest::Close);
	decide(&mut app, UnsavedChoice::Discard);
	assert_eq!(
		app.world_mut()
			.resource_mut::<Messages<AppExit>>()
			.drain()
			.count(),
		1
	);

	let mut clean = project_app(House::default());
	send(&mut clean, ProjectRequest::Close);
	assert_eq!(
		clean
			.world_mut()
			.resource_mut::<Messages<AppExit>>()
			.drain()
			.count(),
		1
	);
}

#[test]
fn a_second_request_while_asking_is_ignored() {
	let mut app = project_app(house_with_walls(1));
	send(&mut app, ProjectRequest::Close);
	send(&mut app, ProjectRequest::New);
	assert_eq!(pending(&app), Some(ProjectRequest::Close));
}

#[test]
fn undo_history_after_opening_belongs_to_the_new_document() {
	let directory = TestDirectory::new("history");
	let path = saved_project(&directory, "other.project", 1);
	let mut app = project_app(House::default());
	open_house(&mut app, &path);
	let house = app.world().resource::<House>();
	assert_eq!(house.graph().walls().count(), 1);
	assert!(!house.can_undo());
}

#[test]
fn saving_first_does_not_write_over_the_file_that_was_asked_for() {
	let directory = TestDirectory::new("save_then_open_keeps_other");
	let other = saved_project(&directory, "other.project", 3);
	let before = fs::read_to_string(&other).unwrap();
	let mut app = project_app(house_with_walls(2));
	app.world_mut().resource_mut::<ProjectPath>().0 = Some(directory.path("current.project"));
	open_house(&mut app, &other);
	decide(&mut app, UnsavedChoice::Save);

	assert_eq!(fs::read_to_string(&other).unwrap(), before);
}

#[test]
fn saving_first_without_a_file_keeps_the_question_open() {
	let directory = TestDirectory::new("unnamed_save_then_open");
	let other = saved_project(&directory, "other.project", 3);
	let before = fs::read_to_string(&other).unwrap();
	let mut app = project_app(house_with_walls(2));
	open_house(&mut app, &other);
	decide(&mut app, UnsavedChoice::Save);

	// Nothing was written or replaced, and the question stays open to discard or cancel.
	assert_eq!(fs::read_to_string(&other).unwrap(), before);
	assert_eq!(wall_count(&app), 2);
	assert_eq!(pending(&app), Some(ProjectRequest::Open));
	assert!(status(&app).contains("No house"));
}

#[test]
fn an_open_question_makes_the_editor_modal() {
	let mut app = project_app(house_with_walls(2));
	app.world_mut().resource_mut::<ProjectPath>().0 =
		Some(std::env::temp_dir().join("modal.project"));
	send(&mut app, ProjectRequest::New);
	assert_eq!(pending(&app), Some(ProjectRequest::New));

	let mut keyboard = ButtonInput::<KeyCode>::default();
	keyboard.press(KeyCode::KeyZ);
	keyboard.press(KeyCode::ControlLeft);
	app.insert_resource(keyboard);
	app.update();
	assert!(
		!app.world()
			.resource::<crate::input::ActionState>()
			.just_pressed(crate::input::Action::Undo)
	);
}

#[test]
fn an_answer_sent_before_its_question_is_ignored() {
	use crate::actions::EditorAction;

	let mut app = project_app(house_with_walls(2));
	app.world_mut()
		.write_message(EditorAction::from(UnsavedDecision(UnsavedChoice::Discard)));
	app.world_mut()
		.write_message(EditorAction::from(ProjectRequest::New));
	app.update();

	// Handled in the order sent, the answer finds no open question and the request then asks.
	assert_eq!(pending(&app), Some(ProjectRequest::New));
	assert_eq!(wall_count(&app), 2);
}

#[test]
fn a_question_and_its_answer_in_one_frame_are_handled_in_order() {
	use crate::actions::EditorAction;

	let directory = TestDirectory::new("one_frame");
	let mut app = library_app(house_with_walls(2), &directory);
	app.world_mut()
		.write_message(EditorAction::from(ProjectRequest::New));
	app.world_mut()
		.write_message(EditorAction::from(UnsavedDecision(UnsavedChoice::Discard)));
	app.update();

	assert_eq!(pending(&app), None);
	assert_eq!(wall_count(&app), 0);
}
