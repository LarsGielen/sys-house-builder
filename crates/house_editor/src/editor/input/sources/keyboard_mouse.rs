use std::collections::HashSet;

use bevy::{
	input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit},
	log::warn_once,
	prelude::*,
};

use crate::editor::input::{
	ActionState, AxisInput, Keymap, Shortcut, signal::Signal, state::SourceId,
};

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
		for &(shortcut, action_name) in keymap.button_bindings() {
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

			// The keymap only binds registered button actions, so this is a bug if it ever fails.
			// Warn once instead of every frame.
			if let Err(error) =
				state.contribute_named(shortcut.source(), action_name, Signal::Button(true))
			{
				warn_once!("cannot contribute to \"{action_name}\": {error:?}");
			}
		}
	}
}

/// How many pixels one scroll line is worth. Trackpads report pixels and wheels report lines, so
/// scroll is converted to lines to give actions one unit regardless of the device.
const PIXELS_PER_LINE: f32 = 40.0;

/// Contributes the mouse motion and scroll of this frame to the actions they are bound to.
pub fn poll_mouse_axes(
	keymap: &Keymap,
	motion: &AccumulatedMouseMotion,
	scroll: &AccumulatedMouseScroll,
	state: &mut ActionState,
) {
	for &(input, action_name) in keymap.axis_bindings() {
		let Some(signal) = axis_signal(input, motion, scroll) else {
			continue;
		};

		// The keymap only binds registered actions of the matching kind, so this is a bug if it
		// ever fails. Warn once instead of every frame.
		if let Err(error) = state.contribute_named(SourceId::MOUSE, action_name, signal) {
			warn_once!("cannot contribute to \"{action_name}\": {error:?}");
		}
	}
}

/// The signal of an axis input this frame, or `None` when it did not move
fn axis_signal(
	input: AxisInput,
	motion: &AccumulatedMouseMotion,
	scroll: &AccumulatedMouseScroll,
) -> Option<Signal> {
	match input {
		AxisInput::MouseMotion => {
			(motion.delta != Vec2::ZERO).then_some(Signal::Axis2D(motion.delta))
		}

		// Horizontal scroll is ignored: nothing is bound to it yet.
		AxisInput::Scroll => {
			let lines = match scroll.unit {
				MouseScrollUnit::Line => scroll.delta.y,
				MouseScrollUnit::Pixel => scroll.delta.y / PIXELS_PER_LINE,
			};
			(lines != 0.0).then_some(Signal::Axis1D(lines))
		}
	}
}

#[cfg(test)]
#[path = "../tests/sources_keyboard_mouse_tests.rs"]
mod tests;
