use std::cmp::Reverse;

use bevy::{log::warn_once, prelude::*};

use crate::core::input::{
	Shortcut, ShortcutInput,
	modifiers::{Modifier, Modifiers},
};

/// An input as contexts see it: the thing a binding takes, whatever action it drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClaimedInput {
	Button(ShortcutInput),
	MouseMotion,
	Scroll,
}

/// An input with the exact modifiers a binding needs. A context that binds a claim handles it,
/// so lower contexts never see it; `ctrl+s` and `s` are different claims.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Claim {
	pub input: ClaimedInput,
	pub modifiers: Modifiers,
}

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
	pub fn key(key: KeyCode) -> Self {
		Self::Key(key)
	}

	fn matches(self, claim: &Claim) -> bool {
		match self {
			Self::All => true,
			Self::Modifier(modifier) => {
				claim.modifiers.contains(modifier)
					|| claim.input == ClaimedInput::Button(ShortcutInput::Modifier(modifier))
			}
			Self::Key(key) => claim.input == ClaimedInput::Button(ShortcutInput::Key(key)),
			Self::Typing => match claim.input {
				ClaimedInput::Button(ShortcutInput::Key(key)) => is_typing(key, claim.modifiers),
				// Mouse buttons, motion and scroll are not typed.
				_ => false,
			},
		}
	}
}

fn is_typing(key: KeyCode, modifiers: Modifiers) -> bool {
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
		_ => !modifiers.ctrl && !modifiers.alt && is_text_key(ShortcutInput::Key(key)),
	}
}

fn is_text_key(input: ShortcutInput) -> bool {
	input
		.name()
		.is_some_and(|name| name.len() == 1 || name == "space")
}

/// What a kind of context does, registered once at startup so the keymap can name it. Priority
/// orders the stack: a higher context sees an input first.
#[derive(Debug, Clone)]
pub struct InputContext {
	name: &'static str,
	priority: i32,
	block: Vec<Block>,
	pass: Vec<Shortcut>,
}

impl InputContext {
	pub fn new(name: &'static str) -> Self {
		Self {
			name,
			priority: 0,
			block: Vec::new(),
			pass: Vec::new(),
		}
	}

	pub fn priority(mut self, priority: i32) -> Self {
		self.priority = priority;
		self
	}

	/// Swallows what the rule matches for the contexts below, unless it is passed.
	pub fn block(mut self, rule: Block) -> Self {
		self.block.push(rule);
		self
	}

	/// Lets exactly this shortcut through to the contexts below, even if a block rule matches it.
	pub fn pass(mut self, shortcut: Shortcut) -> Self {
		self.pass.push(shortcut);
		self
	}

	pub fn name(&self) -> &'static str {
		self.name
	}

	fn passes(&self, claim: &Claim) -> bool {
		self.pass.iter().any(|shortcut| shortcut.claim() == *claim)
	}

	fn blocks(&self, claim: &Claim) -> bool {
		self.block.iter().any(|rule| rule.matches(claim))
	}
}

/// Every context the app knows, ordered from the highest priority. The keymap is checked against
/// it, so it must be filled before the keymap loads.
#[derive(Resource, Default)]
pub struct InputContexts {
	contexts: Vec<InputContext>,
}

impl InputContexts {
	/// Names and priorities must be unique, so the order of the stack is never a guess.
	pub fn register(&mut self, context: InputContext) -> Result<(), String> {
		if let Some(other) = self
			.contexts
			.iter()
			.find(|other| other.name == context.name || other.priority == context.priority)
		{
			return Err(if other.name == context.name {
				format!("input context \"{}\" is already registered", context.name)
			} else {
				format!(
					"input contexts \"{}\" and \"{}\" have the same priority {}",
					other.name, context.name, context.priority
				)
			});
		}

		self.contexts.push(context);
		self.contexts
			.sort_by_key(|context| Reverse(context.priority));
		Ok(())
	}

	pub fn get(&self, name: &str) -> Option<&InputContext> {
		self.contexts.iter().find(|context| context.name == name)
	}

	/// From the highest priority to the lowest.
	pub(crate) fn by_priority(&self) -> &[InputContext] {
		&self.contexts
	}
}

/// Puts an entity's context on the stack for as long as the component exists. The UI that owns
/// the context spawns it: a dialog with its own, a field while it has focus.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputContextRef(pub &'static str);

/// The contexts that have an entity right now, from the highest priority to the lowest.
#[derive(Resource, Default)]
pub struct ActiveContexts {
	contexts: Vec<InputContext>,
}

impl ActiveContexts {
	pub(crate) fn new(mut contexts: Vec<InputContext>) -> Self {
		contexts.sort_by_key(|context| Reverse(context.priority));
		Self { contexts }
	}

	pub fn is_active(&self, name: &str) -> bool {
		self.contexts.iter().any(|context| context.name == name)
	}

	/// Whether a binding of `owner` for `claim` may act right now. `binds(context, claim)` tells
	/// whether a context has a binding for the claim.
	///
	/// The contexts above the owner are asked from the top. One that binds the claim itself
	/// handles it, so the binding is shadowed. Otherwise, if the context passes the claim it
	/// falls through, and if the context blocks it, it stops here. Everything else falls through.
	pub(crate) fn is_live(
		&self,
		owner: &str,
		claim: &Claim,
		binds: impl Fn(&str, &Claim) -> bool,
	) -> bool {
		let Some(position) = self
			.contexts
			.iter()
			.position(|context| context.name == owner)
		else {
			return false;
		};

		for context in &self.contexts[..position] {
			if binds(context.name, claim) {
				return false;
			}
			if context.passes(claim) {
				continue;
			}
			if context.blocks(claim) {
				return false;
			}
		}
		true
	}
}

/// Rebuilds the stack every frame from the entities that exist, like the action state, so a
/// despawned dialog can never leave its context behind.
pub(super) fn update_active_contexts(
	references: Query<&InputContextRef>,
	contexts: Res<InputContexts>,
	mut active: ResMut<ActiveContexts>,
) {
	let mut names: Vec<&'static str> = references.iter().map(|reference| reference.0).collect();
	names.sort_unstable();
	names.dedup();

	let found = names
		.into_iter()
		.filter_map(|name| match contexts.get(name) {
			Some(context) => Some(context.clone()),
			None => {
				warn_once!("input context \"{name}\" is not registered");
				None
			}
		})
		.collect();
	*active = ActiveContexts::new(found);
}

#[cfg(test)]
#[path = "tests/context_tests.rs"]
mod tests;
