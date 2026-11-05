use bevy::prelude::*;

use crate::editor_core::input::{Action, ActionState, signal::Signal, state::SourceId};

#[derive(Event)]
pub struct UiActivated {
	action: &'static str,
	signal: Signal,
}

// Buffers an activation from `Update` until the sources run in the next `PreUpdate`.
#[derive(Message)]
pub(in crate::editor_core::input) struct UiActivatedMessage {
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

pub(in crate::editor_core::input) fn handle_ui_activations(
	activation: On<UiActivated>,
	mut activations: MessageWriter<UiActivatedMessage>,
) {
	activations.write(UiActivatedMessage {
		action: activation.action,
		signal: activation.signal,
	});
}

pub(in crate::editor_core::input) fn poll_ui_activations(
	mut message_reader: MessageReader<UiActivatedMessage>,
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
#[path = "../tests/sources_ui_tests.rs"]
mod tests;
