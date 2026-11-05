use std::collections::HashSet;

use bevy::{
	input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit},
	log::warn_once,
	prelude::*,
};

use crate::editor_core::input::{
	ActionState, AxisInput, Keymap, Shortcut, ShortcutInput, modifiers::Modifiers, signal::Signal,
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

/// Contributes every axis binding that has a value this frame: the mouse motion and scroll, and
/// the buttons held for composites.
pub fn poll_axis_bindings(
	keymap: &Keymap,
	keys: &ButtonInput<KeyCode>,
	mouse_buttons: &ButtonInput<MouseButton>,
	motion: &AccumulatedMouseMotion,
	scroll: &AccumulatedMouseScroll,
	state: &mut ActionState,
) {
	let held_modifiers = Modifiers::from_keys(keys);
	for binding in keymap.axis_bindings() {
		if binding.modifiers != held_modifiers {
			continue;
		}
		let Some(signal) = axis_signal(binding.input, motion, scroll)
			.or_else(|| composite_signal(binding.input, keys, mouse_buttons))
		else {
			continue;
		};

		let signal = binding.scale.apply(signal);
		let action_name = binding.action;

		// The keymap only binds registered actions of the matching kind, so this is a bug if it
		// ever fails. Warn once instead of every frame.
		if let Err(error) = state.contribute_named(binding.input.source(), action_name, signal) {
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

		// Read from the buttons, see `composite_signal`.
		AxisInput::Composite1D { .. } | AxisInput::Composite2D { .. } => None,
	}
}

/// The signal of a composite from the directions held right now, or `None` when nothing is held
/// or opposite directions cancel out. Diagonals are normalised so that they are not faster.
fn composite_signal(
	input: AxisInput,
	keys: &ButtonInput<KeyCode>,
	mouse_buttons: &ButtonInput<MouseButton>,
) -> Option<Signal> {
	let held = |direction: ShortcutInput| {
		if direction.pressed(keys, mouse_buttons) {
			1.0
		} else {
			0.0
		}
	};

	match input {
		AxisInput::Composite1D { up, down } => {
			let value = held(up) - held(down);
			(value != 0.0).then_some(Signal::Axis1D(value))
		}
		AxisInput::Composite2D {
			up,
			down,
			left,
			right,
		} => {
			let value = Vec2::new(held(right) - held(left), held(up) - held(down));
			(value != Vec2::ZERO).then(|| Signal::Axis2D(value.normalize()))
		}
		AxisInput::MouseMotion | AxisInput::Scroll => None,
	}
}

#[cfg(test)]
#[path = "../tests/sources_keyboard_mouse_tests.rs"]
mod tests;
