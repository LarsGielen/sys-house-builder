use super::*;
use crate::tests::support::{press_key, tool_test_app};
use bevy::{input_focus::InputFocus, text::EditableText};

fn actions_app() -> App {
	let mut app = tool_test_app();
	app.add_plugins(ActionsPlugin);
	app
}

fn press_keys(app: &mut App, keys: &[KeyCode]) {
	let mut keyboard = ButtonInput::<KeyCode>::default();
	for &key in keys {
		keyboard.press(key);
	}
	app.insert_resource(keyboard);
}

fn state(app: &App) -> &ActionState {
	app.world().resource::<ActionState>()
}

fn focus_field(app: &mut App) {
	let field = app.world_mut().spawn(EditableText::new("")).id();
	app.insert_resource(InputFocus::from_entity(field));
}

#[test]
fn escape_cancels_in_the_viewport_and_cancels_the_field_when_typing() {
	let mut app = actions_app();
	press_key(&mut app, KeyCode::Escape);
	app.update();
	assert!(state(&app).just_pressed(Action::Cancel));
	assert!(!state(&app).just_pressed(Action::FieldCancel));

	let mut app = actions_app();
	focus_field(&mut app);
	press_key(&mut app, KeyCode::Escape);
	app.update();
	assert!(state(&app).just_pressed(Action::FieldCancel));
	assert!(!state(&app).just_pressed(Action::Cancel));
}

#[test]
fn enter_confirms_or_submits_by_context() {
	let mut app = actions_app();
	press_key(&mut app, KeyCode::NumpadEnter);
	app.update();
	assert!(state(&app).just_pressed(Action::Confirm));

	let mut app = actions_app();
	focus_field(&mut app);
	press_key(&mut app, KeyCode::Enter);
	app.update();
	assert!(state(&app).just_pressed(Action::FieldSubmit));
	assert!(!state(&app).just_pressed(Action::Confirm));
}

#[test]
fn control_s_saves_and_does_not_pan() {
	let mut app = actions_app();
	press_keys(&mut app, &[KeyCode::ControlLeft, KeyCode::KeyS]);
	app.update();
	assert!(state(&app).just_pressed(Action::Save));
	assert!(!state(&app).pressed(Action::PanBack));

	let mut app = actions_app();
	press_keys(&mut app, &[KeyCode::KeyS]);
	app.update();
	assert!(state(&app).pressed(Action::PanBack));
	assert!(!state(&app).just_pressed(Action::Save));
}

#[test]
fn shortcuts_are_not_produced_while_typing() {
	let mut app = actions_app();
	focus_field(&mut app);
	press_keys(&mut app, &[KeyCode::ControlLeft, KeyCode::KeyZ]);
	app.update();
	assert!(!state(&app).just_pressed(Action::Undo));
}

#[test]
fn redo_has_two_default_bindings() {
	for keys in [
		&[KeyCode::ControlLeft, KeyCode::ShiftLeft, KeyCode::KeyZ][..],
		&[KeyCode::ControlRight, KeyCode::KeyY][..],
	] {
		let mut app = actions_app();
		press_keys(&mut app, keys);
		app.update();
		assert!(state(&app).just_pressed(Action::Redo));
		assert!(!state(&app).just_pressed(Action::Undo));
	}
}

#[test]
fn rebinding_moves_an_action_to_the_new_key_only() {
	let mut app = actions_app();
	app.world_mut()
		.resource_mut::<Bindings>()
		.0
		.retain(|binding| binding.action != Action::CameraReset);
	app.world_mut().resource_mut::<Bindings>().0.push(Binding {
		trigger: Trigger::Key(KeyCode::KeyR),
		control: ModifierRule::Ignored,
		shift: ModifierRule::Ignored,
		context: Context::Always,
		action: Action::CameraReset,
	});

	press_key(&mut app, KeyCode::Home);
	app.update();
	assert!(!state(&app).just_pressed(Action::CameraReset));

	press_key(&mut app, KeyCode::KeyR);
	app.update();
	assert!(state(&app).just_pressed(Action::CameraReset));
}

#[test]
fn an_unfocused_window_produces_no_actions() {
	let mut app = actions_app();
	app.world_mut()
		.query_filtered::<&mut Window, With<PrimaryWindow>>()
		.single_mut(app.world_mut())
		.unwrap()
		.focused = false;
	press_key(&mut app, KeyCode::Escape);
	app.update();
	assert!(!state(&app).just_pressed(Action::Cancel));
}

#[test]
fn held_actions_report_press_and_release_edges() {
	let mut app = actions_app();
	let mut mouse = ButtonInput::<MouseButton>::default();
	mouse.press(MouseButton::Left);
	app.insert_resource(mouse);
	app.update();
	assert!(state(&app).just_pressed(Action::ToolPress));
	assert!(state(&app).pressed(Action::ToolPress));

	let mut mouse = ButtonInput::<MouseButton>::default();
	mouse.press(MouseButton::Left);
	mouse.clear();
	mouse.release(MouseButton::Left);
	app.insert_resource(mouse);
	app.update();
	assert!(state(&app).just_released(Action::ToolPress));
	assert!(!state(&app).pressed(Action::ToolPress));
}
