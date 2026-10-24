use bevy::prelude::*;

use crate::{
	edit::{EditNotice, EditPreview, PreviewState, Proposal},
	house::House,
	text_fields::TextFocus,
	wall_tool_active::{ActiveTool, Tool, tool_active},
	wall_tool_selection::SelectedTarget,
};

/// Deletes the selected wall after showing what the deletion takes with it. Its dimensions
/// are edited with the fields in the panel.
pub struct WallToolEditWallPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WallToolEditWallSystems;

impl Plugin for WallToolEditWallPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<EditPreview>()
			.init_resource::<EditNotice>()
			.add_message::<DeleteSelectedWall>()
			.add_message::<ConfirmWallDeletion>()
			.add_systems(
				PostUpdate,
				(
					discard_stale_wall_preview,
					request_wall_deletion.run_if(tool_active(Tool::Select)),
					confirm_wall_deletion.run_if(tool_active(Tool::Select)),
				)
					.chain()
					.in_set(WallToolEditWallSystems),
			);
	}
}

/// Asks for the selected wall's deletion to be previewed.
#[derive(Message)]
pub struct DeleteSelectedWall;

/// Commits the previewed wall deletion.
#[derive(Message)]
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
	// The delete tool previews whatever is under the pointer, so selection does not apply.
	let outdated = match active_tool.0 {
		Tool::Select => selected.wall() != Some(wall),
		Tool::Delete => false,
		_ => true,
	};
	outdated || house_revision != house.revision()
}

/// Previews the deletion instead of performing it, so its effect can be seen first.
fn request_wall_deletion(
	mut requests: MessageReader<DeleteSelectedWall>,
	keyboard: Res<ButtonInput<KeyCode>>,
	typing: TextFocus,
	selected: Res<SelectedTarget>,
	house: Res<House>,
	mut preview: ResMut<EditPreview>,
) {
	let clicked = requests.read().count() > 0;
	let requested = clicked || (keyboard.just_pressed(KeyCode::Delete) && !typing.active());
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
	mut confirmations: MessageReader<ConfirmWallDeletion>,
	keyboard: Res<ButtonInput<KeyCode>>,
	typing: TextFocus,
	mut house: ResMut<House>,
	mut preview: ResMut<EditPreview>,
	mut notice: ResMut<EditNotice>,
) {
	let clicked = confirmations.read().count() > 0;
	if !preview.holds_wall_removal() {
		return;
	}
	if keyboard.just_pressed(KeyCode::Escape) && !typing.active() {
		preview.state = None;
		return;
	}
	let confirmed = clicked
		|| (keyboard.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter]) && !typing.active());
	// An invalid deletion keeps its preview, which already shows the reason.
	if !confirmed || preview.valid_candidate().is_none() {
		return;
	}
	let Some(Proposal::WallRemoval(wall)) = preview.state.as_ref().map(|state| state.proposal)
	else {
		return;
	};
	if let Some(candidate) = preview.take_valid(Proposal::WallRemoval(wall), house.revision()) {
		house.commit(candidate);
		notice.message = None;
	}
}

#[cfg(test)]
#[path = "tests/wall_tool_edit_wall.rs"]
mod tests;
