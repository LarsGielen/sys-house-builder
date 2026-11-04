use bevy::{input::InputSystems, prelude::*};

mod action;
mod keymap;
mod shortcut;
mod signal;
mod sources;
mod state;

pub use action::{Action, ActionGroup};
pub use keymap::Keymap;
pub use shortcut::Shortcut;
pub(crate) use signal::ValueKind;
pub use state::ActionState;

use sources::ButtonSource;

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
		// The keymap is inserted by the plugin that registers the actions, so this system skips
		// frames until it exists instead of requiring a plugin order.
		app.add_systems(
			PreUpdate,
			poll_buttons
				.in_set(ActionSystems::Sources)
				.run_if(resource_exists::<Keymap>),
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
	mut state: ResMut<ActionState>,
	mut source: Local<ButtonSource>,
) {
	source.poll(&keymap, &keys, &mouse_buttons, &mut state);
}

#[cfg(test)]
mod tests {
	use super::*;

	#[derive(Clone, Copy)]
	enum TestAction {
		Jump,
	}

	impl Action for TestAction {
		fn name(self) -> &'static str {
			"test.jump"
		}

		fn value(self) -> ValueKind {
			ValueKind::Button
		}
	}

	impl ActionGroup for TestAction {
		fn all() -> &'static [Self] {
			&[Self::Jump]
		}
	}

	fn app_with_devices() -> App {
		let mut app = App::new();
		app.init_resource::<ButtonInput<KeyCode>>()
			.init_resource::<ButtonInput<MouseButton>>()
			.add_plugins(InputPlugin)
			.add_input_actions::<TestAction>();
		app
	}

	fn run_input_frame(app: &mut App) {
		app.world_mut().run_schedule(PreUpdate);
	}

	#[test]
	fn a_pressed_key_reaches_the_action_state_through_the_schedule() {
		let mut app = app_with_devices();
		let keymap = Keymap::parse(
			"[bindings]\n\"test.jump\" = [\"space\"]",
			app.world().resource::<ActionState>(),
		)
		.unwrap();
		app.insert_resource(keymap);

		run_input_frame(&mut app);
		assert!(
			!app.world()
				.resource::<ActionState>()
				.pressed(TestAction::Jump)
		);

		app.world_mut()
			.resource_mut::<ButtonInput<KeyCode>>()
			.press(KeyCode::Space);
		run_input_frame(&mut app);
		let state = app.world().resource::<ActionState>();
		assert!(state.pressed(TestAction::Jump));
		assert!(state.just_pressed(TestAction::Jump));

		app.world_mut()
			.resource_mut::<ButtonInput<KeyCode>>()
			.clear();
		run_input_frame(&mut app);
		let state = app.world().resource::<ActionState>();
		assert!(state.pressed(TestAction::Jump));
		assert!(!state.just_pressed(TestAction::Jump));
	}

	#[test]
	fn frames_without_a_keymap_are_skipped_instead_of_panicking() {
		let mut app = app_with_devices();
		app.world_mut()
			.resource_mut::<ButtonInput<KeyCode>>()
			.press(KeyCode::Space);

		run_input_frame(&mut app);

		assert!(
			!app.world()
				.resource::<ActionState>()
				.pressed(TestAction::Jump)
		);
	}
}
