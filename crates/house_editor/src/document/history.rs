use bevy::prelude::*;

use crate::{
	actions::{EditorAction, EditorActionPlugin},
	document::edit::{EditNotice, EditPreview},
	document::house::House,
	input::{Action, ActionState, ActionsPlugin},
};

/// Undoes and redoes committed edits, from buttons or keyboard shortcuts.
pub struct HistoryPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HistorySystems;

impl Plugin for HistoryPlugin {
	fn build(&self, app: &mut App) {
		EditorActionPlugin::ensure(app);
		ActionsPlugin::ensure(app);
		app.init_resource::<EditPreview>()
			.init_resource::<EditNotice>()
			.add_message::<DocumentRestored>()
			.add_systems(PostUpdate, apply_history_requests.in_set(HistorySystems));
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryRequest {
	Undo,
	Redo,
}

/// The house was replaced by an earlier or later state. Tools holding a gesture that began on
/// the previous house discard it, since its handles and base document no longer apply.
#[derive(Message)]
pub struct DocumentRestored;

fn apply_history_requests(
	mut requests: MessageReader<EditorAction>,
	actions: Res<ActionState>,
	mut house: ResMut<House>,
	mut preview: ResMut<EditPreview>,
	mut notice: ResMut<EditNotice>,
	mut restored: MessageWriter<DocumentRestored>,
) {
	let shortcut = shortcut_request(&actions);
	for request in requests
		.read()
		.filter_map(|action| action.history())
		.chain(shortcut)
	{
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

fn shortcut_request(actions: &ActionState) -> Option<HistoryRequest> {
	if actions.just_pressed(Action::Undo) {
		Some(HistoryRequest::Undo)
	} else if actions.just_pressed(Action::Redo) {
		Some(HistoryRequest::Redo)
	} else {
		None
	}
}

#[cfg(test)]
#[path = "../tests/document/history.rs"]
mod tests;
