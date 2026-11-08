use super::*;
use crate::core::input::{
	Action, ActionGroup, ActiveContexts, Availability, Block, InputContext, InputContexts,
	Interpretation, Modifier, ShortcutInput, modifiers::Modifiers, signal::AxisMode,
};

#[derive(Clone, Copy)]
enum TestAction {
	Undo,
	Redo,
	Look,
	Zoom,
	Move,
	Rise,
	Snap,
}

impl Action for TestAction {
	fn name(self) -> &'static str {
		match self {
			Self::Undo => "test.undo",
			Self::Redo => "test.redo",
			Self::Look => "test.look",
			Self::Zoom => "test.zoom",
			Self::Move => "test.move",
			Self::Rise => "test.rise",
			Self::Snap => "test.snap",
		}
	}

	fn value(self) -> ValueKind {
		match self {
			Self::Look => ValueKind::Axis2D(AxisMode::Relative),
			Self::Zoom => ValueKind::Axis1D(AxisMode::Relative),
			Self::Move => ValueKind::Axis2D(AxisMode::Absolute),
			Self::Rise => ValueKind::Axis1D(AxisMode::Absolute),
			Self::Snap => ValueKind::Toggle,
			_ => ValueKind::Button,
		}
	}
}

impl ActionGroup for TestAction {
	fn all() -> &'static [Self] {
		&[
			Self::Undo,
			Self::Redo,
			Self::Look,
			Self::Zoom,
			Self::Move,
			Self::Rise,
			Self::Snap,
		]
	}
}

fn registered_contexts() -> InputContexts {
	let mut contexts = InputContexts::default();
	contexts
		.register(InputContext::new("global").priority(0))
		.unwrap();
	contexts
		.register(InputContext::new("viewport").priority(10))
		.unwrap();
	contexts
}

fn registered_actions() -> ActionState {
	let mut actions = ActionState::default();
	actions.register_group::<TestAction>();
	actions
}

#[test]
fn finds_all_shortcuts_of_an_action() {
	let text = "[contexts.global.bindings]\n\"test.undo\" = [\"ctrl+z\"]\n\"test.redo\" = [\"ctrl+shift+z\", \"ctrl+y\"]";
	let keymap = Keymap::parse(text, &registered_actions(), &registered_contexts()).unwrap();

	assert_eq!(
		keymap
			.hints_for(TestAction::Redo.name(), &ActiveContexts::default())
			.len(),
		2
	);
	assert_eq!(
		keymap
			.hints_for(TestAction::Undo.name(), &ActiveContexts::default())
			.len(),
		1
	);
	assert_eq!(keymap.button_bindings().len(), 3);
}

#[test]
fn an_action_without_bindings_has_no_shortcuts() {
	let text = "[contexts.global.bindings]\n\"test.undo\" = [\"ctrl+z\"]";
	let keymap = Keymap::parse(text, &registered_actions(), &registered_contexts()).unwrap();

	assert_eq!(
		keymap
			.hints_for(TestAction::Redo.name(), &ActiveContexts::default())
			.len(),
		0
	);
}

#[test]
fn rejects_actions_that_are_not_registered() {
	let unknown_action = "[contexts.global.bindings]\n\"test.explode\" = [\"ctrl+x\"]";
	assert!(
		Keymap::parse(
			unknown_action,
			&registered_actions(),
			&registered_contexts()
		)
		.is_err()
	);

	let no_actions_registered = "[contexts.global.bindings]\n\"test.undo\" = [\"ctrl+z\"]";
	assert!(
		Keymap::parse(
			no_actions_registered,
			&ActionState::default(),
			&registered_contexts()
		)
		.is_err()
	);
}

#[test]
fn rejects_a_shortcut_bound_to_two_actions() {
	let conflict =
		"[contexts.global.bindings]\n\"test.undo\" = [\"ctrl+z\"]\n\"test.redo\" = [\"ctrl+z\"]";
	assert!(Keymap::parse(conflict, &registered_actions(), &registered_contexts()).is_err());
}

#[test]
fn rejects_invalid_shortcuts_without_returning_a_partial_keymap() {
	let text = "[contexts.global.bindings]\n\"test.undo\" = [\"ctrl+z\", \"meta+z\"]";
	assert!(Keymap::parse(text, &registered_actions(), &registered_contexts()).is_err());
}

