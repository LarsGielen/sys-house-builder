use super::*;
use crate::core::input::{Action, ActionGroup, ValueKind};

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

const KEYMAP: &str = "[bindings]\n\"test.orbit\" = [\"mouse_middle\"]\n\"test.pan\" = [\"shift+mouse_middle\"]\n\"test.undo\" = [\"ctrl+z\", \"ctrl+y\"]";

/// One simulated app: the devices, the source and the action state it feeds.
struct Frames {
	keys: ButtonInput<KeyCode>,
	mouse_buttons: ButtonInput<MouseButton>,
	keymap: Keymap,
	source: ButtonSource,
	state: ActionState,
}

impl Frames {
	fn new() -> Self {
		let mut state = ActionState::default();
		state.register_group::<TestAction>();
		Self {
			keys: ButtonInput::default(),
			mouse_buttons: ButtonInput::default(),
			keymap: Keymap::parse(KEYMAP, &state).unwrap(),
			source: ButtonSource::default(),
			state,
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
	composite_signal(input, &key_input, &mouse_input)
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
