use super::*;
use crate::tests::support::house_with_wall;
use bevy::prelude::Vec2;

#[test]
fn only_a_modified_house_needs_confirmation() {
	let mut house = House::default();
	assert!(!needs_confirmation(&house));
	let (modified, _) = house_with_wall(Vec2::ZERO, Vec2::new(4.0, 0.0));
	assert!(needs_confirmation(&modified));
	house = modified;
	house.mark_saved();
	assert!(!needs_confirmation(&house));
}

#[test]
fn cancelling_keeps_the_document_and_its_history() {
	let (mut house, _) = house_with_wall(Vec2::ZERO, Vec2::new(4.0, 0.0));
	house.undo();
	house.redo();
	let revision = house.revision();

	assert_eq!(UnsavedChoice::Cancel.step(), ReplacementStep::Keep);
	assert_eq!(house.revision(), revision);
	assert!(house.can_undo());
	assert!(house.is_modified());
}

#[test]
fn saving_or_discarding_continues_with_the_replacement() {
	assert_eq!(UnsavedChoice::Save.step(), ReplacementStep::SaveThenReplace);
	assert_eq!(UnsavedChoice::Discard.step(), ReplacementStep::Replace);
}
