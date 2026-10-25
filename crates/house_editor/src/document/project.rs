use house_document::{open_project, save_project};
use house_editor_core::{
	ProjectCommand, ProjectEffect, ProjectWorkflow, UnsavedChoice, needs_confirmation,
};
use std::path::PathBuf;

use bevy::{
	app::AppExit, ecs::system::SystemParam, prelude::*, text::EditableText,
	window::WindowCloseRequested,
};

use crate::{
	actions::ActionSender,
	actions::{EditorAction, EditorActionPlugin},
	document::edit::{EditCandidate, EditNotice, EditPreview},
	document::history::DocumentRestored,
	document::house::House,
	input::{Action, ActionState, ActionsPlugin},
	tools::selection::SelectedTarget,
};

/// Starts, opens, and saves projects, and asks what to do with unsaved changes first.
pub struct ProjectPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProjectSystems;

impl Plugin for ProjectPlugin {
	fn build(&self, app: &mut App) {
		EditorActionPlugin::ensure(app);
		ActionsPlugin::ensure(app);
		app.init_resource::<ProjectPath>()
			.init_resource::<ProjectStatus>()
			.init_resource::<PendingReplacement>()
			.init_resource::<SelectedTarget>()
			.init_resource::<EditPreview>()
			.init_resource::<EditNotice>()
			.add_message::<WindowCloseRequested>()
			.add_message::<AppExit>()
			.add_message::<DocumentRestored>()
			.add_systems(
				PostUpdate,
				(request_close_on_window_close, handle_project_requests)
					.chain()
					.in_set(ProjectSystems),
			);
	}
}

/// What the user asked to do with the project. Opening and saving use the path typed in the
/// path field, or the project's own path when the field is empty.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectRequest {
	New,
	Open,
	Save,
	/// Leaving the editor.
	Close,
}

/// The file the current document was opened from or last saved to.
#[derive(Resource, Default)]
pub struct ProjectPath(pub Option<PathBuf>);

/// The outcome of the last file action, including why it failed.
#[derive(Resource, Default)]
pub struct ProjectStatus(pub String);

/// A replacement or close waiting for the answer to the unsaved-changes question. The current
/// document, selection, and history stay in place until it is answered.
///
/// While this holds a request the editor is modal: [`crate::input`] produces no actions.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct PendingReplacement(ProjectWorkflow);

impl PendingReplacement {
	pub fn pending_request(&self) -> Option<ProjectRequest> {
		self.pending().map(ProjectRequest::from)
	}
}

impl From<ProjectRequest> for ProjectCommand {
	fn from(request: ProjectRequest) -> Self {
		match request {
			ProjectRequest::New => Self::New,
			ProjectRequest::Open => Self::Open,
			ProjectRequest::Save => Self::Save,
			ProjectRequest::Close => Self::Close,
		}
	}
}

