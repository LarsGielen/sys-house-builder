use bevy::prelude::*;

use crate::core::input::{action::Action, sources::SourceId, state::ActionState, value::Signal};

/// A widget was activated and stands for the action it names. The sources contribute it to the
/// action as a press for one frame, like any other input.
#[derive(Event)]
pub struct UiActivated {
	action: &'static str,
	signal: Signal,
}

/// Buffers an activation from `Update` until the sources run in the next `PreUpdate`.
#[derive(Message)]
pub(in crate::core::input) struct PendingUiActivation {
	action: &'static str,
	signal: Signal,
}

/// Marks a widget as standing for an action. `None` is a widget with no action yet.
#[derive(Component, Clone, Copy, Default)]
pub struct UiAction(pub Option<&'static str>);

impl<A: Action> From<A> for UiAction {
	fn from(action: A) -> Self {
		Self(Some(action.name()))
	}
}

impl UiActivated {
	pub fn button(action: &'static str) -> UiActivated {
		UiActivated {
			action,
			signal: Signal::Button(true),
		}
	}
}

pub(in crate::core::input) fn handle_ui_activations(
	activation: On<UiActivated>,
	mut activations: MessageWriter<PendingUiActivation>,
) {
	activations.write(PendingUiActivation {
		action: activation.action,
		signal: activation.signal,
	});
}

pub(in crate::core::input) fn poll_ui_activations(
	mut message_reader: MessageReader<PendingUiActivation>,
	mut state: ResMut<ActionState>,
) {
	for message in message_reader.read() {
		let action_name = message.action;
		if let Err(error) = state.contribute_named(SourceId::UI, action_name, message.signal) {
			warn_once!("cannot contribute to \"{action_name}\": {error:?}");
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::core::input::{
		action::{Action, ActionGroup},
		plugin::{InputAppExt, InputPlugin},
		state::ActionState,
		value::{AxisMode, ValueKind},
	};

	#[derive(Clone, Copy)]
	enum TestAction {
		Click,
		Slide,
	}

	impl Action for TestAction {
		fn name(self) -> &'static str {
			match self {
				Self::Click => "test.click",
				Self::Slide => "test.slide",
			}
		}

		fn value(self) -> ValueKind {
			match self {
				Self::Click => ValueKind::Button,
				Self::Slide => ValueKind::Axis1D(AxisMode::Absolute),
			}
		}
	}

	impl ActionGroup for TestAction {
		fn all() -> &'static [Self] {
			&[Self::Click, Self::Slide]
		}
	}

	fn app_with_ui_source() -> App {
		let mut app = App::new();
		app.add_plugins(InputPlugin)
			.add_input_actions::<TestAction>();
		app
	}

	fn click(app: &mut App, action: &'static str) {
		app.world_mut().trigger(UiActivated::button(action));
	}

	fn run_input_frame(app: &mut App) {
		app.world_mut().run_schedule(PreUpdate);
	}

	#[test]
	fn a_ui_activation_presses_the_action_for_one_frame() {
		let mut app = app_with_ui_source();

		click(&mut app, "test.click");
		run_input_frame(&mut app);
		let state = app.world().resource::<ActionState>();
		assert!(state.pressed(TestAction::Click));
		assert!(state.just_pressed(TestAction::Click));

		run_input_frame(&mut app);
		let state = app.world().resource::<ActionState>();
		assert!(!state.pressed(TestAction::Click));
		assert!(!state.just_pressed(TestAction::Click));
	}

	#[test]
	fn an_activation_for_an_unknown_action_is_ignored() {
		let mut app = app_with_ui_source();

		click(&mut app, "test.missing");
		run_input_frame(&mut app);

		assert!(
			!app.world()
				.resource::<ActionState>()
				.pressed(TestAction::Click)
		);
	}

	#[test]
	fn an_activation_with_the_wrong_kind_of_signal_is_ignored() {
		let mut app = app_with_ui_source();

		// A button press cannot drive an axis action.
		click(&mut app, "test.slide");
		run_input_frame(&mut app);

		assert_eq!(
			app.world()
				.resource::<ActionState>()
				.axis1d(TestAction::Slide),
			0.0
		);
	}
}
