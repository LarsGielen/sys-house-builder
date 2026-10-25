use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use house_editor_core::{DraftAction, WallDraft, WallPreview, advance_draft, wall_preview};
use wall_graph::WallDimensions;

use crate::{
	actions::EditorAction,
	actions::settings::{SettingsAction, SettingsActionPlugin, SettingsActionSystems},
	config::interaction::InteractionConfig,
	document::edit::{EditNotice, EditPreview, PreviewState, Proposal},
	document::history::DocumentRestored,
	document::house::House,
	input::{Action, ActionState, ActionsPlugin},
	tools::active::{Tool, tool_active},
	viewport::ground_pointer::GroundPointer,
	viewport::snapping::SnappedPointer,
};

const HEIGHT_STEP_METRES: f32 = 0.1;
const MINIMUM_HEIGHT_METRES: f32 = 0.5;
const THICKNESS_STEP_METRES: f32 = 0.05;
const MINIMUM_THICKNESS_METRES: f32 = 0.05;

/// Draws straight walls: press to start, then drag-and-release or click twice to confirm.
pub struct WallToolDrawingPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WallToolDrawingSystems;
impl Plugin for WallToolDrawingPlugin {
	fn build(&self, app: &mut App) {
		ActionsPlugin::ensure(app);
		app.init_resource::<InteractionConfig>();
		SettingsActionPlugin::ensure(app);
		app.add_systems(
			PostUpdate,
			apply_wall_settings.in_set(SettingsActionSystems),
		);
		app.init_resource::<WallDrawingSettings>()
			.init_resource::<WallDraftState>()
			.init_resource::<EditPreview>()
			.init_resource::<EditNotice>()
			.add_message::<DocumentRestored>()
			.add_systems(
				PostUpdate,
				(
					discard_wall_draft_after_restore,
					update_wall_drawing.run_if(tool_active(Tool::DrawWall)),
					cancel_wall_drawing.run_if(not(tool_active(Tool::DrawWall))),
				)
					.in_set(WallToolDrawingSystems),
			);
	}
}

/// Dimensions applied to every wall the tool proposes.
#[derive(Resource, Default)]
pub struct WallDrawingSettings {
	dimensions: WallDimensions,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepDirection {
	Decrease,
	Increase,
}

impl StepDirection {
	fn sign(self) -> f32 {
		match self {
			Self::Decrease => -1.0,
			Self::Increase => 1.0,
		}
	}
}

impl WallDrawingSettings {
	pub fn dimensions(&self) -> WallDimensions {
		self.dimensions
	}

	pub fn step_height(&mut self, direction: StepDirection) {
		self.dimensions.height = (self.dimensions.height + direction.sign() * HEIGHT_STEP_METRES)
			.max(MINIMUM_HEIGHT_METRES);
	}

	pub fn step_thickness(&mut self, direction: StepDirection) {
		self.dimensions.thickness = (self.dimensions.thickness
			+ direction.sign() * THICKNESS_STEP_METRES)
			.max(MINIMUM_THICKNESS_METRES);
	}
}

/// The gesture in progress, as a Bevy resource.
#[derive(Resource, Default, Deref, DerefMut, PartialEq)]
struct WallDraftState(WallDraft);

#[derive(SystemParam)]
struct DrawingInput<'w> {
	config: Res<'w, InteractionConfig>,
	actions: Res<'w, ActionState>,
	ground_pointer: Res<'w, GroundPointer>,
	snapped_pointer: Res<'w, SnappedPointer>,
}

fn apply_wall_settings(
	mut actions: MessageReader<EditorAction>,
	mut settings: ResMut<WallDrawingSettings>,
) {
	for action in actions.read().filter_map(|action| action.settings()) {
		match action {
			SettingsAction::StepWallHeight(direction) => settings.step_height(direction),
			SettingsAction::StepWallThickness(direction) => settings.step_thickness(direction),
			_ => {}
		}
	}
}

fn update_wall_drawing(
	input: DrawingInput,
	settings: Res<WallDrawingSettings>,
	mut house: ResMut<House>,
	mut notice: ResMut<EditNotice>,
	mut draft: ResMut<WallDraftState>,
	mut preview: ResMut<EditPreview>,
) {
	if input.actions.just_pressed(Action::Cancel) {
		draft.set_if_neq(WallDraftState(WallDraft::Idle));
		if preview.holds_wall_proposal() {
			preview.state = None;
		}
		return;
	}

	let (next_draft, action) = advance_draft(
		&input.config,
		draft.0,
		&input.actions,
		input.ground_pointer.hit,
		input.snapped_pointer.target,
	);
	draft.set_if_neq(WallDraftState(next_draft));

	// Reborrowing retains Bevy's change tracking until the helper actually writes.
	update_preview(
		next_draft,
		&house,
		&input.snapped_pointer,
		&settings,
		preview.reborrow(),
	);

	if action == DraftAction::Confirm
		&& let Some(Proposal::Wall(proposal)) = preview.state.as_ref().map(|state| state.proposal)
		&& let Some(candidate) = preview.take_valid(Proposal::Wall(proposal), house.revision())
	{
		house.commit(candidate);
		notice.message = None;
		draft.0 = WallDraft::Idle;
	}
}

/// A wall started on the house before an undo or redo would end on a different house.
fn discard_wall_draft_after_restore(
	mut restored: MessageReader<DocumentRestored>,
	mut draft: ResMut<WallDraftState>,
) {
	if restored.read().count() > 0 {
		draft.set_if_neq(WallDraftState(WallDraft::Idle));
	}
}

/// Leaving the tool abandons an unfinished wall instead of resuming it on return.
fn cancel_wall_drawing(mut draft: ResMut<WallDraftState>, mut preview: ResMut<EditPreview>) {
	draft.set_if_neq(WallDraftState(WallDraft::Idle));
	if preview.holds_wall_proposal() {
		preview.state = None;
	}
}

fn update_preview(
	draft: WallDraft,
	house: &House,
	snapped_pointer: &SnappedPointer,
	settings: &WallDrawingSettings,
	mut preview: Mut<EditPreview>,
) {
	let previous = match preview.state.as_ref() {
		Some(PreviewState {
			proposal: Proposal::Wall(previous),
			..
		}) => Some(*previous),
		_ => None,
	};
	match wall_preview(
		draft,
		snapped_pointer.target,
		previous,
		settings.dimensions(),
	) {
		WallPreview::Clear => {
			if preview.holds_wall_proposal() {
				preview.state = None;
			}
		}
		WallPreview::Keep => {}
		WallPreview::Propose(proposal) => EditPreview::update(
			preview,
			Proposal::Wall(proposal),
			house.revision(),
			house.committed(),
		),
	}
}

#[cfg(test)]
#[path = "../tests/tools/drawing.rs"]
mod tests;
