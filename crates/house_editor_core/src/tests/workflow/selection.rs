use super::*;
use glam::Vec2;
use house_document::{EditCandidate, WallProposal, propose_wall};
use wall_graph::WallDimensions;

fn one_wall() -> (EditCandidate, Wall) {
	let candidate = propose_wall(
		&EditCandidate::default(),
		&WallProposal {
			start: Vec2::ZERO,
			end: Vec2::X,
			dimensions: WallDimensions::default(),
		},
	)
	.unwrap();
	let wall = candidate.graph().walls().next().unwrap();
	(candidate, wall)
}

#[test]
fn a_carried_wall_is_kept_only_while_it_exists() {
	let (candidate, wall) = one_wall();
	assert_eq!(
		retained_selection(candidate.graph(), Some(wall), None),
		Some(SelectionTarget::Wall(wall))
	);
	let empty = EditCandidate::default();
	assert_eq!(retained_selection(empty.graph(), Some(wall), None), None);
}

#[test]
fn nothing_is_kept_unless_the_edit_carried_it() {
	let (candidate, _) = one_wall();
	assert_eq!(retained_selection(candidate.graph(), None, None), None);
}

#[test]
fn wall_drafts_go_stale_with_the_tool_the_selection_or_the_house() {
	let (_, wall) = one_wall();
	assert!(!wall_draft_is_stale(Tool::Select, Some(wall), wall, 3, 3));
	assert!(wall_draft_is_stale(Tool::Select, None, wall, 3, 3));
	assert!(wall_draft_is_stale(Tool::Select, Some(wall), wall, 3, 4));
	assert!(!wall_draft_is_stale(Tool::Delete, None, wall, 3, 3));
	assert!(wall_draft_is_stale(Tool::DrawWall, Some(wall), wall, 3, 3));
}

#[test]
fn cancelling_wins_over_confirming_and_invalid_deletions_wait() {
	assert_eq!(deletion_step(true, true, true), DeletionStep::Abandon);
	assert_eq!(deletion_step(false, true, true), DeletionStep::Commit);
	assert_eq!(deletion_step(false, true, false), DeletionStep::Wait);
	assert_eq!(deletion_step(false, false, true), DeletionStep::Wait);
}

#[test]
fn only_removal_proposals_name_a_removal_target() {
	let (_, wall) = one_wall();
	assert_eq!(
		removal_target(Proposal::WallRemoval(wall)),
		Some(SelectionTarget::Wall(wall))
	);
}
