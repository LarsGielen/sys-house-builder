use bevy::prelude::KeyCode;

use crate::core::input::{
	button_combo::PhysicalButton,
	context::{Claim, ClaimedInput},
	modifiers::{Modifier, ModifierSet},
};

/// What a context swallows for the contexts below it, even where it binds nothing. Written in
/// code, never in the keymap file, so an edited file cannot make a modal context leak input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Block {
	/// Every input.
	All,
	/// Any input that includes this modifier, and the modifier key itself.
	Modifier(Modifier),
	/// This key with any modifiers.
	Key(KeyCode),
	/// What a text field consumes: letters, digits and space with no modifier or only Shift, the
	/// editing keys (backspace, delete, enter, arrows) and Ctrl+A/C/V/X/Z/Y. Ctrl+Z and Ctrl+Y
	/// are swallowed even with nothing to undo, so they always act on the text and never the scene.
	Typing,
}

impl Block {
	/// Shorthand for `Block::Key(key)`.
	pub fn key(key: KeyCode) -> Self {
		Self::Key(key)
	}

	pub(super) fn matches(self, claim: &Claim) -> bool {
		match self {
			Self::All => true,
			Self::Modifier(modifier) => {
				claim.modifiers.contains(modifier)
					|| claim.input == ClaimedInput::Button(PhysicalButton::Modifier(modifier))
			}
			Self::Key(key) => claim.input == ClaimedInput::Button(PhysicalButton::Key(key)),
			Self::Typing => match claim.input {
				ClaimedInput::Button(PhysicalButton::Key(key)) => is_typing(key, claim.modifiers),
				// Mouse buttons, motion and scroll are not typed.
				_ => false,
			},
		}
	}
}

fn is_typing(key: KeyCode, modifiers: ModifierSet) -> bool {
	match key {
		KeyCode::Backspace
		| KeyCode::Delete
		| KeyCode::Enter
		| KeyCode::ArrowUp
		| KeyCode::ArrowDown
		| KeyCode::ArrowLeft
		| KeyCode::ArrowRight => true,
		KeyCode::KeyA
		| KeyCode::KeyC
		| KeyCode::KeyV
		| KeyCode::KeyX
		| KeyCode::KeyZ
		| KeyCode::KeyY
			if modifiers.ctrl && !modifiers.alt =>
		{
			true
		}
		_ => !modifiers.ctrl && !modifiers.alt && is_text_key(PhysicalButton::Key(key)),
	}
}

fn is_text_key(input: PhysicalButton) -> bool {
	input
		.name()
		.is_some_and(|name| name.len() == 1 || name == "space")
}
