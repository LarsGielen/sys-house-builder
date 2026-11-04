use bevy::prelude::*;

/// Everything a shortcut can be bound to. Sharing one type keeps the rule that a shortcut
/// belongs to a single action across both kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundAction {
	Editor(EditorAction),
	Camera(CameraControl),
}

impl BoundAction {
	pub(super) fn from_name(name: &str) -> Option<Self> {
		EditorAction::from_name(name)
			.map(Self::Editor)
			.or_else(|| CameraControl::from_name(name).map(Self::Camera))
	}

	/// Sends the event this action stands for.
	pub fn trigger(self, commands: &mut Commands) {
		match self {
			Self::Editor(action) => commands.trigger(action),
			// Camera controls are held rather than triggered, so there is no event to send.
			Self::Camera(_) => {}
		}
	}
}

impl From<EditorAction> for BoundAction {
	fn from(action: EditorAction) -> Self {
		Self::Editor(action)
	}
}

#[derive(Component, Clone, Default)]
pub struct ActionTrigger(pub Option<BoundAction>);

impl<Action: Into<BoundAction>> From<Action> for ActionTrigger {
	fn from(action: Action) -> Self {
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

impl EditorAction {
	// Stored in the keymap file, so they must stay stable even if a variant is renamed.
	pub fn from_name(name: &str) -> Option<Self> {
		match name {
			"editor.undo" => Some(Self::Undo),
			"editor.redo" => Some(Self::Redo),
			"editor.save" => Some(Self::Save),
			"editor.open" => Some(Self::Open),
			_ => None,
		}
	}
}

// ## CameraControl ## //

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraControl {
	Orbit,
	Pan,
}

impl CameraControl {
	// Stored in the keymap file, so they must stay stable even if a variant is renamed.
	fn from_name(name: &str) -> Option<Self> {
		match name {
			"camera.orbit" => Some(Self::Orbit),
			"camera.pan" => Some(Self::Pan),
			_ => None,
		}
	}
}
