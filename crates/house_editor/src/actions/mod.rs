pub mod property;
pub mod settings;

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
	actions::property::PropertyAction,
	actions::settings::SettingsAction,
	document::history::HistoryRequest,
	document::project::{ProjectRequest, UnsavedDecision},
	tools::active::ActivateTool,
	tools::edit_opening::{DeleteSelectedOpening, DuplicateSelectedOpening},
	tools::edit_wall::{ConfirmWallDeletion, DeleteSelectedWall},
};

/// Every request to change the editor, in the order it was made. Buttons, shortcuts, field
/// widgets and other hosts all send into this one stream, and each consumer reads it in order
/// and acts on its own kind, so a request made after an undo is never handled before it.
///
/// Gestures (press, drag, release) are not in the stream; they are read from the frame's
/// [`ActionState`](crate::ActionState) and resolved against the current document.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub enum EditorAction {
	ActivateTool(ActivateTool),
	History(HistoryRequest),
	Project(ProjectRequest),
	Unsaved(UnsavedDecision),
	Settings(SettingsAction),
	Property(PropertyAction),
	DeleteSelectedWall(DeleteSelectedWall),
	ConfirmWallDeletion(ConfirmWallDeletion),
	DeleteSelectedOpening(DeleteSelectedOpening),
	DuplicateSelectedOpening(DuplicateSelectedOpening),
}

macro_rules! action_from {
	($($variant:ident($kind:ty)),* $(,)?) => {
		$(impl From<$kind> for EditorAction {
			fn from(kind: $kind) -> Self {
				Self::$variant(kind)
			}
		})*
	};
}

action_from!(
	ActivateTool(ActivateTool),
	History(HistoryRequest),
	Project(ProjectRequest),
	Unsaved(UnsavedDecision),
	Settings(SettingsAction),
	Property(PropertyAction),
	DeleteSelectedWall(DeleteSelectedWall),
	ConfirmWallDeletion(ConfirmWallDeletion),
	DeleteSelectedOpening(DeleteSelectedOpening),
	DuplicateSelectedOpening(DuplicateSelectedOpening),
);

macro_rules! action_accessors {
	($($method:ident -> $kind:ty = $variant:ident),* $(,)?) => {
		impl EditorAction {
			$(pub fn $method(self) -> Option<$kind> {
				match self {
					Self::$variant(kind) => Some(kind),
					_ => None,
				}
			})*
		}
	};
}

action_accessors!(
	activate_tool -> ActivateTool = ActivateTool,
	history -> HistoryRequest = History,
	project -> ProjectRequest = Project,
	unsaved -> UnsavedDecision = Unsaved,
	settings -> SettingsAction = Settings,
	property -> PropertyAction = Property,
);

/// Sends requests into the action stream.
#[derive(SystemParam)]
pub struct ActionSender<'w> {
	writer: MessageWriter<'w, EditorAction>,
}

impl ActionSender<'_> {
	pub fn send(&mut self, action: impl Into<EditorAction>) {
		self.writer.write(action.into());
	}
}

pub struct EditorActionPlugin;

impl EditorActionPlugin {
	/// Plugins that send or read actions call this so each works without the others.
	pub fn ensure(app: &mut App) {
		if !app.is_plugin_added::<Self>() {
			app.add_plugins(Self);
		}
	}
}

impl Plugin for EditorActionPlugin {
	fn build(&self, app: &mut App) {
		app.add_message::<EditorAction>();
	}
}

#[cfg(test)]
#[path = "../tests/actions.rs"]
mod tests;
