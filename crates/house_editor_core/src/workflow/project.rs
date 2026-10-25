use std::path::PathBuf;

use crate::workflow::unsaved_changes::{ReplacementStep, UnsavedChoice};

/// What the user asked to do with the project.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectCommand {
	New,
	Open,
	Save,
	/// Leaving the editor.
	Close,
}

/// What the host must do next. The workflow decides; the host owns files, widgets and exit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectEffect {
	/// Save to the typed path, or else the project's own.
	Save,
	/// Carry out a command now. `open_path` is the file an `Open` asked for.
	Perform {
		command: ProjectCommand,
		open_path: Option<PathBuf>,
	},
	/// Save the open document to `destination`, then carry out the command only if that
	/// succeeded.
	SaveThenPerform {
		destination: PathBuf,
		command: ProjectCommand,
		open_path: Option<PathBuf>,
	},
	/// An unnamed project must be told where to save. The host clears the path field, which
	/// until now named the file to open, and the user answers again with a typed path.
	AskForSavePath,
	/// An open was asked for with no file named.
	MissingOpenPath,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Pending {
	command: ProjectCommand,
	/// Captured with the request, so answering never re-reads a path field that may by then
	/// name the file to save.
	open_path: Option<PathBuf>,
	awaiting_save_path: bool,
}

/// The unsaved-changes question around replacing or closing a document. While a question is
/// open the editor is modal: a second request is ignored and the host should accept no other
/// input.
#[derive(Default, Debug)]
pub struct ProjectWorkflow {
	pending: Option<Pending>,
}

impl ProjectWorkflow {
	/// The command waiting for an answer.
	pub fn pending(&self) -> Option<ProjectCommand> {
		self.pending.as_ref().map(|pending| pending.command)
	}

	/// A command arrives. `open_path` is where an `Open` would read from, and `modified` says
	/// whether replacing the document would lose changes.
	pub fn request(
		&mut self,
		command: ProjectCommand,
		open_path: Option<PathBuf>,
		modified: bool,
	) -> Option<ProjectEffect> {
		if self.pending.is_some() {
			return None;
		}
		if command == ProjectCommand::Save {
			return Some(ProjectEffect::Save);
		}
		let open_path = match command {
			ProjectCommand::Open => match open_path {
				Some(path) => Some(path),
				None => return Some(ProjectEffect::MissingOpenPath),
			},
			_ => None,
		};
		if modified {
			self.pending = Some(Pending {
				command,
				open_path,
				awaiting_save_path: false,
			});
			None
		} else {
			Some(ProjectEffect::Perform { command, open_path })
		}
	}

	/// The user answered the open question. `project_path` is the document's own file, and
	/// `typed_path` what the path field holds now.
	pub fn answer(
		&mut self,
		choice: UnsavedChoice,
		project_path: Option<PathBuf>,
		typed_path: Option<PathBuf>,
	) -> Option<ProjectEffect> {
		let pending = self.pending.as_mut()?;
		match choice.step() {
			ReplacementStep::Keep => {
				self.pending = None;
				None
			}
			ReplacementStep::Replace => {
				let Pending {
					command, open_path, ..
				} = self.pending.take()?;
				Some(ProjectEffect::Perform { command, open_path })
			}
			ReplacementStep::SaveThenReplace => {
				// The path field names the file to open until the project is asked where to
				// save, so it cannot decide the destination before then.
				let destination =
					project_path.or(typed_path.filter(|_| pending.awaiting_save_path));
				let Some(destination) = destination else {
					pending.awaiting_save_path = true;
					return Some(ProjectEffect::AskForSavePath);
				};
				let Pending {
					command, open_path, ..
				} = self.pending.take()?;
				Some(ProjectEffect::SaveThenPerform {
					destination,
					command,
					open_path,
				})
			}
		}
	}
}

#[cfg(test)]
#[path = "../tests/workflow/project.rs"]
mod tests;
