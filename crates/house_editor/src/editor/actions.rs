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
	// These names are stored in the keymap file, so they must stay stable even if a variant is renamed.
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
