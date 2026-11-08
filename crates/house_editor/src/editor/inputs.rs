use bevy::prelude::*;

use crate::core::input::{Action, ActionGroup, ActionState, InputAppExt, ValueKind};

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
	/// A toggle, so one value is shared by its shortcut and the widget that shows it.
	ToggleSnap,
}

impl Action for EditorAction {
	// Stored in the keymap file, so they must stay stable even if a variant is renamed.
	fn name(self) -> &'static str {
		match self {
			EditorAction::Undo => "editor.undo",
			EditorAction::Redo => "editor.redo",
			EditorAction::Save => "editor.save",
			EditorAction::Open => "editor.open",
			EditorAction::ToggleSnap => "editor.toggle_snap",
		}
	}

	fn value(self) -> ValueKind {
		match self {
			EditorAction::ToggleSnap => ValueKind::Toggle,
			_ => ValueKind::Button,
		}
	}
}

impl ActionGroup for EditorAction {
	fn all() -> &'static [Self] {
		&[
			Self::Undo,
			Self::Redo,
			Self::Save,
			Self::Open,
			Self::ToggleSnap,
		]
	}
}

pub fn handle_editor_actions(state: Res<ActionState>) {
	for &action in EditorAction::all() {
		match action {
			EditorAction::Undo if state.just_pressed(action) => info!("test: Undo"),
			EditorAction::Redo if state.just_pressed(action) => info!("test: Redo"),
			EditorAction::Save if state.just_pressed(action) => info!("test: Save"),
			EditorAction::Open if state.just_pressed(action) => info!("test: Open"),
			EditorAction::ToggleSnap if state.just_toggled(action) => {
				info!(
					"test: Snap {}",
					if state.toggled(action) { "on" } else { "off" }
				)
			}
			_ => {}
		}
	}
}
