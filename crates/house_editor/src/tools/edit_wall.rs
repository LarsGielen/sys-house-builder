use bevy::prelude::*;
use house_editor_core::{DeletionStep, deletion_step, wall_draft_is_stale};

use crate::{
	actions::{EditorAction, EditorActionPlugin},
	document::edit::{EditNotice, EditPreview, PreviewState, Proposal},
	document::house::House,
	input::{Action, ActionState, ActionsPlugin},
	tools::active::{ActiveTool, Tool},
	tools::selection::SelectedTarget,
};

/// Deletes the selected wall after showing what the deletion takes with it. Its dimensions
/// are edited with the fields in the panel.
pub struct WallToolEditWallPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WallToolEditWallSystems;

impl Plugin for WallToolEditWallPlugin {
	fn build(&self, app: &mut App) {
		EditorActionPlugin::ensure(app);
		ActionsPlugin::ensure(app);
		app.init_resource::<EditPreview>()
			.init_resource::<EditNotice>()
			.add_systems(
				PostUpdate,
				(
					discard_stale_wall_preview,
					request_wall_deletion,
					confirm_wall_deletion,
				)
					.chain()
					.in_set(WallToolEditWallSystems),
			);
	}
}

/// Asks for the selected wall's deletion to be previewed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeleteSelectedWall;

/// Commits the previewed wall deletion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConfirmWallDeletion;

/// A wall draft that outlives its wall's selection, or the house it was built on, would
/// keep hiding the committed walls and could be committed over newer changes.
fn discard_stale_wall_preview(
	active_tool: Res<ActiveTool>,
	selected: Res<SelectedTarget>,
	house: Res<House>,
	mut preview: ResMut<EditPreview>,
) {
	let stale = match &preview.state {
		Some(PreviewState {
			proposal: Proposal::WallDimensions(edit),
			house_revision,
			..
		}) => is_stale(&active_tool, &selected, edit.wall, *house_revision, &house),
		Some(PreviewState {
			proposal: Proposal::WallRemoval(wall),
			house_revision,
			..
		}) => is_stale(&active_tool, &selected, *wall, *house_revision, &house),
		_ => false,
	};
	if stale {
		preview.state = None;
	}
}

fn is_stale(
	active_tool: &ActiveTool,
	selected: &SelectedTarget,
	wall: wall_graph::Wall,
	house_revision: u64,
	house: &House,
) -> bool {
	wall_draft_is_stale(
		active_tool.0,
		selected.wall(),
		wall,
		house_revision,
		house.revision(),
	)
}

/// Previews the deletion instead of performing it, so its effect can be seen first.
fn request_wall_deletion(
	active_tool: Res<ActiveTool>,
	mut requests: MessageReader<EditorAction>,
	actions: Res<ActionState>,
	selected: Res<SelectedTarget>,
	house: Res<House>,
	mut preview: ResMut<EditPreview>,
) {
	let clicked = requests
		.read()
		.any(|action| matches!(action, EditorAction::DeleteSelectedWall(_)));
	// A request is consumed even when this tool is not active, so it cannot fire later.
	if active_tool.0 != Tool::Select {
		return;
	}
	let requested = clicked || actions.just_pressed(Action::Delete);
	let Some(wall) = selected.wall().filter(|_| requested) else {
		return;
	};
	EditPreview::update(
		preview.reborrow(),
		Proposal::WallRemoval(wall),
		house.revision(),
		house.committed(),
	);
}

/// Enter or the confirm button commits a previewed deletion; Escape abandons it.
fn confirm_wall_deletion(
	active_tool: Res<ActiveTool>,
	mut confirmations: MessageReader<EditorAction>,
	actions: Res<ActionState>,
	mut house: ResMut<House>,
	mut preview: ResMut<EditPreview>,
	mut notice: ResMut<EditNotice>,
) {
	let clicked = confirmations
		.read()
		.any(|action| matches!(action, EditorAction::ConfirmWallDeletion(_)));
	if active_tool.0 != Tool::Select {
		return;
	}
	let Some(Proposal::WallRemoval(wall)) = preview
		.state
		.as_ref()
		.map(|state| state.proposal)
		.filter(|_| preview.holds_wall_removal())
	else {
		return;
	};
	let step = deletion_step(
		actions.just_pressed(Action::Cancel),
		clicked || actions.just_pressed(Action::Confirm),
		preview.valid_candidate().is_some(),
	);
	match step {
		DeletionStep::Wait => {}
		DeletionStep::Abandon => preview.state = None,
		DeletionStep::Commit => {
			if let Some(candidate) =
				preview.take_valid(Proposal::WallRemoval(wall), house.revision())
			{
				house.commit(candidate);
				notice.message = None;
			}
		}
	}
}

#[cfg(test)]
#[path = "../tests/tools/edit_wall.rs"]
mod tests;