#[test]
fn rejects_a_shortcut_bound_to_an_action_that_is_not_a_button() {
	let text = "[contexts.global.bindings]\n\"test.look\" = [\"ctrl+l\"]";
	let error = Keymap::parse(text, &registered_actions(), &registered_contexts())
		.err()
		.unwrap();

	assert!(error.contains("test.look"), "{error}");
}

fn parse_error(text: &str) -> String {
	Keymap::parse(text, &registered_actions(), &registered_contexts())
		.err()
		.unwrap()
}

#[test]
fn parses_axis_inputs_into_axis_bindings() {
	let text = "[contexts.global.bindings]\n\"test.undo\" = [\"ctrl+z\"]\n\"test.look\" = [\"mouse_motion\"]\n\"test.zoom\" = [\"scroll\"]";
	let keymap = Keymap::parse(text, &registered_actions(), &registered_contexts()).unwrap();

	assert_eq!(keymap.button_bindings().len(), 1);
	assert_eq!(
		keymap.axis_bindings(),
		&[
			AxisBinding {
				input: AxisInput::MouseMotion,
				modifiers: Modifiers::default(),
				scale: Scale::Two([1.0, 1.0]),
				action: "test.look",
				context: "global"
			},
			AxisBinding {
				input: AxisInput::Scroll,
				modifiers: Modifiers::default(),
				scale: Scale::One(1.0),
				action: "test.zoom",
				context: "global"
			}
		]
	);
}

#[test]
fn rejects_an_axis_input_whose_kind_does_not_match_the_action() {
	let on_button = parse_error("[contexts.global.bindings]\n\"test.undo\" = [\"mouse_motion\"]");
	assert!(on_button.contains("test.undo"), "{on_button}");

	let scroll_on_2d = parse_error("[contexts.global.bindings]\n\"test.look\" = [\"scroll\"]");
	assert!(scroll_on_2d.contains("test.look"), "{scroll_on_2d}");

	let motion_on_1d =
		parse_error("[contexts.global.bindings]\n\"test.zoom\" = [\"mouse_motion\"]");
	assert!(motion_on_1d.contains("test.zoom"), "{motion_on_1d}");
}

#[test]
fn parses_modifiers_on_axis_inputs() {
	let text = "[contexts.global.bindings]\n\"test.look\" = [\"ctrl+alt+mouse_motion\"]\n\"test.zoom\" = [\"Shift+scroll\"]";
	let keymap = Keymap::parse(text, &registered_actions(), &registered_contexts()).unwrap();

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
	let unknown = parse_error("[contexts.global.bindings]\n\"test.zoom\" = [\"meta+scroll\"]");
	assert!(unknown.contains("meta"), "{unknown}");

	let repeated =
		parse_error("[contexts.global.bindings]\n\"test.zoom\" = [\"shift+shift+scroll\"]");
	assert!(repeated.contains("repeated"), "{repeated}");
}

#[test]
fn the_same_axis_input_can_be_bound_with_different_modifiers() {
	let text = "[contexts.global.bindings]\n\"test.zoom\" = [\"scroll\", \"shift+scroll\"]";
	let keymap = Keymap::parse(text, &registered_actions(), &registered_contexts()).unwrap();

	assert_eq!(keymap.axis_bindings().len(), 2);
}

#[test]
fn rejects_the_same_axis_input_with_the_same_modifiers_twice() {
	let error = parse_error(
		"[contexts.global.bindings]\n\"test.zoom\" = [\"shift+scroll\", \"Shift+Scroll\"]",
	);

	assert!(error.contains("bound to both"), "{error}");
}

#[test]
fn rejects_an_axis_input_bound_twice() {
	let error = parse_error("[contexts.global.bindings]\n\"test.zoom\" = [\"scroll\", \"scroll\"]");

	assert!(error.contains("bound to both"), "{error}");
}

#[test]
fn a_table_entry_without_options_equals_a_plain_entry() {
	let plain =
		"[contexts.global.bindings]\n\"test.undo\" = [\"ctrl+z\"]\n\"test.zoom\" = [\"scroll\"]";
	let table = "[contexts.global.bindings]\n\"test.undo\" = [{ input = \"ctrl+z\" }]\n\"test.zoom\" = [{ input = \"scroll\" }]";
	let plain = Keymap::parse(plain, &registered_actions(), &registered_contexts()).unwrap();
	let table = Keymap::parse(table, &registered_actions(), &registered_contexts()).unwrap();

	assert_eq!(plain.button_bindings(), table.button_bindings());
	assert_eq!(plain.axis_bindings(), table.axis_bindings());
}

