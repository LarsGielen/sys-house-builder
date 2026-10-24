use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use wall_graph::WallDimensions;

use crate::{
	edit::{EditNotice, EditPreview, PreviewState, Proposal, WallProposal},
	ground_pointer::GroundPointer,
	house::House,
	snapping::SnappedPointer,
	wall_tool_active::{Tool, tool_active},
};

const DRAG_THRESHOLD_PIXELS: f32 = 6.0;
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
		app.init_resource::<WallDrawingSettings>()
			.init_resource::<WallDraft>()
			.init_resource::<EditPreview>()
			.init_resource::<EditNotice>()
			.add_systems(
				PostUpdate,
				(
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

#[derive(Clone, Copy)]
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

/// Where the current gesture is. Positions are graph metres unless noted.
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq)]
enum WallDraft {
	#[default]
	Idle,
	/// The button is down. It is a click until the pointer moves past the drag threshold.
	Pressed {
		start: Vec2,
		/// Viewport pixels, to measure the drag threshold.
		press_position: Vec2,
		dragged: bool,
	},
	/// A click placed the start; the next click confirms the end.
	AwaitingEnd { start: Vec2 },
}

impl WallDraft {
	fn start(self) -> Option<Vec2> {
		match self {
			WallDraft::Idle => None,
			WallDraft::Pressed { start, .. } | WallDraft::AwaitingEnd { start } => Some(start),
		}
	}
}

/// Whether a draft step asks to commit the preview.
#[derive(Debug, PartialEq)]
enum DraftAction {
	Continue,
	Confirm,
}

#[derive(SystemParam)]
struct DrawingInput<'w> {
	mouse: Res<'w, ButtonInput<MouseButton>>,
	keyboard: Res<'w, ButtonInput<KeyCode>>,
	ground_pointer: Res<'w, GroundPointer>,
	snapped_pointer: Res<'w, SnappedPointer>,
}

fn update_wall_drawing(
	input: DrawingInput,
	settings: Res<WallDrawingSettings>,
	mut house: ResMut<House>,
	mut notice: ResMut<EditNotice>,
	mut draft: ResMut<WallDraft>,
	mut preview: ResMut<EditPreview>,
) {
	if input.keyboard.just_pressed(KeyCode::Escape) {
		draft.set_if_neq(WallDraft::Idle);
		if preview.holds_wall_proposal() {
			preview.state = None;
		}
		return;
	}

	let (next_draft, action) = advance_draft(
		*draft,
		&input.mouse,
		&input.ground_pointer,
		&input.snapped_pointer,
	);
	draft.set_if_neq(next_draft);

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
		*draft = WallDraft::Idle;
	}
}

/// Leaving the tool abandons an unfinished wall instead of resuming it on return.
fn cancel_wall_drawing(mut draft: ResMut<WallDraft>, mut preview: ResMut<EditPreview>) {
	draft.set_if_neq(WallDraft::Idle);
	if preview.holds_wall_proposal() {
		preview.state = None;
	}
}

fn advance_draft(
	draft: WallDraft,
	mouse: &ButtonInput<MouseButton>,
	ground_pointer: &GroundPointer,
	snapped_pointer: &SnappedPointer,
) -> (WallDraft, DraftAction) {
	match draft {
		WallDraft::Idle => {
			let started = mouse
				.just_pressed(MouseButton::Left)
				.then(|| Some((ground_pointer.hit?, snapped_pointer.target?)))
				.flatten();
			match started {
				Some((hit, target)) => (
					WallDraft::Pressed {
						start: target.position,
						press_position: hit.viewport_position,
						dragged: false,
					},
					DraftAction::Continue,
				),
				None => (draft, DraftAction::Continue),
			}
		}
		WallDraft::Pressed {
			start,
			press_position,
			dragged,
		} => {
			let dragged = dragged
				|| ground_pointer.hit.is_some_and(|hit| {
					hit.viewport_position.distance(press_position) > DRAG_THRESHOLD_PIXELS
				});
			// The release is handled even when the pointer is over the panel, so a drag
			// that ends there still finishes.
			if !mouse.just_released(MouseButton::Left) {
				return (
					WallDraft::Pressed {
						start,
						press_position,
						dragged,
					},
					DraftAction::Continue,
				);
			}
			if dragged {
				(WallDraft::AwaitingEnd { start }, DraftAction::Confirm)
			} else {
				(WallDraft::AwaitingEnd { start }, DraftAction::Continue)
			}
		}
		WallDraft::AwaitingEnd { .. } => {
			let confirmed = mouse.just_pressed(MouseButton::Left) && ground_pointer.hit.is_some();
			let action = if confirmed {
				DraftAction::Confirm
			} else {
				DraftAction::Continue
			};
			(draft, action)
		}
	}
}

fn update_preview(
	draft: WallDraft,
	house: &House,
	snapped_pointer: &SnappedPointer,
	settings: &WallDrawingSettings,
	mut preview: Mut<EditPreview>,
) {
	let Some(start) = draft.start() else {
		if preview.holds_wall_proposal() {
			preview.state = None;
		}
		return;
	};
	// Until the pointer leaves the click threshold there is no end worth previewing.
	if matches!(draft, WallDraft::Pressed { dragged: false, .. }) {
		if preview.holds_wall_proposal() {
			preview.state = None;
		}
		return;
	}
	// Keep the last endpoint while off the ground, but refresh it if the house changed.
	let proposal = if let Some(target) = snapped_pointer.target {
		WallProposal {
			start,
			end: target.position,
			dimensions: settings.dimensions(),
		}
	} else {
		let Some(PreviewState {
			proposal: Proposal::Wall(previous),
			..
		}) = preview.state.as_ref()
		else {
			return;
		};
		WallProposal {
			start,
			dimensions: settings.dimensions(),
			..*previous
		}
	};
	EditPreview::update(
		preview,
		Proposal::Wall(proposal),
		house.revision(),
		house.committed(),
	);
}

#[cfg(test)]
#[path = "tests/wall_tool_drawing.rs"]
mod tests;
