use bevy::{input::InputSystems, prelude::*};

mod action;
mod axis_input;
mod context;
mod hints;
mod interpretation;
mod keymap;
mod modifiers;
mod shortcut;
mod signal;
mod sources;
mod state;

pub use action::{Action, ActionGroup};
pub use axis_input::{AxisBinding, AxisInput, Scale};
pub use context::{ActiveContexts, Block, InputContext, InputContextRef, InputContexts};
pub use hints::{Availability, InputHints, ShortcutHint};
pub use interpretation::Interpretation;
pub use keymap::{ButtonBinding, Keymap};
pub use modifiers::Modifier;
pub use shortcut::{Shortcut, ShortcutInput};
pub use signal::{AxisMode, ValueKind};
pub use sources::{UiAction, UiActivated};
pub use state::ActionState;

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use sources::{ButtonSource, poll_axis_bindings, poll_ui_activations};

/// The phases of one input frame, in order. Source systems belong in `Sources`; consumers
/// read the resolved state in `Update` or later, so they never see a half-built frame.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum ActionSystems {
	Begin,
	Sources,
	Resolve,
}

pub trait InputAppExt {
	/// Registers every action of the group, so the keymap can be bound to them by name.
	/// Must run before the keymap is loaded.
	fn add_input_actions<Group: ActionGroup>(&mut self) -> &mut Self;

	/// Registers a kind of context, so the keymap can have a section for it. Must run before the
	/// keymap is loaded. Panics if the name or priority is already taken: that is a mistake in
	/// the startup code, not something to recover from.
	fn add_input_context(&mut self, context: InputContext) -> &mut Self;
}

impl InputAppExt for App {
	fn add_input_actions<Group: ActionGroup>(&mut self) -> &mut Self {
		self.init_resource::<ActionState>();
		self.world_mut()
			.resource_mut::<ActionState>()
			.register_group::<Group>();
		self
	}

	fn add_input_context(&mut self, context: InputContext) -> &mut Self {
		self.init_resource::<InputContexts>();
		self.world_mut()
			.resource_mut::<InputContexts>()
			.register(context)
			.expect("input contexts must have unique names and priorities");
		self
	}
}

pub struct InputPlugin;

impl Plugin for InputPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<ActionState>()
			.init_resource::<InputContexts>()
			.init_resource::<ActiveContexts>();

		// After Bevy has processed the raw devices, so sources read this frame's input.
		app.configure_sets(
			PreUpdate,
			(
				ActionSystems::Begin,
				ActionSystems::Sources,
				ActionSystems::Resolve,
			)
				.chain()
				.after(InputSystems),
		);
		app.add_systems(
			PreUpdate,
			(begin_frame, context::update_active_contexts).in_set(ActionSystems::Begin),
		);

		// The keymap is inserted by the plugin that registers the actions, so the keymap sources
		// skip frames until it exists instead of requiring a plugin order.
		// button input
		app.add_systems(
			PreUpdate,
			poll_buttons
				.in_set(ActionSystems::Sources)
				.run_if(resource_exists::<Keymap>),
		);

		// axis input
		app.add_systems(
			PreUpdate,
			poll_axes
				.in_set(ActionSystems::Sources)
				.run_if(resource_exists::<Keymap>),
		);

		// ui input: needs no keymap, since a widget names its action directly
		app.add_observer(sources::handle_ui_activations);
		app.add_message::<sources::UiActivatedMessage>();
		app.add_systems(
			PreUpdate,
			poll_ui_activations.in_set(ActionSystems::Sources),
		);
		app.add_systems(PreUpdate, resolve_frame.in_set(ActionSystems::Resolve));
	}
}

fn begin_frame(mut state: ResMut<ActionState>) {
	state.start_frame();
}

fn resolve_frame(mut state: ResMut<ActionState>) {
	state.resolve();
}

fn poll_buttons(
	keymap: Res<Keymap>,
	keys: Res<ButtonInput<KeyCode>>,
	mouse_buttons: Res<ButtonInput<MouseButton>>,
	active: Res<ActiveContexts>,
	mut state: ResMut<ActionState>,
	mut source: Local<ButtonSource>,
) {
	source.poll(&keymap, &keys, &mouse_buttons, &active, &mut state);
}

fn poll_axes(
	keymap: Res<Keymap>,
	keys: Res<ButtonInput<KeyCode>>,
	mouse_buttons: Res<ButtonInput<MouseButton>>,
	motion: Res<AccumulatedMouseMotion>,
	scroll: Res<AccumulatedMouseScroll>,
	active: Res<ActiveContexts>,
	mut state: ResMut<ActionState>,
) {
	poll_axis_bindings(
		&keymap,
		&keys,
		&mouse_buttons,
		&motion,
		&scroll,
		&active,
		&mut state,
	);
}

#[cfg(test)]
#[path = "tests/mod_tests.rs"]
mod tests;
