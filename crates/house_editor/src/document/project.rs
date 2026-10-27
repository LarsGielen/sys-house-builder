use house_document::{HouseLibrary, open_project, save_project};
use house_editor_core::{
	ProjectCommand, ProjectEffect, ProjectWorkflow, UnsavedChoice, needs_confirmation,
};
use std::{env, path::PathBuf};

use bevy::{app::AppExit, ecs::system::SystemParam, prelude::*, window::WindowCloseRequested};

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
			.init_resource::<Houses>()
			.init_resource::<ProjectStatus>()
			.init_resource::<PendingReplacement>()
			.init_resource::<SelectedTarget>()
			.init_resource::<EditPreview>()
			.init_resource::<EditNotice>()
			.add_message::<OpenHouse>()
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

/// What the user asked to do with the project. Saving writes the open house's own file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectRequest {
	/// Creates a house in the library and opens it.
	New,
	/// Only meaningful with a house to open, which arrives as [`OpenHouse`].
	Open,
	Save,
	/// Leaving the editor.
	Close,
}

/// The file the current document was opened from or last saved to.
#[derive(Resource, Default)]
pub struct ProjectPath(pub Option<PathBuf>);

/// Opens the house stored in this file, after asking about unsaved changes.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct OpenHouse(pub PathBuf);

/// The folder of saved houses. `None` when the platform's data directory cannot be
/// determined, which leaves nowhere to keep houses.
#[derive(Resource, Default)]
pub struct Houses(pub Option<HouseLibrary>);

impl Houses {
	/// `houses` in the application data directory.
	pub fn in_data_directory() -> Self {
		Self(data_directory().map(|directory| HouseLibrary::new(directory.join("houses"))))
	}
}

#[cfg(windows)]
fn data_directory() -> Option<PathBuf> {
	env::var_os("APPDATA").map(|base| PathBuf::from(base).join("house_editor"))
}

/// Follows the XDG convention: `$XDG_DATA_HOME`, else `~/.local/share`. The specification says
/// to ignore a relative `$XDG_DATA_HOME`.
#[cfg(not(windows))]
fn data_directory() -> Option<PathBuf> {
	let base = env::var_os("XDG_DATA_HOME")
		.map(PathBuf::from)
		.filter(|path| path.is_absolute())
		.or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))?;
	Some(base.join("house_editor"))
}

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

#[derive(SystemParam)]
struct Project<'w> {
	house: ResMut<'w, House>,
	path: ResMut<'w, ProjectPath>,
	houses: Res<'w, Houses>,
	status: ResMut<'w, ProjectStatus>,
	selected: ResMut<'w, SelectedTarget>,
	preview: ResMut<'w, EditPreview>,
	notice: ResMut<'w, EditNotice>,
	restored: MessageWriter<'w, DocumentRestored>,
	exit: MessageWriter<'w, AppExit>,
}

impl Project<'_> {
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

	fn handle_request(
		&mut self,
		pending: &mut PendingReplacement,
		request: ProjectRequest,
		open_path: Option<PathBuf>,
	) {
		let effect = pending.request(request.into(), open_path, needs_confirmation(&self.house));
		if let Some(effect) = effect {
			self.execute(effect);
		}
	}

	fn handle_answer(&mut self, pending: &mut PendingReplacement, choice: UnsavedChoice) {
		let effect = pending.answer(choice, self.path.0.clone());
		if let Some(effect) = effect {
			self.execute(effect);
		}
	}

	fn execute(&mut self, effect: ProjectEffect) {
		match effect {
			ProjectEffect::Save => match self.path.0.clone() {
				Some(path) => {
					self.save_to(path);
				}
				None => self.status.0 = "No house is open to save".to_string(),
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
			ProjectEffect::MissingSavePath => {
				self.status.0 = "No house is open to save".to_string();
			}
			ProjectEffect::MissingOpenPath => {
				self.status.0 = "Choose a house to open".to_string();
			}
		}
	}

	/// A new house is saved at once, so it is in the library and has a file to save to.
	fn create_house(&mut self) {
		let Some(library) = &self.houses.0 else {
			self.status.0 = "There is nowhere to keep houses".to_string();
			return;
		};
		match library.create_empty() {
			Ok(entry) => {
				self.status.0 = format!("Created {}", entry.name);
				self.replace_document(EditCandidate::default(), Some(entry.path));
			}
			Err(error) => self.status.0 = error.to_string(),
		}
	}

	fn perform(&mut self, command: ProjectCommand, open_path: Option<PathBuf>) {
		match command {
			ProjectCommand::New => self.create_house(),
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
	mut open_requests: MessageReader<OpenHouse>,
	actions: Res<ActionState>,
	mut pending: ResMut<PendingReplacement>,
	mut project: Project,
) {
	for action in stream.read().copied() {
		match action {
			EditorAction::Project(request) => project.handle_request(&mut pending, request, None),
			EditorAction::Unsaved(UnsavedDecision(choice)) => {
				project.handle_answer(&mut pending, choice);
			}
			_ => {}
		}
	}
	for OpenHouse(path) in open_requests.read() {
		project.handle_request(&mut pending, ProjectRequest::Open, Some(path.clone()));
	}

	// Shortcuts are this frame's device input, after anything already in the stream.
	if actions.just_pressed(Action::Save) {
		project.handle_request(&mut pending, ProjectRequest::Save, None);
	}
}

#[cfg(test)]
#[path = "../tests/document/project.rs"]
mod tests;
