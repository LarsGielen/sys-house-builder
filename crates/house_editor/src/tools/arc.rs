use bevy::{ecs::system::SystemParam, prelude::*};
use house_editor_core::{ArcDraft, ArcMode, NoticeChange, advance_arc_draft, arc_proposal};

use crate::{
	actions::EditorAction,
	actions::settings::{SettingsAction, SettingsActionPlugin, SettingsActionSystems},
	config::interaction::InteractionConfig,
	document::edit::{EditNotice, EditPreview, Proposal},
	document::history::DocumentRestored,
	document::house::House,
	input::{Action, ActionState, ActionsPlugin},
	tools::active::{Tool, tool_active},
	tools::drawing::WallDrawingSettings,
	viewport::coordinates::graph_to_bevy,
	viewport::ground_pointer::GroundPointer,
	viewport::snapping::SnappedPointer,
};

const GUIDE_COLOR: Color = Color::srgb(0.2, 0.8, 0.95);
const GUIDE_HEIGHT_METRES: f32 = 0.03;
const GUIDE_MARKER_RADIUS_METRES: f32 = 0.1;
const DIRECTION_GUIDE_LENGTH_METRES: f32 = 1.0;

/// Draws arcs. A free arc takes its endpoints like a straight wall and then a bend position;
/// the tangent modes continue one or two existing wall ends instead of asking for a bend.
pub struct WallToolArcPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WallToolArcSystems;

impl Plugin for WallToolArcPlugin {
	fn build(&self, app: &mut App) {
		ActionsPlugin::ensure(app);
		app.init_resource::<InteractionConfig>();
		SettingsActionPlugin::ensure(app);
		app.add_systems(PostUpdate, apply_arc_mode.in_set(SettingsActionSystems));
		app.init_resource::<ArcModeState>()
			.init_resource::<ArcDraftState>()
			.init_resource::<WallDrawingSettings>()
			.init_resource::<EditPreview>()
			.init_resource::<EditNotice>()
			.add_message::<DocumentRestored>()
			.add_systems(
				PostUpdate,
				(
					discard_arc_draft_after_restore,
					restart_when_mode_changes.run_if(resource_changed::<ArcModeState>),
					update_wall_arc.run_if(tool_active(Tool::DrawArc)),
					cancel_wall_arc.run_if(not(tool_active(Tool::DrawArc))),
					draw_arc_guides.run_if(tool_active(Tool::DrawArc)),
				)
					.chain()
					.in_set(WallToolArcSystems),
			);
	}
}

/// The chosen arc mode, as a Bevy resource.
#[derive(Resource, Default, Deref, DerefMut, PartialEq)]
pub struct ArcModeState(pub ArcMode);

/// The gesture in progress, as a Bevy resource.
#[derive(Resource, Default, Deref, DerefMut, PartialEq)]
pub struct ArcDraftState(pub ArcDraft);

#[derive(SystemParam)]
struct ArcInput<'w> {
	config: Res<'w, InteractionConfig>,
	actions: Res<'w, ActionState>,
	ground_pointer: Res<'w, GroundPointer>,
	snapped_pointer: Res<'w, SnappedPointer>,
}

fn apply_arc_mode(mut actions: MessageReader<EditorAction>, mut mode: ResMut<ArcModeState>) {
	for action in actions.read().filter_map(|action| action.settings()) {
		if let SettingsAction::SetArcMode(new_mode) = action {
			mode.set_if_neq(ArcModeState(new_mode));
		}
	}
}

