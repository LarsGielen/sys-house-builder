use super::*;
use crate::editor::input::{ActionGroup, signal::AxisMode};

#[derive(Clone, Copy)]
enum TestAction {
	Undo,
	Redo,
	Look,
	Zoom,
}

impl Action for TestAction {
	fn name(self) -> &'static str {
		match self {
			Self::Undo => "test.undo",
			Self::Redo => "test.redo",
			Self::Look => "test.look",
			Self::Zoom => "test.zoom",
		}
	}

	fn value(self) -> ValueKind {
		match self {
			Self::Look => ValueKind::Axis2D(AxisMode::Relative),
			Self::Zoom => ValueKind::Axis1D(AxisMode::Relative),
			_ => ValueKind::Button,
		}
	}
}

impl ActionGroup for TestAction {
	fn all() -> &'static [Self] {
		&[Self::Undo, Self::Redo, Self::Look, Self::Zoom]
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
	assert_eq!(keymap.button_bindings().len(), 3);
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

#[test]
fn rejects_a_shortcut_bound_to_an_action_that_is_not_a_button() {
	let text = "[bindings]\n\"test.look\" = [\"ctrl+l\"]";
	let error = Keymap::parse(text, &registered_actions()).err().unwrap();

	assert!(error.contains("test.look"), "{error}");
}

fn parse_error(text: &str) -> String {
	Keymap::parse(text, &registered_actions()).err().unwrap()
}

#[test]
fn parses_axis_inputs_into_axis_bindings() {
	let text = "[bindings]\n\"test.undo\" = [\"ctrl+z\"]\n\"test.look\" = [\"mouse_motion\"]\n\"test.zoom\" = [\"scroll\"]";
	let keymap = Keymap::parse(text, &registered_actions()).unwrap();

	assert_eq!(keymap.button_bindings().len(), 1);
	assert_eq!(
		keymap.axis_bindings(),
		&[
			(AxisInput::MouseMotion, "test.look"),
			(AxisInput::Scroll, "test.zoom")
		]
	);
}

#[test]
fn rejects_an_axis_input_whose_kind_does_not_match_the_action() {
	let on_button = parse_error("[bindings]\n\"test.undo\" = [\"mouse_motion\"]");
	assert!(on_button.contains("test.undo"), "{on_button}");

	let scroll_on_2d = parse_error("[bindings]\n\"test.look\" = [\"scroll\"]");
	assert!(scroll_on_2d.contains("test.look"), "{scroll_on_2d}");

	let motion_on_1d = parse_error("[bindings]\n\"test.zoom\" = [\"mouse_motion\"]");
	assert!(motion_on_1d.contains("test.zoom"), "{motion_on_1d}");
}

#[test]
fn rejects_modifiers_on_axis_inputs() {
	let error = parse_error("[bindings]\n\"test.zoom\" = [\"shift+scroll\"]");

	assert!(error.contains("modifiers"), "{error}");
}

#[test]
fn rejects_an_axis_input_bound_twice() {
	let error = parse_error("[bindings]\n\"test.zoom\" = [\"scroll\", \"scroll\"]");

	assert!(error.contains("bound to both"), "{error}");
}
