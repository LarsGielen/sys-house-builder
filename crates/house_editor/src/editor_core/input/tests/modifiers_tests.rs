use super::*;

#[test]
fn parses_modifiers_in_any_order_and_case() {
	let modifiers: Modifiers = "Shift+ctrl".parse().unwrap();

	assert!(modifiers.ctrl && modifiers.shift && !modifiers.alt);
}

#[test]
fn empty_text_is_no_modifiers() {
	assert_eq!("".parse::<Modifiers>(), Ok(Modifiers::default()));
}

#[test]
fn rejects_unknown_and_repeated_modifiers() {
	assert!("meta".parse::<Modifiers>().is_err());
	assert!("ctrl+ctrl".parse::<Modifiers>().is_err());
	assert!("ctrl+".parse::<Modifiers>().is_err());
}

#[test]
fn reads_either_side_of_a_held_modifier() {
	let mut keys = ButtonInput::<KeyCode>::default();
	assert_eq!(Modifiers::from_keys(&keys), Modifiers::default());

	keys.press(KeyCode::ShiftRight);
	keys.press(KeyCode::ControlLeft);
	let held = Modifiers::from_keys(&keys);
	assert!(held.ctrl && held.shift && !held.alt);
}

#[test]
fn displays_as_a_shortcut_prefix() {
	assert_eq!(
		"ctrl+shift".parse::<Modifiers>().unwrap().to_string(),
		"Ctrl+Shift+"
	);
	assert_eq!(Modifiers::default().to_string(), "");
}
