use super::*;
use crate::core::input::modifiers::{Modifier, Modifiers};

#[test]
fn shortcut_parsing_is_case_insensitive_and_checks_modifiers() {
	let shortcut: Shortcut = "Ctrl+Shift+Z".parse().unwrap();
	assert_eq!(shortcut.input, ShortcutInput::Key(KeyCode::KeyZ));
	assert!(shortcut.modifiers.ctrl && shortcut.modifiers.shift && !shortcut.modifiers.alt);

	assert!("ctrl+ctrl+z".parse::<Shortcut>().is_err());
	assert!("meta+z".parse::<Shortcut>().is_err());
	assert!("ctrl+".parse::<Shortcut>().is_err());
}

#[test]
fn displays_shortcuts_for_the_user() {
	let shortcut: Shortcut = "ctrl+shift+z".parse().unwrap();
	assert_eq!(shortcut.to_string(), "Ctrl+Shift+Z");

	let shortcut: Shortcut = "f5".parse().unwrap();
	assert_eq!(shortcut.to_string(), "F5");
}

#[test]
fn parses_mouse_buttons_without_confusing_them_with_arrow_keys() {
	let shortcut: Shortcut = "shift+mouse_middle".parse().unwrap();
	assert_eq!(shortcut.input, ShortcutInput::Mouse(MouseButton::Middle));
	assert!(shortcut.modifiers.shift);

	let arrow: Shortcut = "ctrl+left".parse().unwrap();
	assert_eq!(arrow.input, ShortcutInput::Key(KeyCode::ArrowLeft));

	assert!("mouse_back".parse::<Shortcut>().is_err());
}

#[test]
fn displays_mouse_shortcuts_for_the_user() {
	let shortcut: Shortcut = "shift+mouse_middle".parse().unwrap();
	assert_eq!(shortcut.to_string(), "Shift+Mouse middle");
}

#[test]
fn mouse_shortcut_requires_exact_modifiers_when_pressed() {
	let orbit: Shortcut = "mouse_middle".parse().unwrap();
	let pan: Shortcut = "shift+mouse_middle".parse().unwrap();

	let mut keys = ButtonInput::<KeyCode>::default();
	let mut mouse_buttons = ButtonInput::<MouseButton>::default();
	keys.press(KeyCode::ShiftLeft);
	mouse_buttons.press(MouseButton::Middle);

	assert!(pan.just_pressed(&keys, &mouse_buttons));
	assert!(!orbit.just_pressed(&keys, &mouse_buttons));
}

#[test]
fn a_modifier_key_is_a_shortcut_input() {
	let shift: Shortcut = "shift".parse().unwrap();
	assert_eq!(shift.input, ShortcutInput::Modifier(Modifier::Shift));
	assert_eq!(shift.modifiers, Modifiers::default());
	assert_eq!(shift.to_string(), "Shift");

	let both: Shortcut = "ctrl+shift".parse().unwrap();
	assert_eq!(both.input, ShortcutInput::Modifier(Modifier::Shift));
	assert!(both.modifiers.ctrl && !both.modifiers.shift);
	assert_eq!(both.to_string(), "Ctrl+Shift");
}

#[test]
fn a_modifier_key_cannot_be_its_own_modifier() {
	assert!("shift+shift".parse::<Shortcut>().is_err());
	assert!("ctrl+shift+ctrl".parse::<Shortcut>().is_err());
}

#[test]
fn a_modifier_shortcut_needs_only_the_other_modifiers() {
	let mut keys = ButtonInput::<KeyCode>::default();
	let mouse = ButtonInput::<MouseButton>::default();
	let shift: Shortcut = "shift".parse().unwrap();
	let ctrl_shift: Shortcut = "ctrl+shift".parse().unwrap();

	// Either shift key counts, and Shift itself is not an extra modifier.
	keys.press(KeyCode::ShiftRight);
	assert!(shift.just_pressed(&keys, &mouse));
	assert!(!ctrl_shift.just_pressed(&keys, &mouse));

	keys.clear();
	keys.press(KeyCode::ControlLeft);
	keys.clear();
	keys.press(KeyCode::ShiftLeft);
	assert!(ctrl_shift.just_pressed(&keys, &mouse));
	assert!(!shift.just_pressed(&keys, &mouse));
}