#[test]
fn plain_and_table_entries_can_be_mixed_in_one_list() {
	let text =
		"[contexts.global.bindings]\n\"test.redo\" = [\"ctrl+y\", { input = \"ctrl+shift+z\" }]";
	let keymap = Keymap::parse(text, &registered_actions(), &registered_contexts()).unwrap();

	assert_eq!(keymap.button_bindings().len(), 2);
}

#[test]
fn rejects_unknown_table_fields_and_a_missing_input() {
	let misspelled =
		"[contexts.global.bindings]\n\"test.zoom\" = [{ input = \"scroll\", scal = 2.0 }]";
	assert!(Keymap::parse(misspelled, &registered_actions(), &registered_contexts()).is_err());

	let no_input = "[contexts.global.bindings]\n\"test.zoom\" = [{ scale = 2.0 }]";
	assert!(Keymap::parse(no_input, &registered_actions(), &registered_contexts()).is_err());
}

#[test]
fn parses_scales_for_both_axis_shapes() {
	let text = "[contexts.global.bindings]\n\"test.look\" = [{ input = \"mouse_motion\", scale = [0.2, -0.2] }]\n\"test.zoom\" = [{ input = \"scroll\", scale = -1.0 }]";
	let keymap = Keymap::parse(text, &registered_actions(), &registered_contexts()).unwrap();

	let scales: Vec<Scale> = keymap
		.axis_bindings()
		.iter()
		.map(|binding| binding.scale)
		.collect();
	assert_eq!(scales, [Scale::Two([0.2, -0.2]), Scale::One(-1.0)]);
}

#[test]
fn rejects_a_scale_that_does_not_fit_the_action() {
	let pair_on_1d = parse_error(
		"[contexts.global.bindings]\n\"test.zoom\" = [{ input = \"scroll\", scale = [1.0, 2.0] }]",
	);
	assert!(pair_on_1d.contains("test.zoom"), "{pair_on_1d}");

	let number_on_2d = parse_error(
		"[contexts.global.bindings]\n\"test.look\" = [{ input = \"mouse_motion\", scale = 2.0 }]",
	);
	assert!(number_on_2d.contains("test.look"), "{number_on_2d}");

	let on_shortcut = parse_error(
		"[contexts.global.bindings]\n\"test.undo\" = [{ input = \"ctrl+z\", scale = 2.0 }]",
	);
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
		let text = format!(
			"[contexts.global.bindings]\n\"{action}\" = [{{ input = \"{input}\", scale = {scale} }}]"
		);

		assert!(
			Keymap::parse(&text, &registered_actions(), &registered_contexts()).is_err(),
			"{scale}"
		);
	}

	let not_a_number =
		"[contexts.global.bindings]\n\"test.zoom\" = [{ input = \"scroll\", scale = nan }]";
	assert!(Keymap::parse(not_a_number, &registered_actions(), &registered_contexts()).is_err());
}

#[test]
fn reports_the_actions_a_keymap_leaves_unbound_compared_to_the_defaults() {
	let defaults = "[contexts.global.bindings]\n\"test.undo\" = [\"ctrl+z\"]\n\"test.redo\" = [\"ctrl+y\"]\n\"test.look\" = [\"mouse_motion\"]\n\"test.zoom\" = [\"scroll\"]";
	let older = "[contexts.global.bindings]\n\"test.undo\" = [\"ctrl+u\"]\n\"test.zoom\" = [\"shift+scroll\"]";
	let defaults = Keymap::parse(defaults, &registered_actions(), &registered_contexts()).unwrap();
	let older = Keymap::parse(older, &registered_actions(), &registered_contexts()).unwrap();

	// Rebinding an action is not a problem; only an action without any binding is.
	assert_eq!(
		older.actions_unbound_compared_to(&defaults),
		["test.look", "test.redo"]
	);
	assert!(defaults.actions_unbound_compared_to(&defaults).is_empty());
}

const WASD: &str = r#"{ composite = "2d", up = "w", down = "s", left = "a", right = "d" }"#;

fn bindings(lines: &[&str]) -> String {
	format!("[contexts.global.bindings]\n{}", lines.join("\n"))
}