impl From<ProjectCommand> for ProjectRequest {
	fn from(command: ProjectCommand) -> Self {
		match command {
			ProjectCommand::New => Self::New,
			ProjectCommand::Open => Self::Open,
			ProjectCommand::Save => Self::Save,
			ProjectCommand::Close => Self::Close,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnsavedDecision(pub UnsavedChoice);

/// Marks the text field that holds the project's file path.
#[derive(Component)]
pub struct PathField;

#[derive(SystemParam)]
struct Project<'w, 's> {
	house: ResMut<'w, House>,
	path: ResMut<'w, ProjectPath>,
	status: ResMut<'w, ProjectStatus>,
	selected: ResMut<'w, SelectedTarget>,
	preview: ResMut<'w, EditPreview>,
	notice: ResMut<'w, EditNotice>,
	restored: MessageWriter<'w, DocumentRestored>,
	exit: MessageWriter<'w, AppExit>,
	path_field: Query<'w, 's, &'static mut EditableText, With<PathField>>,
}

impl Project<'_, '_> {
	fn typed_path(&self) -> Option<PathBuf> {
		self.path_field
			.iter()
			.map(|text| text.value().to_string())
			.map(|text| text.trim().to_string())
			.find(|text| !text.is_empty())
			.map(PathBuf::from)
	}

	/// The typed path, or else the project's own.
	fn target_path(&self) -> Option<PathBuf> {
		self.typed_path().or_else(|| self.path.0.clone())
	}

	fn clear_path_field(&mut self) {
		for mut text in &mut self.path_field {
			text.editor_mut().set_text("");
		}
	}

	fn save_to(&mut self, path: PathBuf) -> bool {
		match save_project(&path, self.house.committed()) {
			Ok(()) => {
				self.house.mark_saved();
				self.status.0 = format!("Saved {}", path.display());
				self.path.0 = Some(path);
				true
			}
			// The document stays modified, so the failure cannot be mistaken for a save.
			Err(error) => {
				self.status.0 = error.to_string();
				false
			}
		}
	}

	fn replace_document(&mut self, document: EditCandidate, path: Option<PathBuf>) {
		self.house.replace_document(document);
		self.path.0 = path;
		self.selected.0 = None;
		self.preview.state = None;
		self.notice.message = None;
		self.restored.write(DocumentRestored);
	}

	fn handle_request(&mut self, pending: &mut PendingReplacement, request: ProjectRequest) {
		let effect = pending.request(
			request.into(),
			self.target_path(),
			needs_confirmation(&self.house),
		);
		if let Some(effect) = effect {
			self.execute(effect);
		}
	}

	fn handle_answer(&mut self, pending: &mut PendingReplacement, choice: UnsavedChoice) {
		let effect = pending.answer(choice, self.path.0.clone(), self.typed_path());
		if let Some(effect) = effect {
			self.execute(effect);
		}
	}

	fn execute(&mut self, effect: ProjectEffect) {
		match effect {
			ProjectEffect::Save => match self.target_path() {
				Some(path) => {
					self.save_to(path);
				}
				None => self.status.0 = "Type a file path to save to".to_string(),
			},
			ProjectEffect::Perform { command, open_path } => self.perform(command, open_path),
			// A failed save must not be followed by the replacement it was meant to precede.
			ProjectEffect::SaveThenPerform {
				destination,
				command,
				open_path,
			} => {
				if self.save_to(destination) {
					self.perform(command, open_path);
				}
			}
			ProjectEffect::AskForSavePath => {
				self.clear_path_field();
				self.status.0 =
					"This project has no file yet: type where to save it, then choose Save"
						.to_string();
			}
			ProjectEffect::MissingOpenPath => {
				self.status.0 = "Type a file path to open".to_string();
			}
		}
	}

	fn perform(&mut self, command: ProjectCommand, open_path: Option<PathBuf>) {
		match command {
			ProjectCommand::New => {
				self.replace_document(EditCandidate::default(), None);
				self.status.0 = "Started a new project".to_string();
			}
			ProjectCommand::Open => {
				let Some(path) = open_path else {
					return;
				};
				// A rejected file leaves the document, its history, and its modified state alone.
				match open_project(&path) {
					Ok(document) => {
						self.status.0 = format!("Opened {}", path.display());
						self.replace_document(document, Some(path));
					}
					Err(error) => self.status.0 = error.to_string(),
				}
			}
			ProjectCommand::Close => {
				self.exit.write(AppExit::Success);
			}
			ProjectCommand::Save => unreachable!("saving is never deferred"),
		}
	}
}

fn request_close_on_window_close(
	mut closes: MessageReader<WindowCloseRequested>,
	mut requests: ActionSender,
) {
	if closes.read().count() > 0 {
		requests.send(ProjectRequest::Close);
	}
}

/// Handles requests and answers in the order they were made, so an answer to a question
/// cannot overtake the request that raised it.
fn handle_project_requests(
	mut stream: MessageReader<EditorAction>,
	actions: Res<ActionState>,
	mut pending: ResMut<PendingReplacement>,
	mut project: Project,
) {
	for action in stream.read().copied() {
		match action {
			EditorAction::Project(request) => project.handle_request(&mut pending, request),
			EditorAction::Unsaved(UnsavedDecision(choice)) => {
				project.handle_answer(&mut pending, choice);
			}
			_ => {}
		}
	}

	// Shortcuts are this frame's device input, after anything already in the stream.
	if actions.just_pressed(Action::Save) {
		project.handle_request(&mut pending, ProjectRequest::Save);
	} else if actions.just_pressed(Action::Open) {
		project.handle_request(&mut pending, ProjectRequest::Open);
	}
}

#[cfg(test)]
#[path = "../tests/document/project.rs"]
mod tests;
