use super::*;
use crate::core::input::{
	Action, ActionGroup, Block, InputContext, InputContexts, Modifier, ValueKind,
	context::ActiveContexts,
};

#[derive(Clone, Copy)]
enum TestAction {
	Orbit,
	Pan,
	Undo,
}

impl Action for TestAction {
	fn name(self) -> &'static str {
		match self {
			Self::Orbit => "test.orbit",
			Self::Pan => "test.pan",
			Self::Undo => "test.undo",
		}
	}

	fn value(self) -> ValueKind {
		ValueKind::Button
	}
}

impl ActionGroup for TestAction {
	fn all() -> &'static [Self] {
		&[Self::Orbit, Self::Pan, Self::Undo]
	}
}

const KEYMAP: &str = "[contexts.global.bindings]\n\"test.orbit\" = [\"mouse_middle\"]\n\"test.pan\" = [\"shift+mouse_middle\"]\n\"test.undo\" = [\"ctrl+z\", \"ctrl+y\"]";

fn global() -> InputContext {
	InputContext::new("global").priority(0)
}

fn viewport() -> InputContext {
	InputContext::new("viewport").priority(10)
}

/// One simulated app: the devices, the source and the action state it feeds.
struct Frames {
	keys: ButtonInput<KeyCode>,
	mouse_buttons: ButtonInput<MouseButton>,
	keymap: Keymap,
	source: ButtonSource,
	state: ActionState,
	active: ActiveContexts,
}

impl Frames {
	fn new() -> Self {
		Self::with_keymap(KEYMAP, &[global()], &[global()])
	}

	/// `registered` are the contexts the keymap may name, `active` the ones on the stack.
	fn with_keymap(keymap: &str, registered: &[InputContext], active: &[InputContext]) -> Self {
		let mut state = ActionState::default();
		state.register_group::<TestAction>();
		let mut contexts = InputContexts::default();
		for context in registered {
			contexts.register(context.clone()).unwrap();
		}
		let active = ActiveContexts::new(active.to_vec());
		Self {
			keys: ButtonInput::default(),
			mouse_buttons: ButtonInput::default(),
			keymap: Keymap::parse(keymap, &state, &contexts).unwrap(),
			source: ButtonSource::default(),
			state,
			active,
		}
	}

	/// Runs one frame with the inputs changed since the last one, then clears the one-frame
	/// flags (`just_pressed`, `just_released`) the way Bevy does at the start of the next frame.
	fn next(&mut self) {
		self.state.start_frame();
		self.source.poll(
			&self.keymap,
			&self.keys,
			&self.mouse_buttons,
			&self.active,
			&mut self.state,
		);
		self.state.resolve();
		self.keys.clear();
		self.mouse_buttons.clear();
	}
}

#[test]
fn pressing_the_input_starts_the_action() {
	let mut frames = Frames::new();
	frames.mouse_buttons.press(MouseButton::Middle);
	frames.next();

	assert!(frames.state.pressed(TestAction::Orbit));
	assert!(frames.state.just_pressed(TestAction::Orbit));
	assert!(!frames.state.pressed(TestAction::Pan));
}

#[test]
fn a_held_input_stays_pressed_without_a_new_edge() {
	let mut frames = Frames::new();
	frames.mouse_buttons.press(MouseButton::Middle);
	frames.next();
	frames.next();

	assert!(frames.state.pressed(TestAction::Orbit));
	assert!(!frames.state.just_pressed(TestAction::Orbit));
}

#[test]
fn a_shortcut_only_starts_with_exactly_its_modifiers() {
	let mut frames = Frames::new();
	frames.keys.press(KeyCode::ShiftLeft);
	frames.mouse_buttons.press(MouseButton::Middle);
	frames.next();

	assert!(frames.state.pressed(TestAction::Pan));
	assert!(!frames.state.pressed(TestAction::Orbit));
}

#[test]
fn releasing_a_modifier_does_not_end_a_started_shortcut() {
	let mut frames = Frames::new();
	frames.keys.press(KeyCode::ShiftLeft);
	frames.mouse_buttons.press(MouseButton::Middle);
	frames.next();

	frames.keys.release(KeyCode::ShiftLeft);
	frames.next();

	assert!(frames.state.pressed(TestAction::Pan));
	// Orbit would match the bare middle button, but it only starts on the press itself.
	assert!(!frames.state.pressed(TestAction::Orbit));
}

