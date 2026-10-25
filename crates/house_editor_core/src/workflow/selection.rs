use house_document::Proposal;
use wall_graph::{OpeningId, Wall, WallGraph};

use crate::pointing::picking::SelectionTarget;

/// What a left click in the viewport does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tool {
	#[default]
	Select,
	DrawWall,
	DrawArc,
	PlaceOpening,
	Delete,
}

/// The selection that survives a commit: the wall the edit explicitly carried over, or else
/// the selected opening if it still exists. Anything else is cleared, because handles and ids
/// from before an edit may no longer mean the same thing.
pub fn retained_selection(
	graph: &WallGraph,
	carried_wall: Option<Wall>,
	selected_opening: Option<OpeningId>,
) -> Option<SelectionTarget> {
	carried_wall
		.filter(|&wall| graph.wall_dimensions(wall).is_some())
		.map(SelectionTarget::Wall)
		.or_else(|| {
			selected_opening
				.filter(|&opening| graph.opening(opening).is_some())
				.map(SelectionTarget::Opening)
		})
}

/// Whether a draft about `wall` has outlived what it was built on. The delete tool previews
/// whatever is under the pointer, so selection does not apply to it; any tool that does not
/// edit walls discards the draft.
pub fn wall_draft_is_stale(
	tool: Tool,
	selected_wall: Option<Wall>,
	wall: Wall,
	draft_revision: u64,
	house_revision: u64,
) -> bool {
	let outdated = match tool {
		Tool::Select => selected_wall != Some(wall),
		Tool::Delete => false,
		_ => true,
	};
	outdated || draft_revision != house_revision
}

/// Whether an opening draft has outlived what it was built on.
pub fn opening_draft_is_stale(
	tool: Tool,
	selected_opening: Option<OpeningId>,
	opening: OpeningId,
	draft_revision: u64,
	house_revision: u64,
) -> bool {
	tool != Tool::Select || selected_opening != Some(opening) || draft_revision != house_revision
}

/// What to do about a previewed wall deletion this frame.
#[derive(Debug, PartialEq, Eq)]
pub enum DeletionStep {
	Wait,
	Abandon,
	Commit,
}

/// Cancelling wins over confirming in the same frame. An invalid deletion keeps its preview,
/// which already shows the reason.
pub fn deletion_step(cancelled: bool, confirmed: bool, preview_is_valid: bool) -> DeletionStep {
	if cancelled {
		DeletionStep::Abandon
	} else if confirmed && preview_is_valid {
		DeletionStep::Commit
	} else {
		DeletionStep::Wait
	}
}

/// The proposal a held deletion preview is for.
pub fn removal_target(proposal: Proposal) -> Option<SelectionTarget> {
	match proposal {
		Proposal::WallRemoval(wall) => Some(SelectionTarget::Wall(wall)),
		Proposal::OpeningRemoval(opening) => Some(SelectionTarget::Opening(opening)),
		_ => None,
	}
}

#[cfg(test)]
#[path = "../tests/workflow/selection.rs"]
mod tests;
