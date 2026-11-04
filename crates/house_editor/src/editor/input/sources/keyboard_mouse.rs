use std::collections::HashSet;

use bevy::{log::warn_once, prelude::*};

use crate::editor::input::{ActionState, Keymap, Shortcut, signal::Signal};

/// Turns keyboard and mouse button shortcuts into button contributions.
///
/// A shortcut starts only on the frame its input is pressed with exactly its modifiers. After
/// that it stays active for as long as the input is held, whatever the modifiers do, so
/// releasing Shift in the middle of a Shift+drag does not end the drag.
#[derive(Default)]
pub struct ButtonSource {
	/// Shortcuts that started and whose input has not been released since.
	latched: HashSet<Shortcut>,
}

impl ButtonSource {
	pub fn poll(
		&mut self,
		keymap: &Keymap,
		keys: &ButtonInput<KeyCode>,
		mouse_buttons: &ButtonInput<MouseButton>,
		state: &mut ActionState,
	) {
		for &(shortcut, action_name) in keymap.bindings() {
			if self.latched.contains(&shortcut) {
				if !shortcut.input_held(keys, mouse_buttons) {
					self.latched.remove(&shortcut);
				}
			} else if shortcut.just_pressed(keys, mouse_buttons) {
				self.latched.insert(shortcut);
			}

			if !self.latched.contains(&shortcut) {
				continue;
			}

			// The keymap only binds registered actions, but nothing yet stops a shortcut from being
			// bound to an action that is not a button. Warn once instead of every frame.
			if let Err(error) =
				state.contribute_named(shortcut.source(), action_name, Signal::Button(true))
			{
				warn_once!("cannot contribute to \"{action_name}\": {error:?}");
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::editor::input::{Action, ActionGroup, ValueKind};

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
}