#[test]
fn releasing_the_input_ends_the_action() {
	let mut frames = Frames::new();
	frames.keys.press(KeyCode::ShiftLeft);
	frames.mouse_buttons.press(MouseButton::Middle);
	frames.next();

	frames.mouse_buttons.release(MouseButton::Middle);
	frames.next();

	assert!(!frames.state.pressed(TestAction::Pan));
	assert!(frames.state.just_released(TestAction::Pan));
}

#[test]
fn a_modifier_pressed_after_the_input_does_not_turn_it_into_another_shortcut() {
	let mut frames = Frames::new();
	frames.mouse_buttons.press(MouseButton::Middle);
	frames.next();

	frames.keys.press(KeyCode::ShiftLeft);
	frames.next();

	assert!(frames.state.pressed(TestAction::Orbit));
	assert!(!frames.state.pressed(TestAction::Pan));
}

#[test]
fn an_action_with_two_shortcuts_stays_pressed_until_both_are_released() {
	let mut frames = Frames::new();
	frames.keys.press(KeyCode::ControlLeft);
	frames.keys.press(KeyCode::KeyZ);
	frames.keys.press(KeyCode::KeyY);
	frames.next();
	assert!(frames.state.pressed(TestAction::Undo));

	frames.keys.release(KeyCode::KeyZ);
	frames.next();
	assert!(frames.state.pressed(TestAction::Undo));
	assert!(!frames.state.just_released(TestAction::Undo));

	frames.keys.release(KeyCode::KeyY);
	frames.next();
	assert!(!frames.state.pressed(TestAction::Undo));
	assert!(frames.state.just_released(TestAction::Undo));
}

fn motion(delta: Vec2) -> AccumulatedMouseMotion {
	AccumulatedMouseMotion { delta }
}

fn scroll(unit: MouseScrollUnit, delta: Vec2) -> AccumulatedMouseScroll {
	AccumulatedMouseScroll { unit, delta }
}

fn idle_scroll() -> AccumulatedMouseScroll {
	scroll(MouseScrollUnit::Line, Vec2::ZERO)
}

#[test]
fn motion_passes_through_unchanged() {
	let signal = axis_signal(
		AxisInput::MouseMotion,
		&motion(Vec2::new(3.0, -4.0)),
		&idle_scroll(),
	);

	assert_eq!(signal, Some(Signal::Axis2D(Vec2::new(3.0, -4.0))));
}

#[test]
fn an_idle_mouse_contributes_nothing() {
	assert_eq!(
		axis_signal(AxisInput::MouseMotion, &motion(Vec2::ZERO), &idle_scroll()),
		None
	);
	assert_eq!(
		axis_signal(AxisInput::Scroll, &motion(Vec2::ZERO), &idle_scroll()),
		None
	);
}

#[test]
fn scroll_in_lines_passes_through_unchanged() {
	let wheel = scroll(MouseScrollUnit::Line, Vec2::new(0.0, 2.0));

	assert_eq!(
		axis_signal(AxisInput::Scroll, &motion(Vec2::ZERO), &wheel),
		Some(Signal::Axis1D(2.0))
	);
}

#[test]
fn scroll_in_pixels_is_converted_to_lines() {
	let trackpad = scroll(MouseScrollUnit::Pixel, Vec2::new(0.0, -PIXELS_PER_LINE));

	assert_eq!(
		axis_signal(AxisInput::Scroll, &motion(Vec2::ZERO), &trackpad),
		Some(Signal::Axis1D(-1.0))
	);
}

#[test]
fn horizontal_scroll_alone_contributes_nothing() {
	let sideways = scroll(MouseScrollUnit::Line, Vec2::new(3.0, 0.0));

	assert_eq!(
		axis_signal(AxisInput::Scroll, &motion(Vec2::ZERO), &sideways),
		None
	);
}

fn composite_2d() -> AxisInput {
	AxisInput::composite_2d(
		ShortcutInput::Key(KeyCode::KeyW),
		ShortcutInput::Key(KeyCode::KeyS),
		ShortcutInput::Key(KeyCode::KeyA),
		ShortcutInput::Key(KeyCode::KeyD),
	)
	.unwrap()
}

fn composite_1d() -> AxisInput {
	AxisInput::composite_1d(
		ShortcutInput::Mouse(MouseButton::Left),
		ShortcutInput::Key(KeyCode::KeyC),
	)
	.unwrap()
}

fn composite_with(input: AxisInput, keys: &[KeyCode], buttons: &[MouseButton]) -> Option<Signal> {
	let mut key_input = ButtonInput::<KeyCode>::default();
	let mut mouse_input = ButtonInput::<MouseButton>::default();
	keys.iter().for_each(|&key| key_input.press(key));
	buttons.iter().for_each(|&button| mouse_input.press(button));
	composite_signal(input, &key_input, &mouse_input, |_| true)
}

