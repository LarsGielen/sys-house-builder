use super::*;

#[derive(Clone, Copy)]
enum TestAction {
	Jump,
	Look,
	Zoom,
	Move,
	Rise,
}

impl Action for TestAction {
	fn name(self) -> &'static str {
		match self {
			Self::Jump => "test.jump",
			Self::Look => "test.look",
			Self::Zoom => "test.zoom",
			Self::Move => "test.move",
			Self::Rise => "test.rise",
		}
	}

	fn value(self) -> ValueKind {
		match self {
			Self::Jump => ValueKind::Button,
			Self::Look => ValueKind::Axis2D(AxisMode::Relative),
			Self::Zoom => ValueKind::Axis1D(AxisMode::Relative),
			Self::Move => ValueKind::Axis2D(AxisMode::Absolute),
			Self::Rise => ValueKind::Axis1D(AxisMode::Absolute),
		}
	}
}

impl ActionGroup for TestAction {
	fn all() -> &'static [Self] {
		&[Self::Jump, Self::Look, Self::Zoom, Self::Move, Self::Rise]
	}
}

fn app_with_devices() -> App {
	let mut app = App::new();
	app.init_resource::<ButtonInput<KeyCode>>()
		.init_resource::<ButtonInput<MouseButton>>()
		.init_resource::<AccumulatedMouseMotion>()
		.init_resource::<AccumulatedMouseScroll>()
		.add_plugins(InputPlugin)
		.add_input_actions::<TestAction>();
	app
}

fn run_input_frame(app: &mut App) {
	app.world_mut().run_schedule(PreUpdate);
}

#[test]
fn a_pressed_key_reaches_the_action_state_through_the_schedule() {
	let mut app = app_with_devices();
	let keymap = Keymap::parse(
		"[bindings]\n\"test.jump\" = [\"space\"]",
		app.world().resource::<ActionState>(),
	)
	.unwrap();
	app.insert_resource(keymap);

	run_input_frame(&mut app);
	assert!(
		!app.world()
			.resource::<ActionState>()
			.pressed(TestAction::Jump)
	);

	app.world_mut()
		.resource_mut::<ButtonInput<KeyCode>>()
		.press(KeyCode::Space);
	run_input_frame(&mut app);
	let state = app.world().resource::<ActionState>();
	assert!(state.pressed(TestAction::Jump));
	assert!(state.just_pressed(TestAction::Jump));

	app.world_mut()
		.resource_mut::<ButtonInput<KeyCode>>()
		.clear();
	run_input_frame(&mut app);
	let state = app.world().resource::<ActionState>();
	assert!(state.pressed(TestAction::Jump));
	assert!(!state.just_pressed(TestAction::Jump));
}

#[test]
fn frames_without_a_keymap_are_skipped_instead_of_panicking() {
	let mut app = app_with_devices();
	app.world_mut()
		.resource_mut::<ButtonInput<KeyCode>>()
		.press(KeyCode::Space);

	run_input_frame(&mut app);

	assert!(
		!app.world()
			.resource::<ActionState>()
			.pressed(TestAction::Jump)
	);
}

const AXIS_KEYMAP: &str =
	"[bindings]\n\"test.look\" = [\"mouse_motion\"]\n\"test.zoom\" = [\"scroll\"]";

fn app_with_axis_keymap() -> App {
	let mut app = app_with_devices();
	let keymap = Keymap::parse(AXIS_KEYMAP, app.world().resource::<ActionState>()).unwrap();
	app.insert_resource(keymap);
	app
}

#[test]
fn mouse_motion_reaches_the_action_state_for_one_frame() {
	let mut app = app_with_axis_keymap();

	app.world_mut()
		.resource_mut::<AccumulatedMouseMotion>()
		.delta = Vec2::new(3.0, -2.0);
	run_input_frame(&mut app);
	assert_eq!(
		app.world()
			.resource::<ActionState>()
			.axis2d(TestAction::Look),
		Vec2::new(3.0, -2.0)
	);

	// Bevy zeroes the accumulated delta every frame, and the action must follow.
	app.world_mut()
		.resource_mut::<AccumulatedMouseMotion>()
		.delta = Vec2::ZERO;
	run_input_frame(&mut app);
	assert_eq!(
		app.world()
			.resource::<ActionState>()
			.axis2d(TestAction::Look),
		Vec2::ZERO
	);
}

#[test]
fn scroll_reaches_the_action_state_for_one_frame() {
	let mut app = app_with_axis_keymap();

	app.world_mut()
		.resource_mut::<AccumulatedMouseScroll>()
		.delta = Vec2::new(0.0, 2.0);
	run_input_frame(&mut app);
	assert_eq!(
		app.world()
			.resource::<ActionState>()
			.axis1d(TestAction::Zoom),
		2.0
	);

	app.world_mut()
		.resource_mut::<AccumulatedMouseScroll>()
		.delta = Vec2::ZERO;
	run_input_frame(&mut app);
	assert_eq!(
		app.world()
			.resource::<ActionState>()
			.axis1d(TestAction::Zoom),
		0.0
	);
}

