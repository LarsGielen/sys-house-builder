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
	/// Save to the project's own file.
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
	/// A save was chosen but the open document has no file. The question stays open, so the
	/// user can still discard or cancel.
	MissingSavePath,
	/// An open was asked for with no file named.
	MissingOpenPath,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Pending {
	command: ProjectCommand,
	/// Captured with the request, so answering does not depend on what is selected by then.
	open_path: Option<PathBuf>,
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
			self.pending = Some(Pending { command, open_path });
			None
		} else {
			Some(ProjectEffect::Perform { command, open_path })
		}
	}

	/// The user answered the open question. `project_path` is the document's own file.
	pub fn answer(
		&mut self,
		choice: UnsavedChoice,
		project_path: Option<PathBuf>,
	) -> Option<ProjectEffect> {
		self.pending.as_ref()?;
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
				let Some(destination) = project_path else {
					return Some(ProjectEffect::MissingSavePath);
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
