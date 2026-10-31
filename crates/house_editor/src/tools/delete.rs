use bevy::prelude::*;

use crate::{
	document::edit::{EditNotice, EditPreview},
	document::house::House,
	input::{Action, ActionState, ActionsPlugin},
	tools::active::{ActiveTool, Tool, tool_active},
	tools::selection::{SelectionTarget, pick_target},
	viewport::coordinates::bevy_to_graph,
	viewport::ground_pointer::GroundPointer,
};

/// Deletes the wall or opening under the pointer with a click. What the click would delete
/// is previewed while hovering, so the preview is also the confirmation.
pub struct WallToolDeletePlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WallToolDeleteSystems;

impl Plugin for WallToolDeletePlugin {
	fn build(&self, app: &mut App) {
		ActionsPlugin::ensure(app);
		app.init_resource::<EditPreview>()
			.init_resource::<EditNotice>()
			.add_systems(
				PostUpdate,
				(
					update_deletion.run_if(tool_active(Tool::Delete)),
					// Only on a tool change, because the selection tool previews wall deletions too.
					cancel_deletion.run_if(
						resource_changed::<ActiveTool>.and_then(not(tool_active(Tool::Delete))),
					),
				)
					.in_set(WallToolDeleteSystems),
			);
	}
}

fn update_deletion(
	actions: Res<ActionState>,
	ground_pointer: Res<GroundPointer>,
	mut house: ResMut<House>,
	mut notice: ResMut<EditNotice>,
	mut preview: ResMut<EditPreview>,
) {
	let target = ground_pointer.ray.and_then(|ray| {
		pick_target(
			house.graph(),
			house.mesh(),
			bevy_to_graph(ray.origin),
			bevy_to_graph(*ray.direction),
		)
	});

	// Reborrowing retains Bevy's change tracking until the helper actually writes.
	update_preview(target, &house, preview.reborrow());

	if actions.just_pressed(Action::ToolPress)
		&& preview.holds_removal()
		&& let Some(proposal) = preview.state.as_ref().map(|state| state.proposal)
		&& let Some(candidate) = preview.take_valid(proposal, house.revision())
	{
		house.commit(candidate);
		notice.message = None;
	}
}

/// Leaving the tool drops its preview, which would otherwise keep hiding the committed walls.
fn cancel_deletion(mut preview: ResMut<EditPreview>) {
	if preview.holds_removal() {
		preview.state = None;
	}
}

fn update_preview(target: Option<SelectionTarget>, house: &House, mut preview: Mut<EditPreview>) {
	let Some(target) = target else {
		if preview.holds_removal() {
			preview.state = None;
		}
		return;
	};
	EditPreview::update(
		preview,
		target.removal(),
		house.revision(),
		house.committed(),
	);
}

#[cfg(test)]
#[path = "../tests/tools/delete.rs"]
mod tests;
