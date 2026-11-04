use super::*;

#[test]
fn parses_every_axis_input_by_its_name_case_insensitively() {
	for input in AxisInput::ALL {
		assert_eq!(input.name().parse::<AxisInput>(), Ok(input));
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
