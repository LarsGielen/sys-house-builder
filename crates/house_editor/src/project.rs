use std::path::PathBuf;

use bevy::{
	app::AppExit, ecs::system::SystemParam, prelude::*, text::EditableText,
	window::WindowCloseRequested,
};

use crate::{
	edit::{EditCandidate, EditNotice, EditPreview},
	history::DocumentRestored,
	house::House,
	project_io::{open_project, save_project},
	text_fields::TextFocus,
	unsaved_changes::{ReplacementStep, UnsavedChoice, needs_confirmation},
	wall_tool_selection::SelectedTarget,
};

/// Starts, opens, and saves projects, and asks what to do with unsaved changes first.
pub struct ProjectPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProjectSystems;

impl Plugin for ProjectPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<ProjectPath>()
			.init_resource::<ProjectStatus>()
			.init_resource::<PendingReplacement>()
			.init_resource::<SelectedTarget>()
			.init_resource::<EditPreview>()
			.init_resource::<EditNotice>()
			.add_message::<ProjectRequest>()
			.add_message::<WindowCloseRequested>()
			.add_message::<AppExit>()
			.add_message::<UnsavedDecision>()
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
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
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
#[derive(Resource, Default)]
pub struct PendingReplacement(pub Option<ProjectRequest>);

#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
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
	path_field: Query<'w, 's, &'static EditableText, With<PathField>>,
}

impl Project<'_, '_> {
	/// The typed path, or else the project's own.
	fn target_path(&self) -> Option<PathBuf> {
		self.path_field
			.iter()
			.map(|text| text.value().to_string())
			.map(|text| text.trim().to_string())
			.find(|text| !text.is_empty())
			.map(PathBuf::from)
			.or_else(|| self.path.0.clone())
	}

	/// Returns whether the document is now saved.
	fn save(&mut self) -> bool {
		let Some(path) = self.target_path() else {
			self.status.0 = "Type a file path to save to".to_string();
			return false;
		};
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

	fn perform(&mut self, request: ProjectRequest) {
		match request {
			ProjectRequest::New => {
				self.replace_document(EditCandidate::default(), None);
				self.status.0 = "Started a new project".to_string();
			}
			ProjectRequest::Open => {
				let Some(path) = self.target_path() else {
					self.status.0 = "Type a file path to open".to_string();
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
			ProjectRequest::Close => {
				self.exit.write(AppExit::Success);
			}
			ProjectRequest::Save => {
				self.save();
			}
		}
	}
}

fn request_close_on_window_close(
	mut closes: MessageReader<WindowCloseRequested>,
	mut requests: MessageWriter<ProjectRequest>,
) {
	if closes.read().count() > 0 {
		requests.write(ProjectRequest::Close);
	}
}

fn handle_project_requests(
	mut requests: MessageReader<ProjectRequest>,
	mut decisions: MessageReader<UnsavedDecision>,
	keyboard: Res<ButtonInput<KeyCode>>,
	typing: TextFocus,
	mut pending: ResMut<PendingReplacement>,
	mut project: Project,
) {
	let control = keyboard.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
	let shortcut = if control && !typing.active() {
		if keyboard.just_pressed(KeyCode::KeyS) {
			Some(ProjectRequest::Save)
		} else if keyboard.just_pressed(KeyCode::KeyO) {
			Some(ProjectRequest::Open)
		} else {
			None
		}
	} else {
		None
	};

	for request in requests.read().copied().chain(shortcut) {
		match request {
			ProjectRequest::Save => {
				project.save();
			}
			// Another question is already open; it must be answered first.
			_ if pending.0.is_some() => {}
			_ if needs_confirmation(&project.house) => pending.0 = Some(request),
			_ => project.perform(request),
		}
	}

	for &UnsavedDecision(choice) in decisions.read() {
		let Some(request) = pending.0.take() else {
			continue;
		};
		match choice.step() {
			ReplacementStep::Keep => {}
			ReplacementStep::Replace => project.perform(request),
			// A failed save must not be followed by the replacement it was meant to precede.
			ReplacementStep::SaveThenReplace => {
				if project.save() {
					project.perform(request);
				}
			}
		}
	}
}

#[cfg(test)]
#[path = "tests/project.rs"]
mod tests;
