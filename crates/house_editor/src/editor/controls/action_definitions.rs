use bevy::prelude::*;

use crate::editor::input::{Action, ActionGroup, ValueKind};

/// Everything a UI widget can be bound to. The keymap does not use this: it binds shortcuts to
/// action names, so any registered action can have one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundAction {
	Editor(EditorAction),
	Camera(CameraControl),
}

impl BoundAction {
	/// Sends the event this action stands for.
	pub fn trigger(self, commands: &mut Commands) {
		match self {
			Self::Editor(action) => commands.trigger(action),
			// Camera controls are held rather than triggered, so there is no event to send.
			Self::Camera(_) => {}
		}
	}
}

impl Action for BoundAction {
	fn name(self) -> &'static str {
		match self {
			Self::Editor(action) => action.name(),
			Self::Camera(control) => control.name(),
		}
	}

	fn value(self) -> ValueKind {
		match self {
			Self::Editor(action) => action.value(),
			Self::Camera(control) => control.value(),
		}
	}
}

impl From<EditorAction> for BoundAction {
	fn from(action: EditorAction) -> Self {
		Self::Editor(action)
	}
}

impl From<CameraControl> for BoundAction {
	fn from(control: CameraControl) -> Self {
		Self::Camera(control)
	}
}

#[derive(Component, Clone, Default)]
pub struct ActionTrigger(pub Option<BoundAction>);

impl<Source: Into<BoundAction>> From<Source> for ActionTrigger {
	fn from(action: Source) -> Self {
		Self(Some(action.into()))
	}
}

// ## EditorAction ## //

#[derive(Debug, Clone, Copy, PartialEq, Eq, Event)]
pub enum EditorAction {
	Undo,
	Redo,
	Save,
	Open,
}

impl Action for EditorAction {
	// Stored in the keymap file, so they must stay stable even if a variant is renamed.
	fn name(self) -> &'static str {
		match self {
			Self::Undo => "editor.undo",
			Self::Redo => "editor.redo",
			Self::Save => "editor.save",
			Self::Open => "editor.open",
		}
	}

	fn value(self) -> ValueKind {
		ValueKind::Button
	}
}

impl ActionGroup for EditorAction {
	fn all() -> &'static [Self] {
		&[Self::Undo, Self::Redo, Self::Save, Self::Open]
	}
}

// ## CameraControl ## //

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraControl {
	Orbit,
	Pan,
}

impl Action for CameraControl {
	// Stored in the keymap file, so they must stay stable even if a variant is renamed.
	fn name(self) -> &'static str {
		match self {
			Self::Orbit => "camera.orbit",
			Self::Pan => "camera.pan",
		}
	}

	fn value(self) -> ValueKind {
		ValueKind::Button
	}
}

impl ActionGroup for CameraControl {
	fn all() -> &'static [Self] {
		&[Self::Orbit, Self::Pan]
	}
}
