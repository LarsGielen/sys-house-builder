use bevy::{ecs::system::SystemParam, prelude::*};
use house_editor_core::{
	OpeningDrag, OpeningSnapOptions, advance_opening_drag, opening_draft_is_stale,
	opening_drag_proposal,
};

use crate::{
	actions::{EditorAction, EditorActionPlugin},
	config::interaction::InteractionConfig,
	document::edit::{
		EditNotice, EditPreview, PreviewState, Proposal, propose_opening_duplicate,
		propose_opening_removal,
	},
	document::history::DocumentRestored,
	document::house::House,
	input::{Action, ActionState, ActionsPlugin},
	tools::active::{ActiveTool, Tool, tool_active},
	tools::selection::{HoveredTarget, SelectedTarget, SelectionTarget},
	viewport::ground_pointer::GroundPointer,
	viewport::snapping::{SnapSettings, snapping_disabled},
};

/// Moves, duplicates, and deletes the selected opening. Fields in the panel edit the rest of
/// its properties.
pub struct WallToolEditOpeningPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WallToolEditOpeningSystems;

impl Plugin for WallToolEditOpeningPlugin {
	fn build(&self, app: &mut App) {
		EditorActionPlugin::ensure(app);
		ActionsPlugin::ensure(app);
		app.init_resource::<InteractionConfig>();
		app.init_resource::<OpeningDragState>()
			.init_resource::<EditPreview>()
			.init_resource::<EditNotice>()
			.add_message::<DocumentRestored>()
			.add_systems(
				PostUpdate,
				(
					discard_opening_drag_after_restore,
					discard_stale_edit_preview,
					update_opening_drag.run_if(tool_active(Tool::Select)),
					apply_opening_commands,
				)
					.chain()
					.in_set(WallToolEditOpeningSystems),
			);
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeleteSelectedOpening;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DuplicateSelectedOpening;

/// The gesture in progress, as a Bevy resource.
#[derive(Resource, Default, Deref, DerefMut, PartialEq)]
struct OpeningDragState(OpeningDrag);

/// A drag remembers the opening's spec from before an undo or redo, which would then be
/// committed over the restored house.
fn discard_opening_drag_after_restore(
	mut restored: MessageReader<DocumentRestored>,
	mut drag: ResMut<OpeningDragState>,
) {
	if restored.read().count() > 0 {
		drag.set_if_neq(OpeningDragState(OpeningDrag::Idle));
	}
}

#[derive(SystemParam)]
struct DragInput<'w> {
	config: Res<'w, InteractionConfig>,
	actions: Res<'w, ActionState>,
	ground_pointer: Res<'w, GroundPointer>,
	hovered: Res<'w, HoveredTarget>,
	selected: Res<'w, SelectedTarget>,
}

fn update_opening_drag(
	input: DragInput,
	snap_settings: Res<SnapSettings>,
	mut house: ResMut<House>,
	mut notice: ResMut<EditNotice>,
	mut drag: ResMut<OpeningDragState>,
	mut preview: ResMut<EditPreview>,
) {
	// Escape belongs to a focused text field, which reverts its own edit.
	if input.actions.just_pressed(Action::Cancel) {
		drag.set_if_neq(OpeningDragState(OpeningDrag::Idle));
		if preview.holds_opening_edit() {
			preview.state = None;
		}
		return;
	}

	let (next, released) = advance_opening_drag(
		&input.config,
		drag.0,
		&input.actions,
		input.ground_pointer.hit,
		input.hovered.opening(),
		input.selected.opening(),
		house.graph(),
	);
	let OpeningDrag::Pressed {
		opening,
		original,
		grab_distance,
		dragged: true,
		..
	} = next
	else {
		// A press that never became a drag is only a selection click.
		drag.set_if_neq(OpeningDragState(if released {
			OpeningDrag::Idle
		} else {
			next
		}));
		return;
	};

	let options = (!snapping_disabled(&input.actions)).then_some(OpeningSnapOptions {
		grid_spacing: snap_settings.grid_spacing(),
		gap: snap_settings.opening_gap(),
		moving: Some(opening),
	});
	let proposal = input.ground_pointer.hit.and_then(|hit| {
		opening_drag_proposal(
			house.graph(),
			opening,
			original,
			grab_distance,
			hit,
			options,
			|wall| house.opening_bounds(wall),
		)
	});
	// Reborrowing retains Bevy's change tracking until the helper actually writes.
	if let Some(proposal) = proposal {
		EditPreview::update(
			preview.reborrow(),
			Proposal::OpeningEdit(proposal),
			house.revision(),
			house.committed(),
		);
	}

	if released {
		// An invalid drag is abandoned, leaving geometry and category as they were.
		if let Some(Proposal::OpeningEdit(edit)) =
			preview.state.as_ref().map(|state| state.proposal)
			&& edit.opening == opening
			&& let Some(candidate) =
				preview.take_valid(Proposal::OpeningEdit(edit), house.revision())
		{
			house.commit(candidate);
			notice.message = None;
		}
		if preview.holds_opening_edit() {
			preview.state = None;
		}
		drag.0 = OpeningDrag::Idle;
	} else {
		drag.set_if_neq(OpeningDragState(next));
	}
}

/// A draft for an opening that is no longer selected, or built on an older house, would
/// keep hiding the committed walls and could be committed over newer changes.
fn discard_stale_edit_preview(
	active_tool: Res<ActiveTool>,
	selected: Res<SelectedTarget>,
	house: Res<House>,
	mut drag: ResMut<OpeningDragState>,
	mut preview: ResMut<EditPreview>,
	mut notice: ResMut<EditNotice>,
) {
	let drag_is_stale = match drag.0 {
		OpeningDrag::Pressed { opening, .. } => {
			selected.opening() != Some(opening) || house.graph().opening(opening).is_none()
		}
		OpeningDrag::Idle => false,
	};
	if active_tool.0 != Tool::Select || drag_is_stale {
		drag.set_if_neq(OpeningDragState(OpeningDrag::Idle));
	}
	if selected.is_changed() && notice.message.is_some() {
		notice.message = None;
	}
	let stale = match &preview.state {
		Some(PreviewState {
			proposal: Proposal::OpeningEdit(edit),
			house_revision,
			..
		}) => opening_draft_is_stale(
			active_tool.0,
			selected.opening(),
			edit.opening,
			*house_revision,
			house.revision(),
		),
		_ => false,
	};
	if stale {
		preview.state = None;
	}
}

#[derive(SystemParam)]
struct OpeningRequests<'w, 's> {
	stream: MessageReader<'w, 's, EditorAction>,
	actions: Res<'w, ActionState>,
}

impl OpeningRequests<'_, '_> {
	/// The stream is drained every frame so that a request is never delayed to a later
	/// selection.
	fn take(&mut self) -> (bool, bool) {
		let (mut delete_clicked, mut duplicate) = (false, false);
		for action in self.stream.read() {
			match action {
				EditorAction::DeleteSelectedOpening(_) => delete_clicked = true,
				EditorAction::DuplicateSelectedOpening(_) => duplicate = true,
				_ => {}
			}
		}
		let delete = delete_clicked || self.actions.just_pressed(Action::Delete);
		(delete, duplicate)
	}
}

fn apply_opening_commands(
	active_tool: Res<ActiveTool>,
	mut requests: OpeningRequests,
	snap_settings: Res<SnapSettings>,
	mut house: ResMut<House>,
	mut selected: ResMut<SelectedTarget>,
	mut notice: ResMut<EditNotice>,
) {
	// Requests are consumed even when this tool is not active, so they cannot fire later.
	let (delete_requested, duplicate_requested) = requests.take();
	if active_tool.0 != Tool::Select {
		return;
	}
	let Some(opening) = selected.opening() else {
		return;
	};

	if delete_requested {
		match propose_opening_removal(house.committed(), opening) {
			Ok(candidate) => {
				house.commit(candidate);
				selected.0 = None;
				notice.message = None;
			}
			Err(error) => notice.message = Some(format!("Cannot delete: {error}")),
		}
	} else if duplicate_requested {
		match propose_opening_duplicate(house.committed(), opening, snap_settings.opening_gap()) {
			Ok((candidate, copy)) => {
				house.commit(candidate);
				selected.0 = Some(SelectionTarget::Opening(copy));
				notice.message = None;
			}
			Err(error) => notice.message = Some(format!("Cannot duplicate: {error}")),
		}
	}
}

#[cfg(test)]
#[path = "../tests/tools/edit_opening.rs"]
mod tests;
