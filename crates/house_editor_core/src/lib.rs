//! Engine-independent editing behaviour built on `house_document`: the semantic actions a host
//! can send, pointer and snapping policy, opening placement queries, and the decision of when
//! unsaved work needs confirming.
//!
//! A host (a Bevy app, a test, another tool) maps its own devices and widgets to
//! [`Action`]s and its own camera to [`PointerOnGround`] samples. Nothing here reads a key,
//! a button, a window, or a screen.

mod action;
mod gestures;
mod interaction_settings;
mod openings;
mod pointing;
mod workflow;

pub use action::{Action, ActionState};
pub use gestures::arc_drawing::{
	ArcDraft, ArcMode, ArcStep, NoticeChange, advance_arc_draft, arc_proposal,
	free_end_continuation,
};
pub use gestures::handle_drag::{
	ActiveDrag, DragPointer, HANDLE_LIFT_METRES, HandleDrag, HandleStep, WallHandle,
	WallHandlePositions, advance_handle_drag, drag_proposal, handle_positions, handle_under_ray,
};
pub use gestures::opening_drag::{OpeningDrag, advance_opening_drag, opening_drag_proposal};
pub use gestures::wall_drawing::{
	DraftAction, WallDraft, WallPreview, advance_draft, wall_preview,
};
pub use interaction_settings::{InteractionSettings, InvalidInteractionSetting};
pub use openings::area::{OpeningArea, closest_distance_along_wall, opening_area};
pub use openings::placement::{OpeningDefaults, propose_opening_at_ray};
pub use openings::snap::{OpeningSnapOptions, snap_opening};
pub use pointing::picking::{SelectionTarget, WallHit, pick_target, pick_wall_hit};
pub use pointing::pointer::PointerOnGround;
pub use pointing::snapping::{
	InvalidOpeningGap, SnapSettings, SnapSource, SnapTarget, choose_snap_target, snap_to_grid,
};
pub use workflow::preview::{Preview, PreviewState};
pub use workflow::project::{ProjectCommand, ProjectEffect, ProjectWorkflow};
pub use workflow::selection::{
	DeletionStep, Tool, deletion_step, opening_draft_is_stale, removal_target, retained_selection,
	wall_draft_is_stale,
};
pub use workflow::unsaved_changes::{ReplacementStep, UnsavedChoice, needs_confirmation};
