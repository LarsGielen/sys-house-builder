use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Event)]
pub enum EditorAction {
	#[default]
	None,
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

#[derive(Component, Clone, Default)]
pub struct ActionTrigger(pub EditorAction);

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

/// Everything a shortcut can be bound to. Sharing one type keeps the rule that a shortcut
/// belongs to a single action across both kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BoundAction {
	Editor(EditorAction),
	Camera(CameraControl),
}

impl BoundAction {
	pub(super) fn from_name(name: &str) -> Option<Self> {
		EditorAction::from_name(name)
			.map(Self::Editor)
			.or_else(|| CameraControl::from_name(name).map(Self::Camera))
	}
}