#[test]
fn parses_composites_of_both_shapes_with_modifiers_and_scale() {
	let text = bindings(&[
		&format!(
			"\"test.move\" = [{WASD}, {{ composite = \"2d\", up = \"UP\", down = \"down\", left = \"left\", right = \"right\", modifiers = [\"shift\"], scale = [2.0, 2.0] }}]"
		),
		r#""test.rise" = [{ composite = "1d", up = "mouse_left", down = "space" }]"#,
	]);
	let keymap = Keymap::parse(&text, &registered_actions(), &registered_contexts()).unwrap();

	let bindings = keymap.axis_bindings();
	assert_eq!(bindings.len(), 3);
	assert_eq!(bindings[0].modifiers, Modifiers::default());
	assert_eq!(bindings[0].scale, Scale::Two([1.0, 1.0]));
	assert_eq!(bindings[1].modifiers, "shift".parse().unwrap());
	assert_eq!(bindings[1].scale, Scale::Two([2.0, 2.0]));
	assert_eq!(
		bindings[2].input,
		AxisInput::composite_1d(
			ShortcutInput::Mouse(MouseButton::Left),
			ShortcutInput::Key(KeyCode::Space)
		)
		.unwrap()
	);
}

#[test]
fn a_composite_must_match_the_kind_of_its_action() {
	let two_d_on_1d = parse_error(&bindings(&[&format!("\"test.rise\" = [{WASD}]")]));
	assert!(two_d_on_1d.contains("test.rise"), "{two_d_on_1d}");

	let one_d_on_2d = parse_error(&bindings(&[
		r#""test.move" = [{ composite = "1d", up = "w", down = "s" }]"#,
	]));
	assert!(one_d_on_2d.contains("test.move"), "{one_d_on_2d}");

	// Composites are held states, so a relative axis such as looking cannot take one.
	let on_relative = parse_error(&bindings(&[&format!("\"test.look\" = [{WASD}]")]));
	assert!(on_relative.contains("test.look"), "{on_relative}");

	let on_button = parse_error(&bindings(&[&format!("\"test.undo\" = [{WASD}]")]));
	assert!(on_button.contains("test.undo"), "{on_button}");
}

