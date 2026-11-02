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

#[derive(Component, Clone, Default)]
pub struct ActionTrigger(pub EditorAction);
