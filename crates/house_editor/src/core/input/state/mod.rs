use std::collections::HashMap;

use bevy::prelude::*;

use crate::core::input::{
	action::{Action, ActionGroup},
	sources::SourceId,
	value::{Signal, ValueKind},
};

struct ActionSlot {
	value_kind: ValueKind,
	signal: Signal,
	signal_previous: Signal,
	contributions: Vec<(SourceId, Signal)>,
	owner: Option<SourceId>,
	toggled: bool,     // The value of a toggle, which the slot keeps between frames.
	was_pressed: bool, // Whether a toggle was pressed last frame, so that holding the key flips it once.
}

#[derive(Debug)]
pub enum ContributionError {
	Unregistered,
	WrongKind,
}

#[derive(Default, Resource)]
pub struct ActionState {
	slots: HashMap<&'static str, ActionSlot>,
}

impl ActionState {
	pub fn register(&mut self, action: impl Action) {
		self.slots.insert(
			action.name(),
			ActionSlot {
				value_kind: action.value(),
				signal: action.value().default_signal(),
				signal_previous: action.value().default_signal(),
				contributions: Vec::new(),
				owner: None,
				toggled: false,
				was_pressed: false,
			},
		);
	}

	pub fn register_group<Group: ActionGroup>(&mut self) {
		for &action in Group::all() {
			self.register(action);
		}
	}

	/// The registered name equal to `name`, as the `'static` string that identifies the action.
	pub fn registered_name(&self, name: &str) -> Option<&'static str> {
		self.slots.get_key_value(name).map(|(name, _)| *name)
	}

	pub fn kind_of(&self, name: &str) -> Option<ValueKind> {
		self.slots.get(name).map(|slot| slot.value_kind)
	}

	pub fn start_frame(&mut self) {
		for slot in self.slots.values_mut() {
			slot.signal_previous = slot.signal;
			slot.contributions.clear();
		}
	}

	pub fn contribute_named(
		&mut self,
		source: SourceId,
		name: &str,
		signal: Signal,
	) -> Result<(), ContributionError> {
		let Some(slot) = self.slots.get_mut(name) else {
			return Err(ContributionError::Unregistered);
		};

		if !slot.value_kind.accepts(signal) {
			return Err(ContributionError::WrongKind);
		}

		slot.contributions.push((source, signal));
		Ok(())
	}

	pub fn resolve(&mut self) {
		for slot in self.slots.values_mut() {
			slot.resolve();
		}
	}

	pub fn pressed(&self, action: impl Action) -> bool {
		let Some(slot) = self.slots.get(action.name()) else {
			return false;
		};

		matches!(
			(slot.value_kind, slot.signal),
			(ValueKind::Button, Signal::Button(true),)
		)
	}

	pub fn just_pressed(&self, action: impl Action) -> bool {
		let Some(slot) = self.slots.get(action.name()) else {
			return false;
		};

		matches!(
			(slot.value_kind, slot.signal, slot.signal_previous),
			(
				ValueKind::Button,
				Signal::Button(true),
				Signal::Button(false)
			)
		)
	}

	pub fn just_released(&self, action: impl Action) -> bool {
		let Some(slot) = self.slots.get(action.name()) else {
			return false;
		};

		matches!(
			(slot.value_kind, slot.signal, slot.signal_previous),
			(
				ValueKind::Button,
				Signal::Button(false),
				Signal::Button(true)
			)
		)
	}

	/// Whether a toggle is on. False for anything that is not a toggle.
	pub fn toggled(&self, action: impl Action) -> bool {
		self.toggled_named(action.name())
	}

	pub fn toggled_named(&self, name: &str) -> bool {
		self.slots
			.get(name)
			.is_some_and(|slot| slot.value_kind == ValueKind::Toggle && slot.toggled)
	}

	/// Whether a toggle changed since the last frame began.
	pub fn just_toggled(&self, action: impl Action) -> bool {
		self.slots.get(action.name()).is_some_and(|slot| {
			slot.value_kind == ValueKind::Toggle && slot.signal != slot.signal_previous
		})
	}

	pub fn axis1d(&self, action: impl Action) -> f32 {
		match self.slots.get(action.name()).map(|slot| slot.signal) {
			Some(Signal::Axis1D(value)) => value,
			_ => 0.0,
		}
	}

	pub fn axis2d(&self, action: impl Action) -> Vec2 {
		match self.slots.get(action.name()).map(|slot| slot.signal) {
			Some(Signal::Axis2D(value)) => value,
			_ => Vec2::ZERO,
		}
	}
}

