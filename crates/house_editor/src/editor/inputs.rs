use bevy::prelude::*;

use crate::editor_core::input::{Action, ActionGroup, ActionState, InputAppExt, ValueKind};

pub struct EditorInputPlugin;
impl Plugin for EditorInputPlugin {
	fn build(&self, app: &mut App) {
		app.add_input_actions::<EditorAction>();
		app.add_systems(Update, handle_editor_actions);
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

pub fn handle_editor_actions(state: Res<ActionState>) {
	for &action in EditorAction::all() {
		if state.just_pressed(action) {
			match action {
				EditorAction::Undo => info!("test: Undo"),
				EditorAction::Redo => info!("test: Redo"),
				EditorAction::Save => info!("test: Save"),
				EditorAction::Open => info!("test: Open"),
			}
		}
	}
}
