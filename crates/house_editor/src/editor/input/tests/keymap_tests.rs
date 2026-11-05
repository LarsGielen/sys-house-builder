use super::*;
use crate::editor::input::{ActionGroup, modifiers::Modifiers, signal::AxisMode};

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
			AxisBinding {
				input: AxisInput::MouseMotion,
				modifiers: Modifiers::default(),
				scale: Scale::Two([1.0, 1.0]),
				action: "test.look"
			},
			AxisBinding {
				input: AxisInput::Scroll,
				modifiers: Modifiers::default(),
				scale: Scale::One(1.0),
				action: "test.zoom"
			}
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
fn parses_modifiers_on_axis_inputs() {
	let text = "[bindings]\n\"test.look\" = [\"ctrl+alt+mouse_motion\"]\n\"test.zoom\" = [\"Shift+scroll\"]";
	let keymap = Keymap::parse(text, &registered_actions()).unwrap();

	let modifiers: Vec<Modifiers> = keymap
		.axis_bindings()
		.iter()
		.map(|binding| binding.modifiers)
		.collect();
	assert_eq!(
		modifiers,
		["ctrl+alt".parse().unwrap(), "shift".parse().unwrap()]
	);
}

#[test]
fn rejects_unknown_and_repeated_modifiers_on_axis_inputs() {
	let unknown = parse_error("[bindings]\n\"test.zoom\" = [\"meta+scroll\"]");
	assert!(unknown.contains("meta"), "{unknown}");

	let repeated = parse_error("[bindings]\n\"test.zoom\" = [\"shift+shift+scroll\"]");
	assert!(repeated.contains("repeated"), "{repeated}");
}

#[test]
fn the_same_axis_input_can_be_bound_with_different_modifiers() {
	let text = "[bindings]\n\"test.zoom\" = [\"scroll\", \"shift+scroll\"]";
	let keymap = Keymap::parse(text, &registered_actions()).unwrap();

	assert_eq!(keymap.axis_bindings().len(), 2);
}

#[test]
fn rejects_the_same_axis_input_with_the_same_modifiers_twice() {
	let error = parse_error("[bindings]\n\"test.zoom\" = [\"shift+scroll\", \"Shift+Scroll\"]");

	assert!(error.contains("bound to both"), "{error}");
}

#[test]
fn rejects_an_axis_input_bound_twice() {
	let error = parse_error("[bindings]\n\"test.zoom\" = [\"scroll\", \"scroll\"]");

	assert!(error.contains("bound to both"), "{error}");
}

#[test]
fn a_table_entry_without_options_equals_a_plain_entry() {
	let plain = "[bindings]\n\"test.undo\" = [\"ctrl+z\"]\n\"test.zoom\" = [\"scroll\"]";
	let table = "[bindings]\n\"test.undo\" = [{ input = \"ctrl+z\" }]\n\"test.zoom\" = [{ input = \"scroll\" }]";
	let plain = Keymap::parse(plain, &registered_actions()).unwrap();
	let table = Keymap::parse(table, &registered_actions()).unwrap();

	assert_eq!(plain.button_bindings(), table.button_bindings());
	assert_eq!(plain.axis_bindings(), table.axis_bindings());
}

#[test]
fn plain_and_table_entries_can_be_mixed_in_one_list() {
	let text = "[bindings]\n\"test.redo\" = [\"ctrl+y\", { input = \"ctrl+shift+z\" }]";
	let keymap = Keymap::parse(text, &registered_actions()).unwrap();

	assert_eq!(keymap.button_bindings().len(), 2);
}

#[test]
fn rejects_unknown_table_fields_and_a_missing_input() {
	let misspelled = "[bindings]\n\"test.zoom\" = [{ input = \"scroll\", scal = 2.0 }]";
	assert!(Keymap::parse(misspelled, &registered_actions()).is_err());

	let no_input = "[bindings]\n\"test.zoom\" = [{ scale = 2.0 }]";
	assert!(Keymap::parse(no_input, &registered_actions()).is_err());
}

#[test]
fn parses_scales_for_both_axis_shapes() {
	let text = "[bindings]\n\"test.look\" = [{ input = \"mouse_motion\", scale = [0.2, -0.2] }]\n\"test.zoom\" = [{ input = \"scroll\", scale = -1.0 }]";
	let keymap = Keymap::parse(text, &registered_actions()).unwrap();

	let scales: Vec<Scale> = keymap
		.axis_bindings()
		.iter()
		.map(|binding| binding.scale)
		.collect();
	assert_eq!(scales, [Scale::Two([0.2, -0.2]), Scale::One(-1.0)]);
}

#[test]
fn rejects_a_scale_that_does_not_fit_the_action() {
	let pair_on_1d =
		parse_error("[bindings]\n\"test.zoom\" = [{ input = \"scroll\", scale = [1.0, 2.0] }]");
	assert!(pair_on_1d.contains("test.zoom"), "{pair_on_1d}");

	let number_on_2d =
		parse_error("[bindings]\n\"test.look\" = [{ input = \"mouse_motion\", scale = 2.0 }]");
	assert!(number_on_2d.contains("test.look"), "{number_on_2d}");

	let on_shortcut =
		parse_error("[bindings]\n\"test.undo\" = [{ input = \"ctrl+z\", scale = 2.0 }]");
	assert!(on_shortcut.contains("scale"), "{on_shortcut}");
}

#[test]
fn rejects_scales_that_would_silence_or_break_the_binding() {
	for scale in ["0.0", "[1.0, 0.0]"] {
		let input = if scale.starts_with('[') {
			"mouse_motion"
		} else {
			"scroll"
		};
		let action = if scale.starts_with('[') {
			"test.look"
		} else {
			"test.zoom"
		};
		let text =
			format!("[bindings]\n\"{action}\" = [{{ input = \"{input}\", scale = {scale} }}]");

		assert!(
			Keymap::parse(&text, &registered_actions()).is_err(),
			"{scale}"
		);
	}

	let not_a_number = "[bindings]\n\"test.zoom\" = [{ input = \"scroll\", scale = nan }]";
	assert!(Keymap::parse(not_a_number, &registered_actions()).is_err());
}
