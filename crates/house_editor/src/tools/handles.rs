use bevy::{ecs::system::SystemParam, prelude::*};
pub use house_editor_core::WallHandle;
use house_editor_core::{
	DragPointer, HANDLE_LIFT_METRES, HandleDrag, advance_handle_drag, drag_proposal,
	handle_positions, handle_under_ray,
};

use crate::{
	config::interaction::InteractionConfig,
	document::edit::EditPreview,
	document::history::DocumentRestored,
	document::house::House,
	input::{Action, ActionState, ActionsPlugin},
	tools::active::{Tool, tool_active},
	tools::selection::{HoveredTarget, SelectedTarget, WallSelectionCarryOver},
	viewport::coordinates::{bevy_to_graph, graph_to_bevy},
	viewport::ground_pointer::GroundPointer,
	viewport::snapping::{SnapSettings, SnappedPointer, snapping_disabled},
};

const HANDLE_DRAW_RADIUS_METRES: f32 = 0.1;
const BEND_COLOR: Color = Color::srgb(0.85, 0.4, 0.95);
const HOVER_COLOR: Color = Color::srgb(1.0, 0.85, 0.2);

/// Drags the handles of the selected wall: its ends and junctions, its bend, or the wall
/// as a whole. A drag is one draft proposal; releasing commits it as one history entry.
pub struct WallToolHandlesPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WallToolHandlesSystems {
	/// Finds the handle under the pointer. Runs before selection so that grabbing a handle
	/// is not also a click that changes the selection.
	Hover,
	Drag,
}

impl Plugin for WallToolHandlesPlugin {
	fn build(&self, app: &mut App) {
		ActionsPlugin::ensure(app);
		app.init_resource::<InteractionConfig>();
		app.init_resource::<HoveredHandle>()
			.init_resource::<HandleDragState>()
			.init_resource::<EditPreview>()
			.init_resource::<WallSelectionCarryOver>()
			.add_message::<DocumentRestored>()
			.add_systems(
				PostUpdate,
				update_hovered_handle.in_set(WallToolHandlesSystems::Hover),
			)
			.add_systems(
				PostUpdate,
				(
					discard_drag_after_restore,
					update_handle_drag.run_if(tool_active(Tool::Select)),
					cancel_handle_drag.run_if(not(tool_active(Tool::Select))),
				)
					.chain()
					.in_set(WallToolHandlesSystems::Drag),
			)
			.add_systems(PostUpdate, draw_handles.after(WallToolHandlesSystems::Drag));
	}
}

/// The end or bend handle under the pointer. The body is not a visible handle, so it is
/// decided when a press arrives.
#[derive(Resource, Default, PartialEq)]
pub struct HoveredHandle(pub Option<WallHandle>);

/// The gesture in progress, as a Bevy resource.
#[derive(Resource, Default, Deref, DerefMut, PartialEq)]
struct HandleDragState(HandleDrag);

fn update_hovered_handle(
	tool: Res<crate::tools::active::ActiveTool>,
	house: Res<House>,
	selected: Res<SelectedTarget>,
	ground_pointer: Res<GroundPointer>,
	mut hovered: ResMut<HoveredHandle>,
) {
	let handle = (tool.0 == Tool::Select)
		.then(|| {
			let (wall, ray) = (selected.wall()?, ground_pointer.ray?);
			handle_under_ray(
				house.graph(),
				wall,
				bevy_to_graph(ray.origin),
				bevy_to_graph(*ray.direction),
			)
		})
		.flatten();
	hovered.set_if_neq(HoveredHandle(handle));
}

#[derive(SystemParam)]
struct HandleInput<'w> {
	config: Res<'w, InteractionConfig>,
	actions: Res<'w, ActionState>,
	ground_pointer: Res<'w, GroundPointer>,
	snapped_pointer: Res<'w, SnappedPointer>,
	snap_settings: Res<'w, SnapSettings>,
	hovered_handle: Res<'w, HoveredHandle>,
	hovered_target: Res<'w, HoveredTarget>,
	selected: Res<'w, SelectedTarget>,
}

