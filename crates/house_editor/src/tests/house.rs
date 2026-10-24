use super::*;
use crate::{
	edit::{OpeningProposal, WallProposal, propose_opening, propose_wall},
	opening_category::OpeningCategory,
};
use bevy::prelude::Vec2;
use wall_graph::{OpeningSpec, WallDimensions};

fn wall_proposal(end: Vec2) -> WallProposal {
	WallProposal {
		start: Vec2::ZERO,
		end,
		dimensions: WallDimensions::default(),
	}
}

fn commit_wall(house: &mut House, end: Vec2) {
	let candidate = propose_wall(house.committed(), &wall_proposal(end)).unwrap();
	house.commit(candidate);
}

fn commit_window(house: &mut House) {
	let wall = house.graph().walls().next().unwrap();
	let proposal = OpeningProposal {
		wall,
		spec: OpeningSpec {
			center_distance: 2.0,
			width: 0.9,
			bottom: 0.9,
			height: 1.2,
		},
		category: OpeningCategory::Window,
	};
	house.commit(propose_opening(house.committed(), &proposal).unwrap());
}

#[test]
fn each_commit_is_one_undo_step() {
	let mut house = House::default();
	assert!(!house.can_undo());
	commit_wall(&mut house, Vec2::new(4.0, 0.0));
	commit_window(&mut house);

	assert!(house.undo());
	assert!(house.undo());
	assert!(!house.undo());
	assert_eq!(house.graph().walls().count(), 0);
}

#[test]
fn undo_and_redo_restore_graph_and_categories_together() {
	let mut house = House::default();
	commit_wall(&mut house, Vec2::new(4.0, 0.0));
	commit_window(&mut house);
	let opening = house.graph().openings().next().unwrap().0;

	assert!(house.undo());
	assert!(house.graph().opening(opening).is_none());
	assert_eq!(house.committed().categories().get(opening), None);
	assert!(!house.mesh().is_empty());

	assert!(house.redo());
	assert!(house.graph().opening(opening).is_some());
	assert_eq!(
		house.committed().categories().get(opening),
		Some(OpeningCategory::Window)
	);
}

#[test]
fn restoring_replaces_the_scene_like_a_commit() {
	let mut house = House::default();
	commit_wall(&mut house, Vec2::new(4.0, 0.0));
	let revision = house.revision();
	house.undo();
	assert!(house.revision() > revision);
	let revision = house.revision();
	house.redo();
	assert!(house.revision() > revision);
}

#[test]
fn unavailable_undo_and_redo_change_nothing() {
	let mut house = House::default();
	assert!(!house.undo());
	assert!(!house.redo());
	assert_eq!(house.revision(), 0);
}

#[test]
fn a_commit_after_undo_discards_the_redo_branch() {
	let mut house = House::default();
	commit_wall(&mut house, Vec2::new(4.0, 0.0));
	commit_window(&mut house);
	house.undo();
	assert!(house.can_redo());

	commit_wall(&mut house, Vec2::new(0.0, 3.0));
	assert!(!house.can_redo());
	assert!(!house.redo());
	assert!(house.graph().openings().next().is_none());
}

#[test]
fn a_new_house_is_unmodified() {
	assert!(!House::default().is_modified());
}

#[test]
fn modified_follows_edits_saves_undo_and_redo() {
	let mut house = House::default();
	commit_wall(&mut house, Vec2::new(4.0, 0.0));
	assert!(house.is_modified());

	house.mark_saved();
	assert!(!house.is_modified());

	commit_window(&mut house);
	assert!(house.is_modified());
	house.undo();
	assert!(!house.is_modified());
	house.redo();
	assert!(house.is_modified());
	house.undo();
	house.undo();
	assert!(house.is_modified());
	house.redo();
	assert!(!house.is_modified());
}

#[test]
fn a_new_edit_at_an_old_position_is_not_the_saved_document() {
	let mut house = House::default();
	commit_wall(&mut house, Vec2::new(4.0, 0.0));
	commit_window(&mut house);
	house.mark_saved();

	house.undo();
	// The same number of steps from the start as the saved document, but different content.
	commit_wall(&mut house, Vec2::new(0.0, 3.0));
	assert!(house.is_modified());
	assert!(!house.redo());
	house.undo();
	assert!(house.is_modified());
}

#[test]
fn undoing_a_drag_restores_connected_walls_and_openings_together() {
	let mut house = House::default();
	commit_wall(&mut house, Vec2::new(4.0, 0.0));
	commit_window(&mut house);
	let corner = house.graph().walls().next().unwrap().destination();
	let other_wall = propose_wall(
		house.committed(),
		&WallProposal {
			start: Vec2::new(4.0, 0.0),
			end: Vec2::new(4.0, 3.0),
			dimensions: WallDimensions::default(),
		},
	)
	.unwrap();
	house.commit(other_wall);
	let opening = house.graph().openings().next().unwrap().0;
	let before_spec = house.graph().opening(opening).unwrap().spec;

	let drag = crate::edit::propose_move(
		house.committed(),
		&crate::edit::MoveProposal {
			first: (corner, Vec2::new(5.0, 1.0)),
			second: None,
			grabbed: None,
		},
	)
	.unwrap();
	house.commit(drag);
	assert_eq!(
		house.graph().node_position(corner),
		Some(Vec2::new(5.0, 1.0))
	);
	assert_ne!(house.graph().opening(opening).unwrap().spec, before_spec);

	assert!(house.undo());
	assert_eq!(
		house.graph().node_position(corner),
		Some(Vec2::new(4.0, 0.0))
	);
	assert_eq!(house.graph().opening(opening).unwrap().spec, before_spec);
	assert_eq!(
		house.committed().categories().get(opening),
		Some(OpeningCategory::Window)
	);
	assert!(house.redo());
	assert_eq!(
		house.graph().node_position(corner),
		Some(Vec2::new(5.0, 1.0))
	);
}