#[test]
fn rejects_malformed_composite_tables_with_a_message_that_names_the_problem() {
	let cases = [
		(r#"{ input = "scroll", composite = "1d" }"#, "not both"),
		(r#"{ scale = 2.0 }"#, "needs `input` or `composite`"),
		(
			r#"{ composite = "3d", up = "w", down = "s" }"#,
			"unknown composite",
		),
		(
			r#"{ composite = "2d", up = "w", down = "s", left = "a" }"#,
			"missing `right`",
		),
		(r#"{ composite = "1d", up = "w" }"#, "missing `down`"),
		(
			r#"{ composite = "1d", up = "w", down = "s", left = "a" }"#,
			"does not take `left`",
		),
		(
			r#"{ input = "scroll", up = "w" }"#,
			"only go with `composite`",
		),
		(
			r#"{ input = "scroll", modifiers = ["shift"] }"#,
			"only goes with `composite`",
		),
		(
			r#"{ composite = "1d", up = "w", down = "plus" }"#,
			"unknown key",
		),
		(
			r#"{ composite = "1d", up = "w", down = "W" }"#,
			"two directions",
		),
		(
			r#"{ composite = "1d", up = "w", down = "s", modifiers = ["meta"] }"#,
			"meta",
		),
	];

	for (entry, expected) in cases {
		let text = bindings(&[&format!("\"test.rise\" = [{entry}]")]);
		let error = parse_error(&text);
		assert!(error.contains(expected), "{entry}: {error}");
	}
}

#[test]
fn a_composite_scale_must_fit_the_shape() {
	let number_on_2d = bindings(&[
		r#""test.move" = [{ composite = "2d", up = "w", down = "s", left = "a", right = "d", scale = 2.0 }]"#,
	]);
	assert!(Keymap::parse(&number_on_2d, &registered_actions(), &registered_contexts()).is_err());

	let pair_on_1d = bindings(&[
		r#""test.rise" = [{ composite = "1d", up = "w", down = "s", scale = [1.0, 1.0] }]"#,
	]);
	assert!(Keymap::parse(&pair_on_1d, &registered_actions(), &registered_contexts()).is_err());
}

#[test]
fn a_direction_cannot_also_be_a_shortcut_with_the_same_modifiers() {
	let plain = bindings(&[
		&format!("\"test.move\" = [{WASD}]"),
		r#""test.undo" = ["s"]"#,
	]);
	let error = parse_error(&plain);
	assert!(
		error.contains("test.undo") && error.contains("test.move"),
		"{error}"
	);

	// Ctrl+S is a different button state than S, so both can exist.
	let with_ctrl = bindings(&[
		&format!("\"test.move\" = [{WASD}]"),
		r#""test.undo" = ["ctrl+s"]"#,
	]);
	assert!(Keymap::parse(&with_ctrl, &registered_actions(), &registered_contexts()).is_ok());
}

#[test]
fn a_mouse_button_direction_conflicts_with_the_same_mouse_shortcut() {
	let text = bindings(&[
		r#""test.rise" = [{ composite = "1d", up = "mouse_left", down = "space" }]"#,
		r#""test.undo" = ["mouse_left"]"#,
	]);

	assert!(Keymap::parse(&text, &registered_actions(), &registered_contexts()).is_err());
}

#[test]
fn two_composites_cannot_share_a_direction_unless_their_modifiers_differ() {
	let shared = bindings(&[
		&format!("\"test.move\" = [{WASD}]"),
		r#""test.rise" = [{ composite = "1d", up = "w", down = "c" }]"#,
	]);
	let error = parse_error(&shared);
	assert!(error.contains("both"), "{error}");

	let modified = bindings(&[
		&format!("\"test.move\" = [{WASD}]"),
		r#""test.rise" = [{ composite = "1d", up = "w", down = "c", modifiers = ["shift"] }]"#,
	]);
	assert!(Keymap::parse(&modified, &registered_actions(), &registered_contexts()).is_ok());
}

#[test]
fn the_same_composite_cannot_be_bound_twice() {
	let text = bindings(&[&format!("\"test.move\" = [{WASD}, {WASD}]")]);

	assert!(parse_error(&text).contains("bound to both"));
}

// ## Contexts ## //

#[test]
fn bindings_know_their_context() {
	let text = "[contexts.global.bindings]\n\"test.undo\" = [\"ctrl+z\"]\n\
		[contexts.viewport.bindings]\n\"test.zoom\" = [\"scroll\"]";
	let keymap = Keymap::parse(text, &registered_actions(), &registered_contexts()).unwrap();

	assert_eq!(keymap.button_bindings()[0].context, "global");
	assert_eq!(keymap.axis_bindings()[0].context, "viewport");
}

#[test]
fn an_unknown_context_is_an_error() {
	let error = parse_error("[contexts.dialog.bindings]\n\"test.undo\" = [\"ctrl+z\"]");

	assert!(error.contains("unknown context \"dialog\""), "{error}");
}

#[test]
fn a_file_without_contexts_is_rejected() {
	// The format from before contexts, so an old user file falls back to the built-in keymap.
	let error = parse_error("[bindings]\n\"test.undo\" = [\"ctrl+z\"]");

	assert!(error.contains("bindings"), "{error}");
}

#[test]
fn two_contexts_may_bind_the_same_input() {
	let text = "[contexts.global.bindings]\n\"test.undo\" = [\"ctrl+z\"]\n\"test.zoom\" = [\"scroll\"]\n\
		[contexts.viewport.bindings]\n\"test.redo\" = [\"ctrl+z\"]\n\"test.look\" = [\"mouse_motion\"]\n\
		\"test.move\" = [{ composite = \"2d\", up = \"w\", down = \"s\", left = \"a\", right = \"d\" }]";

	assert!(Keymap::parse(text, &registered_actions(), &registered_contexts()).is_ok());
}

#[test]
fn the_conflict_rule_applies_within_each_context() {
	let in_one_context =
		"[contexts.viewport.bindings]\n\"test.undo\" = [\"ctrl+z\"]\n\"test.redo\" = [\"ctrl+z\"]";
	let error = parse_error(in_one_context);
	assert!(error.contains("context \"viewport\""), "{error}");

	// A key of a composite and a shortcut in the same context.
	let with_composite = "[contexts.viewport.bindings]\n\"test.undo\" = [\"w\"]\n\
		\"test.move\" = [{ composite = \"2d\", up = \"w\", down = \"s\", left = \"a\", right = \"d\" }]";
	assert!(parse_error(with_composite).contains("context \"viewport\""));

	// The same two bindings in different contexts are shadowing, not a conflict.
	let split = "[contexts.global.bindings]\n\"test.undo\" = [\"w\"]\n\
		[contexts.viewport.bindings]\n\"test.move\" = [{ composite = \"2d\", up = \"w\", down = \"s\", left = \"a\", right = \"d\" }]";
	assert!(Keymap::parse(split, &registered_actions(), &registered_contexts()).is_ok());
}

#[test]
fn an_action_can_be_bound_in_several_contexts_and_lists_the_highest_first() {
	let text = "[contexts.global.bindings]\n\"test.undo\" = [\"ctrl+z\"]\n\
		[contexts.viewport.bindings]\n\"test.undo\" = [\"u\"]";
	let keymap = Keymap::parse(text, &registered_actions(), &registered_contexts()).unwrap();

	let shortcuts: Vec<String> = keymap
		.hints_for("test.undo", &ActiveContexts::default())
		.iter()
		.map(|hint| hint.shortcut.to_string())
		.collect();
	assert_eq!(shortcuts, ["U", "Ctrl+Z"]);
}

#[test]
fn a_composite_can_use_modifier_keys_as_directions() {
	let text = "[contexts.viewport.bindings]\n\"test.rise\" = [{ composite = \"1d\", up = \"shift\", down = \"ctrl\" }]";
	let keymap = Keymap::parse(text, &registered_actions(), &registered_contexts()).unwrap();

	assert_eq!(
		keymap.axis_bindings()[0].input.directions(),
		[
			ShortcutInput::Modifier(Modifier::Shift),
			ShortcutInput::Modifier(Modifier::Ctrl)
		]
	);
}

#[test]
fn a_composite_cannot_require_a_modifier_it_uses_as_a_direction() {
	let text = "[contexts.viewport.bindings]\n\"test.rise\" = [{ composite = \"1d\", up = \"shift\", down = \"c\", modifiers = [\"shift\"] }]";
	let error = parse_error(text);

	assert!(error.contains("direction and as a modifier"), "{error}");
}

#[test]
fn a_modifier_key_conflicts_like_any_other_input_within_a_context() {
	let text = "[contexts.viewport.bindings]\n\"test.undo\" = [\"shift\"]\n\
		\"test.rise\" = [{ composite = \"1d\", up = \"shift\", down = \"ctrl\" }]";
	assert!(parse_error(text).contains("context \"viewport\""));
}

// ## Hints ## //

fn stack(names: &[&str]) -> ActiveContexts {
	let contexts = registered_contexts();
	ActiveContexts::new(
		names
			.iter()
			.map(|name| contexts.get(name).unwrap().clone())
			.collect(),
	)
}

fn availabilities(
	keymap: &Keymap,
	action: &'static str,
	active: &ActiveContexts,
) -> Vec<Availability> {
	keymap
		.hints_for(action, active)
		.iter()
		.map(|hint| hint.availability)
		.collect()
}

const SHADOWED: &str = "[contexts.global.bindings]\n\"test.undo\" = [\"ctrl+z\"]\n\
	[contexts.viewport.bindings]\n\"test.redo\" = [\"ctrl+z\"]";

#[test]
fn a_hint_is_available_when_its_context_is_active_and_nothing_takes_the_shortcut() {
	let keymap = Keymap::parse(SHADOWED, &registered_actions(), &registered_contexts()).unwrap();

	assert_eq!(
		availabilities(&keymap, "test.undo", &stack(&["global"])),
		[Availability::Available]
	);
}

#[test]
fn a_hint_of_an_inactive_context_says_so() {
	let keymap = Keymap::parse(SHADOWED, &registered_actions(), &registered_contexts()).unwrap();

	assert_eq!(
		availabilities(&keymap, "test.redo", &stack(&["global"])),
		[Availability::ContextInactive]
	);
}

#[test]
fn a_hint_shadowed_by_a_higher_context_is_blocked() {
	let keymap = Keymap::parse(SHADOWED, &registered_actions(), &registered_contexts()).unwrap();

	assert_eq!(
		availabilities(&keymap, "test.undo", &stack(&["global", "viewport"])),
		[Availability::Blocked]
	);
}

#[test]
fn a_hint_blocked_by_a_block_rule_is_blocked() {
	let mut contexts = registered_contexts();
	contexts
		.register(
			InputContext::new("text")
				.priority(20)
				.block(Block::Modifier(Modifier::Ctrl)),
		)
		.unwrap();
	let keymap = Keymap::parse(SHADOWED, &registered_actions(), &contexts).unwrap();
	let active = ActiveContexts::new(vec![
		contexts.get("global").unwrap().clone(),
		contexts.get("text").unwrap().clone(),
	]);

	assert_eq!(
		availabilities(&keymap, "test.undo", &active),
		[Availability::Blocked]
	);
}

#[test]
fn the_best_hint_comes_first() {
	let text = "[contexts.global.bindings]\n\"test.undo\" = [\"ctrl+z\", \"ctrl+u\"]\n\
		[contexts.viewport.bindings]\n\"test.undo\" = [\"u\"]\n\"test.redo\" = [\"ctrl+z\"]";
	let keymap = Keymap::parse(text, &registered_actions(), &registered_contexts()).unwrap();

	// Viewport inactive: ctrl+z and ctrl+u work, the viewport's `u` does not.
	let hints = keymap.hints_for("test.undo", &stack(&["global"]));
	let listed: Vec<_> = hints
		.iter()
		.map(|hint| (hint.shortcut.to_string(), hint.availability))
		.collect();
	assert_eq!(
		listed,
		[
			("Ctrl+Z".to_string(), Availability::Available),
			("Ctrl+U".to_string(), Availability::Available),
			("U".to_string(), Availability::ContextInactive),
		]
	);

	// Viewport active: ctrl+z is taken by the viewport, so the shortcut that works comes first.
	let hints = keymap.hints_for("test.undo", &stack(&["global", "viewport"]));
	assert_eq!(hints[0].shortcut.to_string(), "U");
	assert_eq!(hints[0].availability, Availability::Available);
	assert_eq!(hints[2].shortcut.to_string(), "Ctrl+Z");
	assert_eq!(hints[2].availability, Availability::Blocked);
}

#[test]
fn an_action_without_shortcuts_has_no_hints() {
	let keymap = Keymap::parse(SHADOWED, &registered_actions(), &registered_contexts()).unwrap();

	assert!(
		keymap
			.hints_for("test.look", &stack(&["global"]))
			.is_empty()
	);
}

// ## Interpretation ## //

fn first_button(text: &str) -> ButtonBinding {
	Keymap::parse(text, &registered_actions(), &registered_contexts())
		.unwrap()
		.button_bindings()[0]
}

#[test]
fn a_shortcut_is_pressed_unless_the_entry_says_otherwise() {
	let plain = first_button("[contexts.global.bindings]\n\"test.undo\" = [\"ctrl+z\"]");
	let table = first_button(
		"[contexts.global.bindings]\n\"test.undo\" = [{ input = \"ctrl+z\", interpret = \"released\" }]",
	);
	let explicit = first_button(
		"[contexts.global.bindings]\n\"test.undo\" = [{ input = \"ctrl+z\", interpret = \"Pressed\" }]",
	);

	assert_eq!(plain.interpretation, Interpretation::Pressed);
	assert_eq!(table.interpretation, Interpretation::Released);
	assert_eq!(explicit.interpretation, Interpretation::Pressed);
}

#[test]
fn an_unknown_interpretation_is_an_error() {
	let error = parse_error(
		"[contexts.global.bindings]\n\"test.undo\" = [{ input = \"ctrl+z\", interpret = \"held\" }]",
	);

	assert!(error.contains("unknown interpretation \"held\""), "{error}");
}

#[test]
fn only_a_shortcut_can_have_an_interpretation() {
	let on_axis = parse_error(
		"[contexts.global.bindings]\n\"test.zoom\" = [{ input = \"scroll\", interpret = \"released\" }]",
	);
	assert!(on_axis.contains("only a shortcut"), "{on_axis}");

	let on_composite = parse_error(
		"[contexts.global.bindings]\n\"test.move\" = [{ composite = \"2d\", up = \"w\", down = \"s\", left = \"a\", right = \"d\", interpret = \"released\" }]",
	);
	assert!(on_composite.contains("only a shortcut"), "{on_composite}");
}

#[test]
fn a_shortcut_can_flip_a_toggle() {
	let keymap = Keymap::parse(
		"[contexts.global.bindings]\n\"test.snap\" = [\"g\"]",
		&registered_actions(),
		&registered_contexts(),
	);

	assert!(keymap.is_ok());
}
