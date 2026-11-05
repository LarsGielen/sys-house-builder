use super::*;
use crate::editor_core::input::{
	ActionGroup, ActionState, AxisMode, InputAppExt, InputPlugin, ValueKind, action::Action,
};

#[derive(Clone, Copy)]
enum TestAction {
	Click,
	Slide,
}

impl Action for TestAction {
	fn name(self) -> &'static str {
		match self {
			Self::Click => "test.click",
			Self::Slide => "test.slide",
		}
	}

	fn value(self) -> ValueKind {
		match self {
			Self::Click => ValueKind::Button,
			Self::Slide => ValueKind::Axis1D(AxisMode::Absolute),
		}
	}
}

impl ActionGroup for TestAction {
	fn all() -> &'static [Self] {
		&[Self::Click, Self::Slide]
	}
}

fn app_with_ui_source() -> App {
	let mut app = App::new();
	app.add_plugins(InputPlugin)
		.add_input_actions::<TestAction>();
	app
}

fn click(app: &mut App, action: &'static str) {
	app.world_mut().trigger(UiActivated::button(action));
}

fn run_input_frame(app: &mut App) {
	app.world_mut().run_schedule(PreUpdate);
}

#[test]
fn a_ui_activation_presses_the_action_for_one_frame() {
	let mut app = app_with_ui_source();

	click(&mut app, "test.click");
	run_input_frame(&mut app);
	let state = app.world().resource::<ActionState>();
	assert!(state.pressed(TestAction::Click));
	assert!(state.just_pressed(TestAction::Click));

	run_input_frame(&mut app);
	let state = app.world().resource::<ActionState>();
	assert!(!state.pressed(TestAction::Click));
	assert!(!state.just_pressed(TestAction::Click));
}

#[test]
fn an_activation_for_an_unknown_action_is_ignored() {
	let mut app = app_with_ui_source();

	click(&mut app, "test.missing");
	run_input_frame(&mut app);

	assert!(
		!app.world()
			.resource::<ActionState>()
			.pressed(TestAction::Click)
	);
}

#[test]
fn an_activation_with_the_wrong_kind_of_signal_is_ignored() {
	let mut app = app_with_ui_source();

	// A button press cannot drive an axis action.
	click(&mut app, "test.slide");
	run_input_frame(&mut app);

	assert_eq!(
		app.world()
			.resource::<ActionState>()
			.axis1d(TestAction::Slide),
		0.0
	);
}
