use bevy::prelude::*;

use crate::editor::input::{Action, ActionGroup, AxisMode, ValueKind};

/// Everything a UI widget can be bound to. The keymap does not use this: it binds shortcuts to
/// action names, so any registered action can have one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundAction {
	Editor(EditorAction),
	Camera(CameraAction),
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

impl From<CameraAction> for BoundAction {
	fn from(control: CameraAction) -> Self {
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
			EditorAction::Undo => "editor.undo",
			EditorAction::Redo => "editor.redo",
			EditorAction::Save => "editor.save",
			EditorAction::Open => "editor.open",
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
pub enum CameraAction {
	Orbit,
	Pan,
	Look,
	Zoom,
}

impl Action for CameraAction {
	// Stored in the keymap file, so they must stay stable even if a variant is renamed.
	fn name(self) -> &'static str {
		match self {
			CameraAction::Orbit => "camera.orbit",
			CameraAction::Pan => "camera.pan",
			CameraAction::Look => "camera.look",
			CameraAction::Zoom => "camera.zoom",
		}
	}

	fn value(self) -> ValueKind {
		match self {
			CameraAction::Orbit => ValueKind::Button,
			CameraAction::Pan => ValueKind::Button,
			CameraAction::Look => ValueKind::Axis2D(AxisMode::Relative),
			CameraAction::Zoom => ValueKind::Axis1D(AxisMode::Relative),
		}
	}
}

impl ActionGroup for CameraAction {
	fn all() -> &'static [Self] {
		&[Self::Orbit, Self::Pan, Self::Look, Self::Zoom]
	}
}