fn update_handle_drag(
	input: HandleInput,
	mut house: ResMut<House>,
	mut drag: ResMut<HandleDragState>,
	mut preview: ResMut<EditPreview>,
	mut carry_over: ResMut<WallSelectionCarryOver>,
) {
	if input.actions.just_pressed(Action::Cancel) {
		drag.set_if_neq(HandleDragState(HandleDrag::Idle));
		if preview.holds_handle_drag() {
			preview.state = None;
		}
		return;
	}

	let step = advance_handle_drag(
		&input.config,
		drag.0,
		&input.actions,
		input.ground_pointer.hit,
		input.selected.wall(),
		input.hovered_handle.0,
		input.hovered_target.wall(),
	);
	drag.set_if_neq(HandleDragState(step.drag));
	let Some(active) = step.active else {
		return;
	};

	let proposal = input.ground_pointer.hit.and_then(|hit| {
		drag_proposal(
			active,
			&DragPointer {
				hit,
				snap_target: input.snapped_pointer.target,
				snapping_disabled: snapping_disabled(&input.actions),
				grid_spacing: input.snap_settings.grid_spacing(),
			},
			house.graph(),
		)
	});
	// Reborrowing retains Bevy's change tracking until the helper actually writes.
	if let Some(proposal) = proposal {
		EditPreview::update(
			preview.reborrow(),
			proposal,
			house.revision(),
			house.committed(),
		);
	}

	if step.released {
		// A rejected drag is abandoned, leaving the layout as it was.
		if let Some(proposal) = preview
			.state
			.as_ref()
			.map(|state| state.proposal)
			.filter(|_| preview.holds_handle_drag())
			&& let Some(candidate) = preview.take_valid(proposal, house.revision())
		{
			carry_over.0 = candidate
				.report()
				.rebuilt_walls
				.iter()
				.find(|&&(old, _)| old == active.wall)
				.map(|&(_, new)| new);
			house.commit(candidate);
		}
		if preview.holds_handle_drag() {
			preview.state = None;
		}
		drag.0 = HandleDrag::Idle;
	}
}

/// Leaving the tool abandons a drag instead of resuming it on return.
fn cancel_handle_drag(mut drag: ResMut<HandleDragState>, mut preview: ResMut<EditPreview>) {
	drag.set_if_neq(HandleDragState(HandleDrag::Idle));
	if preview.holds_handle_drag() {
		preview.state = None;
	}
}

/// A drag remembers positions from before an undo or redo, which would then be committed
/// over the restored house.
fn discard_drag_after_restore(
	mut restored: MessageReader<DocumentRestored>,
	mut drag: ResMut<HandleDragState>,
) {
	if restored.read().count() > 0 {
		drag.set_if_neq(HandleDragState(HandleDrag::Idle));
	}
}

/// The bend handle, which the selection's own highlight does not draw, and an emphasis on
/// whichever handle the pointer is over.
fn draw_handles(
	tool: Res<crate::tools::active::ActiveTool>,
	house: Res<House>,
	selected: Res<SelectedTarget>,
	hovered: Res<HoveredHandle>,
	mut gizmos: Gizmos,
) {
	let Some(wall) = selected.wall().filter(|_| tool.0 == Tool::Select) else {
		return;
	};
	let Some(positions) = handle_positions(house.graph(), wall) else {
		return;
	};
	let mut sphere = |position: Vec3, color: Color| {
		gizmos.sphere(
			Isometry3d::from_translation(graph_to_bevy(position)),
			HANDLE_DRAW_RADIUS_METRES,
			color,
		);
	};
	sphere(positions.bend, BEND_COLOR);
	if let Some(handle) = hovered.0 {
		let (position, color) = match handle {
			WallHandle::Endpoint(node) if node == wall.origin() => (positions.origin, HOVER_COLOR),
			WallHandle::Endpoint(_) => (positions.destination, HOVER_COLOR),
			WallHandle::Bend(_) | WallHandle::Body(_) => (positions.bend, HOVER_COLOR),
		};
		sphere(position + Vec3::Z * HANDLE_LIFT_METRES, color);
	}
}

#[cfg(test)]
#[path = "../tests/tools/handles.rs"]
mod tests;
