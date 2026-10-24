use bevy::{ecs::system::SystemParam, prelude::*};
use wall_graph::{OpeningId, OpeningSpec};

use crate::{
	edit::{
		EditNotice, EditPreview, OpeningEditProposal, PreviewState, Proposal,
		propose_opening_duplicate, propose_opening_removal,
	},
	ground_pointer::GroundPointer,
	house::House,
	opening_area::closest_distance_along_wall,
	opening_snap::{OpeningSnapOptions, snap_opening},
	snapping::{SnapSettings, snapping_disabled},
	text_fields::TextFocus,
	wall_tool_active::{ActiveTool, Tool, tool_active},
	wall_tool_selection::{HoveredTarget, SelectedTarget, SelectionTarget},
};

const DRAG_THRESHOLD_PIXELS: f32 = 6.0;

/// Moves, duplicates, and deletes the selected opening. Fields in the panel edit the rest of
/// its properties.
pub struct WallToolEditOpeningPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WallToolEditOpeningSystems;

impl Plugin for WallToolEditOpeningPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<OpeningDrag>()
			.init_resource::<EditPreview>()
			.init_resource::<EditNotice>()
			.add_message::<DeleteSelectedOpening>()
			.add_message::<DuplicateSelectedOpening>()
			.add_systems(
				PostUpdate,
				(
					discard_stale_edit_preview,
					update_opening_drag.run_if(tool_active(Tool::Select)),
					apply_opening_commands.run_if(tool_active(Tool::Select)),
				)
					.chain()
					.in_set(WallToolEditOpeningSystems),
			);
	}
}

#[derive(Message)]
pub struct DeleteSelectedOpening;

#[derive(Message)]
pub struct DuplicateSelectedOpening;

#[derive(Resource, Default, Clone, Copy, Debug, PartialEq)]
enum OpeningDrag {
	#[default]
	Idle,
	/// The button is down. It is a click until the pointer moves past the drag threshold.
	Pressed {
		opening: OpeningId,
		original: OpeningSpec,
		/// Where along the wall the pointer grabbed the opening, so it does not jump.
		grab_distance: f64,
		/// Viewport pixels, to measure the drag threshold.
		press_position: Vec2,
		dragged: bool,
	},
}

#[derive(SystemParam)]
struct DragInput<'w, 's> {
	mouse: Res<'w, ButtonInput<MouseButton>>,
	keyboard: Res<'w, ButtonInput<KeyCode>>,
	ground_pointer: Res<'w, GroundPointer>,
	hovered: Res<'w, HoveredTarget>,
	selected: Res<'w, SelectedTarget>,
	typing: TextFocus<'w, 's>,
}

