use bevy::{ecs::system::SystemParam, prelude::*};

use crate::core::input::{ActiveContexts, Keymap, Shortcut};

/// Whether pressing a shortcut would do something right now. The hint only reports it: whatever
/// draws the hint decides how each case looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
	Available,
	/// The shortcut's context has no entity right now, such as the viewport while the cursor is
	/// elsewhere.
	ContextInactive,
	/// The context is active, but a higher context binds the same input or blocks it.
	Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortcutHint {
	pub shortcut: Shortcut,
	pub availability: Availability,
}

/// The shortcut hints of an action for display, such as in a tooltip.
#[derive(SystemParam)]
pub struct InputHints<'w> {
	keymap: Res<'w, Keymap>,
	active: Res<'w, ActiveContexts>,
}

impl InputHints<'_> {
	/// The shortcut to show for an action, or `None` if it has none.
	pub fn for_action(&self, name: &'static str) -> Option<ShortcutHint> {
		self.keymap.hints_for(name, &self.active).into_iter().next()
	}
}
