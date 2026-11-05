use bevy::prelude::{KeyCode, MouseButton};

use super::*;

#[test]
fn parses_every_axis_input_by_its_name_case_insensitively() {
	for input in AxisInput::NAMED {
		assert_eq!(input.name().unwrap().parse::<AxisInput>(), Ok(input));
	}
	assert_eq!(
		"Mouse_Motion".parse::<AxisInput>(),
		Ok(AxisInput::MouseMotion)
	);
	assert_eq!(" scroll ".parse::<AxisInput>(), Ok(AxisInput::Scroll));
}

#[test]
fn rejects_unknown_names_and_shortcuts() {
	assert!("wheel".parse::<AxisInput>().is_err());
	assert!("".parse::<AxisInput>().is_err());
	assert!("shift+scroll".parse::<AxisInput>().is_err());
	assert!("mouse_right".parse::<AxisInput>().is_err());
}

#[test]
fn each_input_produces_one_exact_relative_kind() {
	assert_eq!(
		AxisInput::MouseMotion.value_kind(),
		ValueKind::Axis2D(AxisMode::Relative)
	);
	assert_eq!(
		AxisInput::Scroll.value_kind(),
		ValueKind::Axis1D(AxisMode::Relative)
	);
}

#[test]
fn displays_the_keymap_name() {
	assert_eq!(AxisInput::MouseMotion.to_string(), "mouse_motion");
}

#[test]
fn a_scale_multiplies_each_component_and_a_negative_one_inverts() {
	assert_eq!(
		Scale::One(-2.0).apply(Signal::Axis1D(3.0)),
		Signal::Axis1D(-6.0)
	);
	assert_eq!(
		Scale::Two([0.5, -1.0]).apply(Signal::Axis2D(Vec2::new(4.0, 2.0))),
		Signal::Axis2D(Vec2::new(2.0, -2.0))
	);
}

#[test]
fn the_identity_scale_leaves_a_value_unchanged() {
	let kind_1d = ValueKind::Axis1D(AxisMode::Relative);
	let kind_2d = ValueKind::Axis2D(AxisMode::Relative);

	assert_eq!(
		Scale::identity(kind_1d).apply(Signal::Axis1D(3.0)),
		Signal::Axis1D(3.0)
	);
	assert_eq!(
		Scale::identity(kind_2d).apply(Signal::Axis2D(Vec2::new(1.0, 2.0))),
		Signal::Axis2D(Vec2::new(1.0, 2.0))
	);
}

fn key(key: KeyCode) -> ShortcutInput {
	ShortcutInput::Key(key)
}

fn wasd() -> AxisInput {
	AxisInput::composite_2d(
		key(KeyCode::KeyW),
		key(KeyCode::KeyS),
		key(KeyCode::KeyA),
		key(KeyCode::KeyD),
	)
	.unwrap()
}

#[test]
fn composites_produce_absolute_axes_of_their_dimension() {
	let rise = AxisInput::composite_1d(key(KeyCode::Space), key(KeyCode::KeyC)).unwrap();

	assert_eq!(wasd().value_kind(), ValueKind::Axis2D(AxisMode::Absolute));
	assert_eq!(rise.value_kind(), ValueKind::Axis1D(AxisMode::Absolute));
}

#[test]
fn a_composite_cannot_use_the_same_input_for_two_directions() {
	let same_axis = AxisInput::composite_1d(key(KeyCode::KeyW), key(KeyCode::KeyW));
	assert!(same_axis.is_err());

	let across_axes = AxisInput::composite_2d(
		key(KeyCode::KeyW),
		key(KeyCode::KeyS),
		key(KeyCode::KeyA),
		key(KeyCode::KeyW),
	);
	assert!(across_axes.is_err());
}

#[test]
fn a_composite_is_a_mouse_input_only_when_every_direction_is_a_mouse_button() {
	let mouse = |button| ShortcutInput::Mouse(button);
	let zoom =
		AxisInput::composite_1d(mouse(MouseButton::Left), mouse(MouseButton::Right)).unwrap();
	let mixed = AxisInput::composite_1d(mouse(MouseButton::Left), key(KeyCode::KeyC)).unwrap();

	assert_eq!(zoom.source(), SourceId::MOUSE);
	assert_eq!(mixed.source(), SourceId::KEYBOARD);
	assert_eq!(wasd().source(), SourceId::KEYBOARD);
	assert_eq!(AxisInput::Scroll.source(), SourceId::MOUSE);
}

#[test]
fn composites_have_no_name_and_do_not_parse() {
	assert_eq!(wasd().name(), None);
	assert!("wasd".parse::<AxisInput>().is_err());
}

#[test]
fn a_composite_displays_its_directions() {
	assert_eq!(
		wasd().to_string(),
		"composite 2d (up=w, down=s, left=a, right=d)"
	);
}

#[test]
fn only_composites_have_directions() {
	assert!(AxisInput::MouseMotion.directions().is_empty());
	assert_eq!(wasd().directions().len(), 4);
}
