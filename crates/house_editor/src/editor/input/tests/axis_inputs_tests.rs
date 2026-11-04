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