#[test]
fn a_binding_scale_is_applied_before_the_value_reaches_the_action_state() {
	let mut app = app_with_devices();
	let text = "[bindings]\n\"test.look\" = [{ input = \"mouse_motion\", scale = [0.5, -1.0] }]\n\"test.zoom\" = [{ input = \"scroll\", scale = -1.0 }]";
	let keymap = Keymap::parse(text, app.world().resource::<ActionState>()).unwrap();
	app.insert_resource(keymap);

	app.world_mut()
		.resource_mut::<AccumulatedMouseMotion>()
		.delta = Vec2::new(4.0, 2.0);
	app.world_mut()
		.resource_mut::<AccumulatedMouseScroll>()
		.delta = Vec2::new(0.0, 3.0);
	run_input_frame(&mut app);

	let state = app.world().resource::<ActionState>();
	assert_eq!(state.axis2d(TestAction::Look), Vec2::new(2.0, -2.0));
	assert_eq!(state.axis1d(TestAction::Zoom), -3.0);
}

#[test]
fn an_axis_binding_fires_only_with_exactly_its_modifiers() {
	let mut app = app_with_devices();
	let text = "[bindings]\n\"test.zoom\" = [\"scroll\"]\n\"test.look\" = [\"shift+mouse_motion\"]";
	let keymap = Keymap::parse(text, app.world().resource::<ActionState>()).unwrap();
	app.insert_resource(keymap);
	let scroll = Vec2::new(0.0, 2.0);
	let motion = Vec2::new(3.0, 1.0);

	// Without Shift only the plain binding fires.
	app.world_mut()
		.resource_mut::<AccumulatedMouseScroll>()
		.delta = scroll;
	app.world_mut()
		.resource_mut::<AccumulatedMouseMotion>()
		.delta = motion;
	run_input_frame(&mut app);
	let state = app.world().resource::<ActionState>();
	assert_eq!(state.axis1d(TestAction::Zoom), 2.0);
	assert_eq!(state.axis2d(TestAction::Look), Vec2::ZERO);

	// With Shift held the plain binding stops and the Shift binding takes over.
	app.world_mut()
		.resource_mut::<ButtonInput<KeyCode>>()
		.press(KeyCode::ShiftRight);
	run_input_frame(&mut app);
	let state = app.world().resource::<ActionState>();
	assert_eq!(state.axis1d(TestAction::Zoom), 0.0);
	assert_eq!(state.axis2d(TestAction::Look), motion);

	// Any extra modifier breaks the match.
	app.world_mut()
		.resource_mut::<ButtonInput<KeyCode>>()
		.press(KeyCode::ControlLeft);
	run_input_frame(&mut app);
	let state = app.world().resource::<ActionState>();
	assert_eq!(state.axis2d(TestAction::Look), Vec2::ZERO);
}

const COMPOSITE_KEYMAP: &str = r#"[bindings]
"test.move" = [{ composite = "2d", up = "w", down = "s", left = "a", right = "d" }]
"test.rise" = [{ composite = "1d", up = "mouse_left", down = "c", scale = 3.0 }]"#;

fn app_with_composite_keymap() -> App {
	let mut app = app_with_devices();
	let keymap = Keymap::parse(COMPOSITE_KEYMAP, app.world().resource::<ActionState>()).unwrap();
	app.insert_resource(keymap);
	app
}

#[test]
fn held_keys_drive_a_composite_for_as_long_as_they_are_held() {
	let mut app = app_with_composite_keymap();

	app.world_mut()
		.resource_mut::<ButtonInput<KeyCode>>()
		.press(KeyCode::KeyW);
	run_input_frame(&mut app);
	assert_eq!(
		app.world()
			.resource::<ActionState>()
			.axis2d(TestAction::Move),
		Vec2::new(0.0, 1.0)
	);

	// Still held, with no new edge: an absolute axis keeps its value.
	app.world_mut()
		.resource_mut::<ButtonInput<KeyCode>>()
		.clear();
	run_input_frame(&mut app);
	assert_eq!(
		app.world()
			.resource::<ActionState>()
			.axis2d(TestAction::Move),
		Vec2::new(0.0, 1.0)
	);

	app.world_mut()
		.resource_mut::<ButtonInput<KeyCode>>()
		.release(KeyCode::KeyW);
	run_input_frame(&mut app);
	assert_eq!(
		app.world()
			.resource::<ActionState>()
			.axis2d(TestAction::Move),
		Vec2::ZERO
	);
}

#[test]
fn a_composite_can_be_driven_by_a_mouse_button_and_is_scaled() {
	let mut app = app_with_composite_keymap();

	app.world_mut()
		.resource_mut::<ButtonInput<MouseButton>>()
		.press(MouseButton::Left);
	run_input_frame(&mut app);
	assert_eq!(
		app.world()
			.resource::<ActionState>()
			.axis1d(TestAction::Rise),
		3.0
	);

	app.world_mut()
		.resource_mut::<ButtonInput<KeyCode>>()
		.press(KeyCode::KeyC);
	run_input_frame(&mut app);
	assert_eq!(
		app.world()
			.resource::<ActionState>()
			.axis1d(TestAction::Rise),
		0.0
	);
}

#[test]
fn a_composite_does_not_move_while_a_modifier_is_held() {
	let mut app = app_with_composite_keymap();

	// The S of Ctrl+S must not also move the camera backwards.
	let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
	keys.press(KeyCode::ControlLeft);
	keys.press(KeyCode::KeyS);
	run_input_frame(&mut app);

	assert_eq!(
		app.world()
			.resource::<ActionState>()
			.axis2d(TestAction::Move),
		Vec2::ZERO
	);
}