#[test]
fn a_held_direction_gives_a_unit_vector() {
	assert_eq!(
		composite_with(composite_2d(), &[KeyCode::KeyW], &[]),
		Some(Signal::Axis2D(Vec2::new(0.0, 1.0)))
	);
	assert_eq!(
		composite_with(composite_2d(), &[KeyCode::KeyA], &[]),
		Some(Signal::Axis2D(Vec2::new(-1.0, 0.0)))
	);
}

#[test]
fn diagonals_are_not_faster_than_straight_lines() {
	let Some(Signal::Axis2D(value)) =
		composite_with(composite_2d(), &[KeyCode::KeyW, KeyCode::KeyD], &[])
	else {
		panic!("a diagonal must give a 2D signal");
	};

	assert!((value.length() - 1.0).abs() < 1e-6);
	assert!(value.x > 0.0 && value.y > 0.0);
}

#[test]
fn opposite_directions_cancel_out() {
	assert_eq!(
		composite_with(composite_2d(), &[KeyCode::KeyW, KeyCode::KeyS], &[]),
		None
	);
	// One axis can cancel while the other still moves.
	assert_eq!(
		composite_with(
			composite_2d(),
			&[KeyCode::KeyW, KeyCode::KeyS, KeyCode::KeyD],
			&[]
		),
		Some(Signal::Axis2D(Vec2::new(1.0, 0.0)))
	);
}

#[test]
fn an_idle_composite_contributes_nothing() {
	assert_eq!(composite_with(composite_2d(), &[], &[]), None);
	assert_eq!(composite_with(composite_1d(), &[], &[]), None);
}

#[test]
fn a_1d_composite_reads_keys_and_mouse_buttons() {
	assert_eq!(
		composite_with(composite_1d(), &[], &[MouseButton::Left]),
		Some(Signal::Axis1D(1.0))
	);
	assert_eq!(
		composite_with(composite_1d(), &[KeyCode::KeyC], &[]),
		Some(Signal::Axis1D(-1.0))
	);
	assert_eq!(
		composite_with(composite_1d(), &[KeyCode::KeyC], &[MouseButton::Left]),
		None
	);
}

#[test]
fn the_mouse_signals_ignore_composites_and_the_other_way_round() {
	assert_eq!(
		axis_signal(composite_2d(), &motion(Vec2::ONE), &idle_scroll()),
		None
	);
	assert_eq!(
		composite_with(AxisInput::MouseMotion, &[KeyCode::KeyW], &[]),
		None
	);
}

// ## Contexts ## //

const TWO_CONTEXTS: &str = "[contexts.global.bindings]\n\"test.undo\" = [\"ctrl+z\"]\n\"test.pan\" = [\"mouse_middle\"]\n\
	[contexts.viewport.bindings]\n\"test.orbit\" = [\"mouse_middle\"]";

#[test]
fn a_shortcut_of_an_inactive_context_does_nothing() {
	let mut frames = Frames::with_keymap(TWO_CONTEXTS, &[global(), viewport()], &[global()]);
	frames.mouse_buttons.press(MouseButton::Middle);
	frames.next();

	assert!(!frames.state.pressed(TestAction::Orbit));
	assert!(frames.state.pressed(TestAction::Pan));
}

#[test]
fn a_higher_context_takes_the_shortcut_it_binds() {
	let mut frames = Frames::with_keymap(
		TWO_CONTEXTS,
		&[global(), viewport()],
		&[global(), viewport()],
	);
	frames.mouse_buttons.press(MouseButton::Middle);
	frames.next();

	assert!(frames.state.pressed(TestAction::Orbit));
	assert!(!frames.state.pressed(TestAction::Pan));
}

#[test]
fn a_held_shortcut_ends_when_its_context_goes_away() {
	let mut frames = Frames::with_keymap(
		TWO_CONTEXTS,
		&[global(), viewport()],
		&[global(), viewport()],
	);
	frames.mouse_buttons.press(MouseButton::Middle);
	frames.next();
	frames.next();
	assert!(frames.state.pressed(TestAction::Orbit));

	// The button is still held, but the viewport is gone: the drag must not keep running, and
	// the lower binding must not start from a button that was pressed earlier.
	frames.active = ActiveContexts::new(vec![global()]);
	frames.next();

	assert!(!frames.state.pressed(TestAction::Orbit));
	assert!(!frames.state.pressed(TestAction::Pan));
}

