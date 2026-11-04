use super::*;
use crate::editor::input::{ActionGroup, ValueKind};

#[derive(Clone, Copy)]
enum TestAction {
	Undo,
	Redo,
}

impl Action for TestAction {
	fn name(self) -> &'static str {
		match self {
			Self::Undo => "test.undo",
			Self::Redo => "test.redo",
		}
	}

	fn value(self) -> ValueKind {
		ValueKind::Button
	}
}

impl ActionGroup for TestAction {
	fn all() -> &'static [Self] {
		&[Self::Undo, Self::Redo]
	}
}

fn registered_actions() -> ActionState {
	let mut actions = ActionState::default();
	actions.register_group::<TestAction>();
	actions
}

#[test]
fn finds_all_shortcuts_of_an_action() {
	let text =
		"[bindings]\n\"test.undo\" = [\"ctrl+z\"]\n\"test.redo\" = [\"ctrl+shift+z\", \"ctrl+y\"]";
	let keymap = Keymap::parse(text, &registered_actions()).unwrap();

	assert_eq!(keymap.shortcuts_for(TestAction::Redo).count(), 2);
	assert_eq!(keymap.shortcuts_for(TestAction::Undo).count(), 1);
	assert_eq!(keymap.bindings().len(), 3);
}

#[test]
fn an_action_without_bindings_has_no_shortcuts() {
	let text = "[bindings]\n\"test.undo\" = [\"ctrl+z\"]";
	let keymap = Keymap::parse(text, &registered_actions()).unwrap();

	assert_eq!(keymap.shortcuts_for(TestAction::Redo).count(), 0);
}

#[test]
fn rejects_actions_that_are_not_registered() {
	let unknown_action = "[bindings]\n\"test.explode\" = [\"ctrl+x\"]";
	assert!(Keymap::parse(unknown_action, &registered_actions()).is_err());

	let no_actions_registered = "[bindings]\n\"test.undo\" = [\"ctrl+z\"]";
	assert!(Keymap::parse(no_actions_registered, &ActionState::default()).is_err());
}

#[test]
fn rejects_a_shortcut_bound_to_two_actions() {
	let conflict = "[bindings]\n\"test.undo\" = [\"ctrl+z\"]\n\"test.redo\" = [\"ctrl+z\"]";
	assert!(Keymap::parse(conflict, &registered_actions()).is_err());
}

#[test]
fn rejects_invalid_shortcuts_without_returning_a_partial_keymap() {
	let text = "[bindings]\n\"test.undo\" = [\"ctrl+z\", \"meta+z\"]";
	assert!(Keymap::parse(text, &registered_actions()).is_err());
}