fn update_opening_drag(
	input: DragInput,
	snap_settings: Res<SnapSettings>,
	mut house: ResMut<House>,
	mut notice: ResMut<EditNotice>,
	mut drag: ResMut<OpeningDrag>,
	mut preview: ResMut<EditPreview>,
) {
	// Escape belongs to a focused text field, which reverts its own edit.
	if input.keyboard.just_pressed(KeyCode::Escape) && !input.typing.active() {
		drag.set_if_neq(OpeningDrag::Idle);
		if preview.holds_opening_edit() {
			preview.state = None;
		}
		return;
	}

	let (next, released) = advance_drag(*drag, &input, &house);
	let OpeningDrag::Pressed {
		opening,
		original,
		grab_distance,
		dragged: true,
		..
	} = next
	else {
		// A press that never became a drag is only a selection click.
		drag.set_if_neq(if released { OpeningDrag::Idle } else { next });
		return;
	};

	let options = (!snapping_disabled(&input.keyboard)).then_some(OpeningSnapOptions {
		grid_spacing: snap_settings.grid_spacing(),
		gap: snap_settings.opening_gap(),
		moving: Some(opening),
	});
	let proposal = input.ground_pointer.hit.and_then(|hit| {
		let wall = house.graph().opening(opening)?.wall;
		let distance = closest_distance_along_wall(house.graph(), wall, hit.graph_position)?;
		let raw = OpeningSpec {
			center_distance: original.center_distance + distance - grab_distance,
			..original
		};
		Some(OpeningEditProposal {
			opening,
			spec: options.map_or(raw, |options| {
				snap_opening(house.graph(), wall, raw, options)
			}),
		})
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
		*drag = OpeningDrag::Idle;
	} else {
		drag.set_if_neq(next);
	}
}

/// Starts a drag on the selected opening, tracks the threshold, and reports the release.
fn advance_drag(drag: OpeningDrag, input: &DragInput, house: &House) -> (OpeningDrag, bool) {
	match drag {
		OpeningDrag::Idle => {
			let started = input
				.mouse
				.just_pressed(MouseButton::Left)
				.then(|| {
					let opening = input
						.hovered
						.opening()
						.filter(|&id| Some(id) == input.selected.opening())?;
					let hit = input.ground_pointer.hit?;
					let current = house.graph().opening(opening)?;
					Some(OpeningDrag::Pressed {
						opening,
						original: current.spec,
						grab_distance: closest_distance_along_wall(
							house.graph(),
							current.wall,
							hit.graph_position,
						)?,
						press_position: hit.viewport_position,
						dragged: false,
					})
				})
				.flatten();
			(started.unwrap_or(drag), false)
		}
		OpeningDrag::Pressed {
			opening,
			original,
			grab_distance,
			press_position,
			dragged,
		} => {
			let dragged = dragged
				|| input.ground_pointer.hit.is_some_and(|hit| {
					hit.viewport_position.distance(press_position) > DRAG_THRESHOLD_PIXELS
				});
			(
				OpeningDrag::Pressed {
					opening,
					original,
					grab_distance,
					press_position,
					dragged,
				},
				// A release over the panel still finishes the drag.
				input.mouse.just_released(MouseButton::Left),
			)
		}
	}
}

/// A draft for an opening that is no longer selected, or built on an older house, would
/// keep hiding the committed walls and could be committed over newer changes.
fn discard_stale_edit_preview(
	active_tool: Res<ActiveTool>,
	selected: Res<SelectedTarget>,
	house: Res<House>,
	mut drag: ResMut<OpeningDrag>,
	mut preview: ResMut<EditPreview>,
	mut notice: ResMut<EditNotice>,
) {
	let drag_is_stale = match *drag {
		OpeningDrag::Pressed { opening, .. } => {
			selected.opening() != Some(opening) || house.graph().opening(opening).is_none()
		}
		OpeningDrag::Idle => false,
	};
	if active_tool.0 != Tool::Select || drag_is_stale {
		drag.set_if_neq(OpeningDrag::Idle);
	}
	if selected.is_changed() && notice.message.is_some() {
		notice.message = None;
	}
	let stale = match &preview.state {
		Some(PreviewState {
			proposal: Proposal::OpeningEdit(edit),
			house_revision,
			..
		}) => {
			active_tool.0 != Tool::Select
				|| selected.opening() != Some(edit.opening)
				|| *house_revision != house.revision()
		}
		_ => false,
	};
	if stale {
		preview.state = None;
	}
}

#[derive(SystemParam)]
struct OpeningRequests<'w, 's> {
	deletes: MessageReader<'w, 's, DeleteSelectedOpening>,
	duplicates: MessageReader<'w, 's, DuplicateSelectedOpening>,
	keyboard: Res<'w, ButtonInput<KeyCode>>,
	typing: TextFocus<'w, 's>,
}

impl OpeningRequests<'_, '_> {
	/// Both readers are drained every frame so that a request is never delayed to a later
	/// selection.
	fn take(&mut self) -> (bool, bool) {
		let delete_clicked = self.deletes.read().count() > 0;
		let duplicate = self.duplicates.read().count() > 0;
		let delete = delete_clicked
			|| (self.keyboard.just_pressed(KeyCode::Delete) && !self.typing.active());
		(delete, duplicate)
	}
}

fn apply_opening_commands(
	mut requests: OpeningRequests,
	snap_settings: Res<SnapSettings>,
	mut house: ResMut<House>,
	mut selected: ResMut<SelectedTarget>,
	mut notice: ResMut<EditNotice>,
) {
	let (delete_requested, duplicate_requested) = requests.take();
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
#[path = "tests/wall_tool_edit_opening.rs"]
mod tests;
