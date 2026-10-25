use super::*;

fn path(name: &str) -> PathBuf {
	PathBuf::from(name)
}

#[test]
fn a_clean_document_is_replaced_without_asking() {
	let mut workflow = ProjectWorkflow::default();
	assert_eq!(
		workflow.request(ProjectCommand::New, None, false),
		Some(ProjectEffect::Perform {
			command: ProjectCommand::New,
			open_path: None
		})
	);
	assert_eq!(workflow.pending(), None);
}

#[test]
fn saving_never_asks_and_is_ignored_while_a_question_is_open() {
	let mut workflow = ProjectWorkflow::default();
	assert_eq!(
		workflow.request(ProjectCommand::Save, None, true),
		Some(ProjectEffect::Save)
	);
	workflow.request(ProjectCommand::New, None, true);
	assert_eq!(workflow.request(ProjectCommand::Save, None, true), None);
}

#[test]
fn a_modified_document_asks_first_and_ignores_a_second_request() {
	let mut workflow = ProjectWorkflow::default();
	assert_eq!(workflow.request(ProjectCommand::Close, None, true), None);
	assert_eq!(workflow.pending(), Some(ProjectCommand::Close));
	assert_eq!(workflow.request(ProjectCommand::New, None, true), None);
	assert_eq!(workflow.pending(), Some(ProjectCommand::Close));
}

#[test]
fn an_open_without_a_file_is_refused_before_anything_is_asked() {
	let mut workflow = ProjectWorkflow::default();
	assert_eq!(
		workflow.request(ProjectCommand::Open, None, true),
		Some(ProjectEffect::MissingOpenPath)
	);
	assert_eq!(workflow.pending(), None);
}

#[test]
fn cancel_forgets_the_question_and_discard_performs_the_captured_open() {
	let mut workflow = ProjectWorkflow::default();
	workflow.request(ProjectCommand::Open, Some(path("other")), true);
	assert_eq!(workflow.answer(UnsavedChoice::Cancel, None, None), None);
	assert_eq!(workflow.pending(), None);

	workflow.request(ProjectCommand::Open, Some(path("other")), true);
	assert_eq!(
		workflow.answer(UnsavedChoice::Discard, None, Some(path("typed"))),
		Some(ProjectEffect::Perform {
			command: ProjectCommand::Open,
			open_path: Some(path("other"))
		})
	);
}

#[test]
fn saving_first_writes_to_the_projects_own_file_not_the_one_being_opened() {
	let mut workflow = ProjectWorkflow::default();
	workflow.request(ProjectCommand::Open, Some(path("other")), true);
	assert_eq!(
		workflow.answer(
			UnsavedChoice::Save,
			Some(path("current")),
			Some(path("other"))
		),
		Some(ProjectEffect::SaveThenPerform {
			destination: path("current"),
			command: ProjectCommand::Open,
			open_path: Some(path("other"))
		})
	);
	assert_eq!(workflow.pending(), None);
}

#[test]
fn an_unnamed_project_asks_for_a_path_and_then_uses_the_typed_one() {
	let mut workflow = ProjectWorkflow::default();
	workflow.request(ProjectCommand::Open, Some(path("other")), true);
	assert_eq!(
		workflow.answer(UnsavedChoice::Save, None, Some(path("other"))),
		Some(ProjectEffect::AskForSavePath)
	);
	assert_eq!(workflow.pending(), Some(ProjectCommand::Open));
	assert_eq!(
		workflow.answer(UnsavedChoice::Save, None, Some(path("new"))),
		Some(ProjectEffect::SaveThenPerform {
			destination: path("new"),
			command: ProjectCommand::Open,
			open_path: Some(path("other"))
		})
	);
}

#[test]
fn an_answer_with_no_question_open_does_nothing() {
	let mut workflow = ProjectWorkflow::default();
	assert_eq!(workflow.answer(UnsavedChoice::Discard, None, None), None);
}