fn update_wall_arc(
	input: ArcInput,
	mode: Res<ArcModeState>,
	settings: Res<WallDrawingSettings>,
	mut house: ResMut<House>,
	mut notice: ResMut<EditNotice>,
	mut draft: ResMut<ArcDraftState>,
	mut preview: ResMut<EditPreview>,
) {
	// Escape belongs to a focused text field, which reverts its own edit.
	if input.actions.just_pressed(Action::Cancel) {
		draft.set_if_neq(ArcDraftState(ArcDraft::Idle));
		if preview.holds_arc_proposal() {
			preview.state = None;
		}
		return;
	}

	let step = advance_arc_draft(
		&input.config,
		draft.0,
		mode.0,
		&input.actions,
		input.ground_pointer.hit,
		input.snapped_pointer.target,
		house.graph(),
	);
	match step.notice {
		NoticeChange::Keep => {}
		NoticeChange::Clear => notice.message = None,
		NoticeChange::Set(message) => notice.message = Some(message),
	}
	let (next_draft, confirmed) = (step.draft, step.confirmed);
	draft.set_if_neq(ArcDraftState(next_draft));

	let proposal = arc_proposal(
		next_draft,
		mode.0,
		input.snapped_pointer.target,
		house.graph(),
		settings.dimensions(),
	);
	match proposal {
		Some(proposal) => {
			// Reborrowing retains Bevy's change tracking until the helper actually writes.
			EditPreview::update(
				preview.reborrow(),
				Proposal::Arc(proposal),
				house.revision(),
				house.committed(),
			);
		}
		// Keep the last preview while the pointer is off the ground.
		None if (input.snapped_pointer.target.is_some() || next_draft == ArcDraft::Idle)
			&& preview.holds_arc_proposal() =>
		{
			preview.state = None;
		}
		None => {}
	}

	if confirmed
		&& let Some(proposal) = proposal
		&& let Some(candidate) = preview.take_valid(Proposal::Arc(proposal), house.revision())
	{
		house.commit(candidate);
		notice.message = None;
		draft.0 = ArcDraft::Idle;
	}
}

/// Leaving the tool abandons an unfinished arc instead of resuming it on return.
fn cancel_wall_arc(mut draft: ResMut<ArcDraftState>, mut preview: ResMut<EditPreview>) {
	draft.set_if_neq(ArcDraftState(ArcDraft::Idle));
	if preview.holds_arc_proposal() {
		preview.state = None;
	}
}

fn restart_when_mode_changes(mut draft: ResMut<ArcDraftState>, mut preview: ResMut<EditPreview>) {
	draft.set_if_neq(ArcDraftState(ArcDraft::Idle));
	if preview.holds_arc_proposal() {
		preview.state = None;
	}
}

/// An arc started on the house before an undo or redo would end on a different house.
fn discard_arc_draft_after_restore(
	mut restored: MessageReader<DocumentRestored>,
	mut draft: ResMut<ArcDraftState>,
) {
	if restored.read().count() > 0 {
		draft.set_if_neq(ArcDraftState(ArcDraft::Idle));
	}
}

/// Shows what the arc is built from: the chord, the bend, and the wall ends it continues.
fn draw_arc_guides(
	draft: Res<ArcDraftState>,
	snapped_pointer: Res<SnappedPointer>,
	mut gizmos: Gizmos,
) {
	let to_bevy = |position: Vec2| graph_to_bevy(position.extend(GUIDE_HEIGHT_METRES));
	let marker = |position: Vec2, gizmos: &mut Gizmos| {
		gizmos.sphere(
			Isometry3d::from_translation(to_bevy(position)),
			GUIDE_MARKER_RADIUS_METRES,
			GUIDE_COLOR,
		);
	};
	match draft.0 {
		ArcDraft::Idle => {}
		ArcDraft::Pressed { start, .. } => marker(start, &mut gizmos),
		ArcDraft::AwaitingEnd {
			start,
			start_direction,
		} => {
			marker(start, &mut gizmos);
			let guide_end = start_direction.map_or_else(
				|| snapped_pointer.target.map(|target| target.position),
				|direction| Some(start + direction * DIRECTION_GUIDE_LENGTH_METRES),
			);
			if let Some(end) = guide_end {
				gizmos.line(to_bevy(start), to_bevy(end), GUIDE_COLOR);
			}
		}
		ArcDraft::AwaitingBend {
			start, end, bend, ..
		} => {
			marker(start, &mut gizmos);
			marker(end, &mut gizmos);
			gizmos.line(to_bevy(start), to_bevy(end), GUIDE_COLOR);
			marker(bend, &mut gizmos);
		}
	}
}

#[cfg(test)]
#[path = "../tests/tools/arc.rs"]
mod tests;
