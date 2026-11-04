use bevy::{input::InputSystems, prelude::*};

mod action;
mod shortcut;
mod signal;
mod state;

pub use action::{Action, ActionGroup};
pub use shortcut::Shortcut;
pub(crate) use signal::ValueKind;
pub use state::ActionState;

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
}

impl InputAppExt for App {
	fn add_input_actions<Group: ActionGroup>(&mut self) -> &mut Self {
		self.init_resource::<ActionState>();
		self.world_mut()
			.resource_mut::<ActionState>()
			.register_group::<Group>();
		self
	}
}

pub struct InputPlugin;

impl Plugin for InputPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<ActionState>();
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
		app.add_systems(PreUpdate, begin_frame.in_set(ActionSystems::Begin));
		app.add_systems(PreUpdate, resolve_frame.in_set(ActionSystems::Resolve));
	}
}

fn begin_frame(mut state: ResMut<ActionState>) {
	state.start_frame();
}

fn resolve_frame(mut state: ResMut<ActionState>) {
	state.resolve();
}