mod arbitration;

#[cfg(test)]
mod tests {
	use crate::core::input::value::AxisMode;

	use super::*;

	#[derive(Clone, Copy)]
	enum TestAction {
		Jump,
		Move,
		Zoom,
		Look,
		Rise,
		Snap,
	}

	impl Action for TestAction {
		fn name(self) -> &'static str {
			match self {
				TestAction::Jump => "test.jump",
				TestAction::Move => "test.move",
				TestAction::Zoom => "test.zoom",
				TestAction::Look => "test.look",
				TestAction::Rise => "test.rise",
				TestAction::Snap => "test.snap",
			}
		}
		fn value(self) -> ValueKind {
			match self {
				TestAction::Jump => ValueKind::Button,
				TestAction::Move => ValueKind::Axis2D(AxisMode::Absolute),
				TestAction::Zoom => ValueKind::Axis1D(AxisMode::Relative),
				TestAction::Look => ValueKind::Axis2D(AxisMode::Relative),
				TestAction::Rise => ValueKind::Axis1D(AxisMode::Absolute),
				TestAction::Snap => ValueKind::Toggle,
			}
		}
	}

	use TestAction::{Jump, Look, Move, Rise, Snap, Zoom};

	const CONTROLLER: SourceId = SourceId(3);
	const UI: SourceId = SourceId(4);

	fn state_with(actions: &[TestAction]) -> ActionState {
		let mut state = ActionState::default();
		for &action in actions {
			state.register(action);
		}
		state
	}

	/// Runs one full frame: start, contribute everything in the given order, resolve.
	fn frame(state: &mut ActionState, contributions: &[(SourceId, TestAction, Signal)]) {
		state.start_frame();
		for &(source, action, signal) in contributions {
			state
				.contribute_named(source, action.name(), signal)
				.unwrap();
		}
		state.resolve();
	}

	#[test]
	fn a_held_button_is_just_pressed_only_on_the_first_frame() {
		let mut state = ActionState::default();
		state.register(TestAction::Jump);
		let keyboard = SourceId(0);

		state.start_frame();
		state
			.contribute_named(keyboard, TestAction::Jump.name(), Signal::Button(true))
			.unwrap();
		state.resolve();
		assert!(state.pressed(TestAction::Jump));
		assert!(state.just_pressed(TestAction::Jump));

		state.start_frame();
		state
			.contribute_named(keyboard, TestAction::Jump.name(), Signal::Button(true))
			.unwrap();
		state.resolve();
		assert!(state.pressed(TestAction::Jump));
		assert!(!state.just_pressed(TestAction::Jump));
	}

	#[test]
	fn a_source_that_stops_contributing_releases_the_button() {
		let mut state = state_with(&[Jump]);

		frame(
			&mut state,
			&[(SourceId::KEYBOARD, Jump, Signal::Button(true))],
		);
		assert!(state.pressed(Jump));
		assert!(state.just_pressed(Jump));

		frame(&mut state, &[]);
		assert!(!state.pressed(Jump));
		assert!(state.just_released(Jump));

		frame(&mut state, &[]);
		assert!(!state.pressed(Jump));
		assert!(!state.just_released(Jump));
	}

	#[test]
	fn a_button_stays_pressed_while_any_source_holds_it() {
		let mut state = state_with(&[Jump]);

		frame(
			&mut state,
			&[
				(SourceId::KEYBOARD, Jump, Signal::Button(true)),
				(UI, Jump, Signal::Button(true)),
			],
		);
		assert!(state.pressed(Jump));

		frame(&mut state, &[(UI, Jump, Signal::Button(true))]);
		assert!(state.pressed(Jump));
		assert!(!state.just_released(Jump));

		frame(&mut state, &[]);
		assert!(!state.pressed(Jump));
		assert!(state.just_released(Jump));
	}

	#[test]
	fn a_second_source_pressing_does_not_count_as_a_new_press() {
		let mut state = state_with(&[Jump]);

		frame(
			&mut state,
			&[(SourceId::KEYBOARD, Jump, Signal::Button(true))],
		);
		assert!(state.just_pressed(Jump));

		frame(
			&mut state,
			&[
				(SourceId::KEYBOARD, Jump, Signal::Button(true)),
				(UI, Jump, Signal::Button(true)),
			],
		);
		assert!(state.pressed(Jump));
		assert!(!state.just_pressed(Jump));
	}

	#[test]
	fn a_contribution_of_false_is_the_same_as_not_contributing() {
		let mut state = state_with(&[Jump]);

		frame(
			&mut state,
			&[(SourceId::KEYBOARD, Jump, Signal::Button(false))],
		);
		assert!(!state.pressed(Jump));
		assert!(!state.just_pressed(Jump));
	}

	#[test]
	fn contributing_to_an_unregistered_action_is_an_error_and_changes_nothing() {
		let mut state = ActionState::default();

		state.start_frame();
		let result = state.contribute_named(SourceId::KEYBOARD, Jump.name(), Signal::Button(true));
		state.resolve();

		assert!(matches!(result, Err(ContributionError::Unregistered)));
		assert!(!state.pressed(Jump));
	}

	#[test]
	fn contributing_the_wrong_signal_kind_is_an_error_and_changes_nothing() {
		let mut state = state_with(&[Jump, Move]);

		state.start_frame();
		let axis_to_button =
			state.contribute_named(SourceId::KEYBOARD, Jump.name(), Signal::Axis1D(1.0));
		let button_to_axis =
			state.contribute_named(SourceId::KEYBOARD, Move.name(), Signal::Button(true));
		let wrong_dimension =
			state.contribute_named(SourceId::KEYBOARD, Move.name(), Signal::Axis1D(1.0));
		state.resolve();

		assert!(matches!(axis_to_button, Err(ContributionError::WrongKind)));
		assert!(matches!(button_to_axis, Err(ContributionError::WrongKind)));
		assert!(matches!(wrong_dimension, Err(ContributionError::WrongKind)));
		assert!(!state.pressed(Jump));
		assert_eq!(state.axis2d(Move), Vec2::ZERO);
	}

	#[test]
	fn a_rejected_contribution_does_not_affect_valid_ones_in_the_same_frame() {
		let mut state = state_with(&[Jump]);

		state.start_frame();
		state
			.contribute_named(SourceId::KEYBOARD, Jump.name(), Signal::Button(true))
			.unwrap();
		assert!(
			state
				.contribute_named(UI, Jump.name(), Signal::Axis1D(1.0))
				.is_err()
		);
		state.resolve();

		assert!(state.pressed(Jump));
	}

	#[test]
	fn an_axis_passes_a_single_source_through() {
		let mut state = state_with(&[Move]);

		frame(
			&mut state,
			&[(
				SourceId::KEYBOARD,
				Move,
				Signal::Axis2D(Vec2::new(1.0, 0.0)),
			)],
		);
		assert_eq!(state.axis2d(Move), Vec2::new(1.0, 0.0));
	}

	#[test]
	fn an_axis_returns_to_neutral_when_the_source_stops_contributing() {
		let mut state = state_with(&[Move]);

		frame(
			&mut state,
			&[(
				SourceId::KEYBOARD,
				Move,
				Signal::Axis2D(Vec2::new(1.0, 0.0)),
			)],
		);
		frame(&mut state, &[]);
		assert_eq!(state.axis2d(Move), Vec2::ZERO);
	}

	#[test]
	fn an_axis_never_sums_two_sources() {
		let mut state = state_with(&[Move]);

		// Both leave neutral in the same frame. Ties go to the first contribution.
		frame(
			&mut state,
			&[
				(
					SourceId::KEYBOARD,
					Move,
					Signal::Axis2D(Vec2::new(1.0, 0.0)),
				),
				(CONTROLLER, Move, Signal::Axis2D(Vec2::new(0.0, 1.0))),
			],
		);
		assert_eq!(state.axis2d(Move), Vec2::new(1.0, 0.0));
	}

	#[test]
	fn the_first_source_to_leave_neutral_owns_the_axis() {
		let mut state = state_with(&[Move]);

		frame(
			&mut state,
			&[(
				SourceId::KEYBOARD,
				Move,
				Signal::Axis2D(Vec2::new(1.0, 0.0)),
			)],
		);

		// The controller is listed first, so a pass would only win by order, not by ownership.
		frame(
			&mut state,
			&[
				(CONTROLLER, Move, Signal::Axis2D(Vec2::new(0.0, 1.0))),
				(
					SourceId::KEYBOARD,
					Move,
					Signal::Axis2D(Vec2::new(1.0, 0.0)),
				),
			],
		);
		assert_eq!(state.axis2d(Move), Vec2::new(1.0, 0.0));
	}

	#[test]
	fn an_owner_keeps_the_axis_until_it_returns_to_neutral() {
		let mut state = state_with(&[Move]);

		frame(
			&mut state,
			&[(
				SourceId::KEYBOARD,
				Move,
				Signal::Axis2D(Vec2::new(1.0, 0.0)),
			)],
		);
		frame(
			&mut state,
			&[
				(
					SourceId::KEYBOARD,
					Move,
					Signal::Axis2D(Vec2::new(0.5, 0.0)),
				),
				(CONTROLLER, Move, Signal::Axis2D(Vec2::new(0.0, 1.0))),
			],
		);
		assert_eq!(state.axis2d(Move), Vec2::new(0.5, 0.0));
	}

	#[test]
	fn another_source_can_take_over_in_the_frame_the_owner_returns_to_neutral() {
		let mut state = state_with(&[Move]);

		frame(
			&mut state,
			&[(
				SourceId::KEYBOARD,
				Move,
				Signal::Axis2D(Vec2::new(1.0, 0.0)),
			)],
		);

		// Same-frame handover (recommended). If you choose a one-frame gap instead, the
		// expected value here becomes Vec2::ZERO and the next frame hands over.
		frame(
			&mut state,
			&[
				(SourceId::KEYBOARD, Move, Signal::Axis2D(Vec2::ZERO)),
				(CONTROLLER, Move, Signal::Axis2D(Vec2::new(0.0, 1.0))),
			],
		);
		assert_eq!(state.axis2d(Move), Vec2::new(0.0, 1.0));
	}

	#[test]
	fn a_source_that_stops_contributing_gives_up_ownership() {
		let mut state = state_with(&[Move]);

		frame(
			&mut state,
			&[(
				SourceId::KEYBOARD,
				Move,
				Signal::Axis2D(Vec2::new(1.0, 0.0)),
			)],
		);
		frame(&mut state, &[]);
		assert_eq!(state.axis2d(Move), Vec2::ZERO);

		frame(
			&mut state,
			&[(CONTROLLER, Move, Signal::Axis2D(Vec2::new(0.0, 1.0)))],
		);
		assert_eq!(state.axis2d(Move), Vec2::new(0.0, 1.0));
	}

	#[test]
	fn a_non_owner_at_neutral_does_not_disturb_the_owner() {
		let mut state = state_with(&[Move]);

		frame(
			&mut state,
			&[(
				SourceId::KEYBOARD,
				Move,
				Signal::Axis2D(Vec2::new(1.0, 0.0)),
			)],
		);
		frame(
			&mut state,
			&[
				(
					SourceId::KEYBOARD,
					Move,
					Signal::Axis2D(Vec2::new(1.0, 0.0)),
				),
				(CONTROLLER, Move, Signal::Axis2D(Vec2::ZERO)),
			],
		);
		assert_eq!(state.axis2d(Move), Vec2::new(1.0, 0.0));
	}

	#[test]
	fn a_relative_axis_reports_this_frames_delta_and_resets_to_zero() {
		let mut state = state_with(&[Look]);

		frame(
			&mut state,
			&[(SourceId::MOUSE, Look, Signal::Axis2D(Vec2::new(3.0, -2.0)))],
		);
		assert_eq!(state.axis2d(Look), Vec2::new(3.0, -2.0));

		// A delta must not linger into a frame without motion.
		frame(&mut state, &[]);
		assert_eq!(state.axis2d(Look), Vec2::ZERO);
	}

	#[test]
	fn a_one_dimensional_axis_works_like_the_two_dimensional_one() {
		let mut state = state_with(&[Zoom]);

		frame(&mut state, &[(SourceId::MOUSE, Zoom, Signal::Axis1D(1.0))]);
		assert_eq!(state.axis1d(Zoom), 1.0);

		frame(&mut state, &[]);
		assert_eq!(state.axis1d(Zoom), 0.0);
	}

	#[test]
	fn a_freshly_registered_action_is_neutral_and_has_no_edges() {
		let state = state_with(&[Jump, Move]);

		assert!(!state.pressed(Jump));
		assert!(!state.just_pressed(Jump));
		assert!(!state.just_released(Jump));
		assert_eq!(state.axis2d(Move), Vec2::ZERO);
	}

	#[test]
	fn querying_an_unregistered_action_returns_neutral_values() {
		let state = ActionState::default();

		assert!(!state.pressed(Jump));
		assert!(!state.just_pressed(Jump));
		assert!(!state.just_released(Jump));
		assert_eq!(state.axis1d(Zoom), 0.0);
		assert_eq!(state.axis2d(Move), Vec2::ZERO);
	}

	#[test]
	fn registering_an_action_again_resets_its_slot() {
		let mut state = state_with(&[Jump]);

		frame(
			&mut state,
			&[(SourceId::KEYBOARD, Jump, Signal::Button(true))],
		);
		state.register(Jump);
		assert!(!state.pressed(Jump));
	}

	#[test]
	fn bindings_of_one_source_on_an_absolute_axis_never_exceed_the_strongest_one() {
		let mut state = state_with(&[Move, Rise]);

		// WASD and the arrow keys are both forward.
		frame(
			&mut state,
			&[
				(SourceId::KEYBOARD, Move, Signal::Axis2D(Vec2::Y)),
				(SourceId::KEYBOARD, Move, Signal::Axis2D(Vec2::Y)),
				(SourceId::KEYBOARD, Rise, Signal::Axis1D(1.0)),
				(SourceId::KEYBOARD, Rise, Signal::Axis1D(1.0)),
			],
		);
		assert_eq!(state.axis2d(Move), Vec2::Y);
		assert_eq!(state.axis1d(Rise), 1.0);

		// Forward on one binding and sideways on the other stays a unit stick.
		frame(
			&mut state,
			&[
				(SourceId::KEYBOARD, Move, Signal::Axis2D(Vec2::Y)),
				(SourceId::KEYBOARD, Move, Signal::Axis2D(Vec2::X)),
			],
		);
		assert!((state.axis2d(Move).length() - 1.0).abs() < 1e-6);
	}

	#[test]
	fn opposite_bindings_of_one_source_still_cancel_on_an_absolute_axis() {
		let mut state = state_with(&[Move, Rise]);

		frame(
			&mut state,
			&[
				(SourceId::KEYBOARD, Move, Signal::Axis2D(Vec2::Y)),
				(SourceId::KEYBOARD, Move, Signal::Axis2D(Vec2::NEG_Y)),
				(SourceId::KEYBOARD, Rise, Signal::Axis1D(1.0)),
				(SourceId::KEYBOARD, Rise, Signal::Axis1D(-1.0)),
			],
		);

		assert_eq!(state.axis2d(Move), Vec2::ZERO);
		assert_eq!(state.axis1d(Rise), 0.0);
	}

	#[test]
	fn a_single_scaled_binding_on_an_absolute_axis_is_not_clamped() {
		let mut state = state_with(&[Move]);

		frame(
			&mut state,
			&[(
				SourceId::KEYBOARD,
				Move,
				Signal::Axis2D(Vec2::new(0.0, 2.0)),
			)],
		);

		assert_eq!(state.axis2d(Move), Vec2::new(0.0, 2.0));
	}

	#[test]
	fn relative_axes_still_sum_within_one_source() {
		let mut state = state_with(&[Look, Zoom]);

		frame(
			&mut state,
			&[
				(SourceId::MOUSE, Look, Signal::Axis2D(Vec2::new(3.0, 0.0))),
				(SourceId::MOUSE, Look, Signal::Axis2D(Vec2::new(4.0, 0.0))),
				(SourceId::MOUSE, Zoom, Signal::Axis1D(1.0)),
				(SourceId::MOUSE, Zoom, Signal::Axis1D(2.0)),
			],
		);

		assert_eq!(state.axis2d(Look), Vec2::new(7.0, 0.0));
		assert_eq!(state.axis1d(Zoom), 3.0);
	}

	// ## Toggles ## //

	const PRESS: Signal = Signal::Button(true);

	#[test]
	fn a_press_flips_a_toggle_and_the_next_press_flips_it_back() {
		let mut state = state_with(&[Snap]);
		assert!(!state.toggled(Snap));

		frame(&mut state, &[(SourceId::KEYBOARD, Snap, PRESS)]);
		assert!(state.toggled(Snap));
		assert!(state.just_toggled(Snap));

		frame(&mut state, &[]);
		assert!(state.toggled(Snap), "the value outlives the press");
		assert!(!state.just_toggled(Snap));

		frame(&mut state, &[(SourceId::KEYBOARD, Snap, PRESS)]);
		assert!(!state.toggled(Snap));
		assert!(state.just_toggled(Snap));
	}

	#[test]
	fn a_held_press_flips_a_toggle_once() {
		let mut state = state_with(&[Snap]);

		for _ in 0..3 {
			frame(&mut state, &[(SourceId::KEYBOARD, Snap, PRESS)]);
		}

		assert!(state.toggled(Snap));
	}

	#[test]
	fn two_sources_pressing_together_flip_a_toggle_once() {
		let mut state = state_with(&[Snap]);

		frame(
			&mut state,
			&[
				(SourceId::KEYBOARD, Snap, PRESS),
				(SourceId::UI, Snap, PRESS),
			],
		);

		assert!(state.toggled(Snap));
	}

	#[test]
	fn a_toggle_takes_button_signals_only() {
		let mut state = state_with(&[Snap]);

		assert!(
			state
				.contribute_named(SourceId::UI, Snap.name(), Signal::Axis1D(1.0))
				.is_err()
		);
		assert!(
			state
				.contribute_named(SourceId::UI, Snap.name(), PRESS)
				.is_ok()
		);
	}

	#[test]
	fn a_toggle_is_not_a_pressed_button() {
		let mut state = state_with(&[Snap]);

		frame(&mut state, &[(SourceId::KEYBOARD, Snap, PRESS)]);

		assert!(!state.pressed(Snap));
		assert!(!state.just_pressed(Snap));
	}
}