#[test]
fn a_context_that_appears_while_a_button_is_held_does_not_start_its_shortcut() {
	let mut frames = Frames::with_keymap(TWO_CONTEXTS, &[global(), viewport()], &[global()]);
	frames.mouse_buttons.press(MouseButton::Middle);
	frames.next();
	frames.active = ActiveContexts::new(vec![global(), viewport()]);
	frames.next();

	assert!(!frames.state.pressed(TestAction::Orbit));
}

#[test]
fn a_blocking_context_swallows_the_shortcuts_below_it() {
	let text_field = InputContext::new("text")
		.priority(20)
		.block(Block::Modifier(Modifier::Ctrl));
	let mut frames = Frames::with_keymap(
		TWO_CONTEXTS,
		&[global(), viewport(), text_field.clone()],
		&[global(), text_field],
	);
	frames.keys.press(KeyCode::ControlLeft);
	frames.keys.press(KeyCode::KeyZ);
	frames.next();

	assert!(!frames.state.pressed(TestAction::Undo));
}

// ## Modifier keys as inputs ## //

#[test]
fn a_modifier_key_can_be_a_shortcut() {
	let keymap = "[contexts.global.bindings]\n\"test.orbit\" = [\"shift\"]\n\"test.undo\" = [\"ctrl+shift\"]";
	let mut frames = Frames::with_keymap(keymap, &[global()], &[global()]);
	frames.keys.press(KeyCode::ShiftLeft);
	frames.next();

	assert!(frames.state.just_pressed(TestAction::Orbit));
	assert!(!frames.state.pressed(TestAction::Undo));

	// Held, it stays pressed, and releasing the key ends it.
	frames.next();
	assert!(frames.state.pressed(TestAction::Orbit));
	frames.keys.release(KeyCode::ShiftLeft);
	frames.next();
	assert!(!frames.state.pressed(TestAction::Orbit));
}

#[test]
fn a_modifier_shortcut_needs_the_modifiers_it_names() {
	let keymap = "[contexts.global.bindings]\n\"test.orbit\" = [\"shift\"]\n\"test.undo\" = [\"ctrl+shift\"]";
	let mut frames = Frames::with_keymap(keymap, &[global()], &[global()]);
	frames.keys.press(KeyCode::ControlLeft);
	frames.next();
	frames.keys.press(KeyCode::ShiftLeft);
	frames.next();

	assert!(frames.state.pressed(TestAction::Undo));
	assert!(!frames.state.pressed(TestAction::Orbit));
}

// ## Interpretation ## //

const RELEASED: &str = "[contexts.global.bindings]\n\"test.orbit\" = [{ input = \"mouse_middle\", interpret = \"released\" }]\n\"test.undo\" = [{ input = \"ctrl+z\", interpret = \"released\" }]";

#[test]
fn a_released_shortcut_is_active_for_one_frame_when_the_input_is_released() {
	let mut frames = Frames::with_keymap(RELEASED, &[global()], &[global()]);
	frames.mouse_buttons.press(MouseButton::Middle);
	frames.next();
	assert!(
		!frames.state.pressed(TestAction::Orbit),
		"nothing on the press"
	);

	frames.next();
	assert!(
		!frames.state.pressed(TestAction::Orbit),
		"nothing while held"
	);

	frames.mouse_buttons.release(MouseButton::Middle);
	frames.next();
	assert!(frames.state.pressed(TestAction::Orbit));
	assert!(frames.state.just_pressed(TestAction::Orbit));

	frames.next();
	assert!(
		!frames.state.pressed(TestAction::Orbit),
		"only for one frame"
	);
}

#[test]
fn a_released_shortcut_fires_even_if_a_modifier_is_let_go_first() {
	let mut frames = Frames::with_keymap(RELEASED, &[global()], &[global()]);
	frames.keys.press(KeyCode::ControlLeft);
	frames.keys.press(KeyCode::KeyZ);
	frames.next();
	frames.keys.release(KeyCode::ControlLeft);
	frames.next();
	assert!(!frames.state.pressed(TestAction::Undo));

	frames.keys.release(KeyCode::KeyZ);
	frames.next();
	assert!(frames.state.pressed(TestAction::Undo));
}

#[test]
fn a_released_shortcut_that_loses_its_context_is_cancelled() {
	let mut frames = Frames::with_keymap(RELEASED, &[global()], &[global()]);
	frames.mouse_buttons.press(MouseButton::Middle);
	frames.next();

	frames.active = ActiveContexts::new(vec![]);
	frames.next();
	frames.active = ActiveContexts::new(vec![global()]);
	frames.mouse_buttons.release(MouseButton::Middle);
	frames.next();

	assert!(!frames.state.pressed(TestAction::Orbit));
}
