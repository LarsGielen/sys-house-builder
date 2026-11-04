use bevy::prelude::*;

use crate::editor::controls::EditorAction;

pub fn handle_action(action: On<EditorAction>) {
	match *action {
		EditorAction::Undo => info!("test: Undo"),
		EditorAction::Redo => info!("test: Redo"),
		EditorAction::Save => info!("test: Save"),
		EditorAction::Open => info!("test: Open"),
	}
}
