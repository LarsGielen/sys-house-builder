use bevy::prelude::*;

use crate::{
	edit::{EditNotice, EditPreview},
	house::House,
	text_fields::TextFocus,
};

/// Undoes and redoes committed edits, from buttons or keyboard shortcuts.
pub struct HistoryPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HistorySystems;

impl Plugin for HistoryPlugin {
	fn build(&self, app: &mut App) {
		app.add_message::<HistoryRequest>()
			.add_message::<DocumentRestored>()
			.add_systems(PostUpdate, apply_history_requests.in_set(HistorySystems));
	}
}

#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryRequest {
	Undo,
	Redo,
}

/// The house was replaced by an earlier or later state. Tools holding a gesture that began on
/// the previous house discard it, since its handles and base document no longer apply.
#[derive(Message)]
pub struct DocumentRestored;

fn apply_history_requests(
	mut requests: MessageReader<HistoryRequest>,
	keyboard: Res<ButtonInput<KeyCode>>,
	typing: TextFocus,
	mut house: ResMut<House>,
	mut preview: ResMut<EditPreview>,
	mut notice: ResMut<EditNotice>,
	mut restored: MessageWriter<DocumentRestored>,
) {
	let shortcut = if typing.active() {
		None
	} else {
		shortcut_request(&keyboard)
	};
	for request in requests.read().copied().chain(shortcut) {
		let changed = match request {
			HistoryRequest::Undo => house.undo(),
			HistoryRequest::Redo => house.redo(),
		};
		if !changed {
			continue;
		}
		preview.state = None;
		notice.message = None;
		restored.write(DocumentRestored);
	}
}

/// Ctrl+Z undoes; Ctrl+Shift+Z and Ctrl+Y redo.
fn shortcut_request(keyboard: &ButtonInput<KeyCode>) -> Option<HistoryRequest> {
	if !keyboard.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
		return None;
	}
	let shift = keyboard.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
	if keyboard.just_pressed(KeyCode::KeyZ) {
		Some(if shift {
			HistoryRequest::Redo
		} else {
			HistoryRequest::Undo
		})
	} else if keyboard.just_pressed(KeyCode::KeyY) {
		Some(HistoryRequest::Redo)
	} else {
		None
	}
}

#[cfg(test)]
#[path = "tests/history.rs"]
mod tests;
