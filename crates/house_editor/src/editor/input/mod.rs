use bevy::{input::InputSystems, prelude::*};

mod action;
mod shortcut;
mod signal;
mod state;

pub use shortcut::Shortcut;
pub use state::ActionState;

/// The phases of one input frame, in order. Source systems belong in `Sources`; consumers
/// read the resolved state in `Update` or later, so they never see a half-built frame.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum ActionSystems {
	Begin,
	Sources,
	Resolve,
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
