pub mod hints;

use bevy::{
	ecs::system::SystemParam,
	input::{
		InputSystems,
		mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit},
	},
	prelude::*,
	window::PrimaryWindow,
};

use crate::{document::project::PendingReplacement, ui::fields::TextFocus};
pub use house_editor_core::Action;
use house_editor_core::ActionState as CoreActionState;

/// Device pixels that one scroll "line" stands for, when a device reports pixels.
pub const SCROLL_PIXELS_PER_LINE: f32 = 40.0;

/// What an [`Action`] sees of the current frame, as a Bevy resource.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct ActionState(CoreActionState);

/// The one place physical devices become editor operations. Everything else asks
/// [`ActionState`] what is happening and never looks at a key or button, so changing
/// [`Bindings`] is enough to rebind an operation.
pub struct ActionsPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActionInputSystems;

impl ActionsPlugin {
	/// Plugins that read actions call this so each works without the others.
	pub fn ensure(app: &mut App) {
		if !app.is_plugin_added::<Self>() {
			app.add_plugins(Self);
		}
	}
}

impl Plugin for ActionsPlugin {
	fn build(&self, app: &mut App) {
		// Without Bevy's input plugin (headless hosts) nothing is ever pressed.
		app.init_resource::<ButtonInput<KeyCode>>()
			.init_resource::<ButtonInput<MouseButton>>()
			.init_resource::<AccumulatedMouseMotion>()
			.init_resource::<AccumulatedMouseScroll>()
			.init_resource::<Bindings>()
			.init_resource::<ActionState>()
			.add_systems(
				PreUpdate,
				resolve_actions
					.in_set(ActionInputSystems)
					.after(InputSystems),
			);
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trigger {
	Key(KeyCode),
	Mouse(MouseButton),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModifierRule {
	Required,
	Absent,
	Ignored,
}

impl ModifierRule {
	fn allows(self, held: bool) -> bool {
		match self {
			Self::Required => held,
			Self::Absent => !held,
			Self::Ignored => true,
		}
	}
}

/// Where a binding applies. Keys typed into a field belong to the field, so the same key can
/// mean different things in different contexts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Context {
	Always,
	Viewport,
	TextField,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Binding {
	pub trigger: Trigger,
	pub control: ModifierRule,
	pub shift: ModifierRule,
	pub context: Context,
	pub action: Action,
}

impl Binding {
	fn new(trigger: Trigger, context: Context, action: Action) -> Self {
		Self {
			trigger,
			control: ModifierRule::Ignored,
			shift: ModifierRule::Ignored,
			context,
			action,
		}
	}

	fn key(key: KeyCode, context: Context, action: Action) -> Self {
		Self::new(Trigger::Key(key), context, action)
	}

	fn with_modifiers(mut self, control: ModifierRule, shift: ModifierRule) -> Self {
		self.control = control;
		self.shift = shift;
		self
	}
}

/// The active binding profile. The default reproduces the editor's standard controls.
#[derive(Resource, Clone, Debug)]
pub struct Bindings(pub Vec<Binding>);

impl Default for Bindings {
	fn default() -> Self {
		use Action::*;
		use Context::*;
		use ModifierRule::*;

		let mut bindings = vec![
			Binding::new(Trigger::Mouse(MouseButton::Left), Always, ToolPress),
			Binding::new(Trigger::Mouse(MouseButton::Right), Always, Orbit),
			Binding::new(Trigger::Mouse(MouseButton::Middle), Always, Pan),
			Binding::key(KeyCode::Escape, Viewport, Cancel),
			Binding::key(KeyCode::Escape, TextField, FieldCancel),
			Binding::key(KeyCode::Delete, Viewport, Delete),
			Binding::key(KeyCode::Home, Always, CameraReset),
		];
		for key in [KeyCode::Enter, KeyCode::NumpadEnter] {
			bindings.push(Binding::key(key, Viewport, Confirm));
			bindings.push(Binding::key(key, TextField, FieldSubmit));
		}
		for key in [KeyCode::ShiftLeft, KeyCode::ShiftRight] {
			bindings.push(Binding::key(key, Always, SuppressSnapping));
		}
		bindings.extend([
			Binding::key(KeyCode::KeyZ, Viewport, Undo).with_modifiers(Required, Absent),
			Binding::key(KeyCode::KeyZ, Viewport, Redo).with_modifiers(Required, Required),
			Binding::key(KeyCode::KeyY, Viewport, Redo).with_modifiers(Required, Ignored),
			Binding::key(KeyCode::KeyS, Viewport, Save).with_modifiers(Required, Ignored),
			Binding::key(KeyCode::KeyO, Viewport, Open).with_modifiers(Required, Ignored),
		]);
		// Panning must not fire for shortcuts such as Ctrl+S.
		for (key, action) in [
			(KeyCode::KeyA, PanLeft),
			(KeyCode::KeyD, PanRight),
			(KeyCode::KeyW, PanForward),
			(KeyCode::KeyS, PanBack),
		] {
			bindings.push(Binding::key(key, Always, action).with_modifiers(Absent, Ignored));
		}
		Self(bindings)
	}
}

#[derive(SystemParam)]
struct Devices<'w> {
	keyboard: Res<'w, ButtonInput<KeyCode>>,
	mouse: Res<'w, ButtonInput<MouseButton>>,
	motion: Res<'w, AccumulatedMouseMotion>,
	scroll: Res<'w, AccumulatedMouseScroll>,
}

impl Trigger {
	fn label(self) -> String {
		match self {
			Self::Mouse(MouseButton::Left) => "Left click".to_string(),
			Self::Mouse(MouseButton::Right) => "Right drag".to_string(),
			Self::Mouse(MouseButton::Middle) => "Middle drag".to_string(),
			Self::Mouse(button) => format!("{button:?}"),
			Self::Key(key) => match key {
				KeyCode::Escape => "Esc".to_string(),
				KeyCode::Enter | KeyCode::NumpadEnter => "Enter".to_string(),
				KeyCode::Delete => "Del".to_string(),
				KeyCode::ShiftLeft | KeyCode::ShiftRight => "Shift".to_string(),
				KeyCode::ControlLeft | KeyCode::ControlRight => "Ctrl".to_string(),
				other => format!("{other:?}")
					.strip_prefix("Key")
					.map_or_else(|| format!("{other:?}"), str::to_string),
			},
		}
	}
}

impl Bindings {
	/// The first binding of `action` as a label such as `Ctrl+S`, or `unbound`.
	pub fn describe(&self, action: Action) -> String {
		let Some(binding) = self.0.iter().find(|binding| binding.action == action) else {
			return "unbound".to_string();
		};
		let mut label = String::new();
		if binding.control == ModifierRule::Required {
			label.push_str("Ctrl+");
		}
		if binding.shift == ModifierRule::Required {
			label.push_str("Shift+");
		}
		label.push_str(&binding.trigger.label());
		label
	}

	/// Replaces each `{ActionName}` in `template` with how that action is triggered, so text
	/// that mentions a control follows the active bindings.
	pub fn fill(&self, template: &str) -> String {
		let mut text = template.to_string();
		for action in Action::ALL {
			let placeholder = format!("{{{action:?}}}");
			if text.contains(&placeholder) {
				text = text.replace(&placeholder, &self.describe(action));
			}
		}
		text
	}
}

fn resolve_actions(
	bindings: Res<Bindings>,
	devices: Devices,
	typing: TextFocus,
	pending: Option<Res<PendingReplacement>>,
	window: Option<Single<&Window, With<PrimaryWindow>>>,
	mut state: ResMut<ActionState>,
) {
	state.clear();
	// A window without focus gets no keys or buttons, and a held input is released with it.
	// An open unsaved-changes question is modal: it must be answered before anything else.
	if window.is_some_and(|window| !window.focused)
		|| pending.is_some_and(|pending| pending.pending_request().is_some())
	{
		return;
	}

	let Devices {
		keyboard,
		mouse,
		motion,
		scroll,
	} = devices;
	let control = keyboard.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
	let shift = keyboard.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
	let typing = typing.active();

	for binding in &bindings.0 {
		let applies = match binding.context {
			Context::Always => true,
			Context::Viewport => !typing,
			Context::TextField => typing,
		};
		if !applies || !binding.control.allows(control) || !binding.shift.allows(shift) {
			continue;
		}
		let (pressed, just_pressed, just_released) = match binding.trigger {
			Trigger::Key(key) => (
				keyboard.pressed(key),
				keyboard.just_pressed(key),
				keyboard.just_released(key),
			),
			Trigger::Mouse(button) => (
				mouse.pressed(button),
				mouse.just_pressed(button),
				mouse.just_released(button),
			),
		};
		if just_pressed {
			state.press(binding.action);
		} else if pressed {
			state.hold(binding.action);
		}
		if just_released {
			state.release(binding.action);
		}
	}

	state.set_pointer_motion(motion.delta);
	state.set_scroll_lines(match scroll.unit {
		MouseScrollUnit::Line => scroll.delta.y,
		MouseScrollUnit::Pixel => scroll.delta.y / SCROLL_PIXELS_PER_LINE,
	});
}

#[cfg(test)]
#[path = "../tests/input.rs"]
mod tests;
