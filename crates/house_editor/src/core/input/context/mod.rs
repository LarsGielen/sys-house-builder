use std::cmp::Reverse;

use bevy::{log::warn_once, prelude::*};

use crate::core::input::Shortcut;

mod block;
mod claim;

pub use block::Block;
pub(crate) use claim::{Claim, ClaimedInput};

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
mod tests {
	use bevy::ecs::system::RunSystemOnce;

	use super::*;
	use crate::core::input::{
		PhysicalButton,
		modifiers::{Modifier, ModifierSet},
	};

	fn plain(key: KeyCode) -> Claim {
		with(key, ModifierSet::default())
	}

	fn ctrl(key: KeyCode) -> Claim {
		with(
			key,
			ModifierSet {
				ctrl: true,
				..ModifierSet::default()
			},
		)
	}

	fn shift(key: KeyCode) -> Claim {
		with(
			key,
			ModifierSet {
				shift: true,
				..ModifierSet::default()
			},
		)
	}

	fn with(key: KeyCode, modifiers: ModifierSet) -> Claim {
		Claim {
			input: ClaimedInput::Button(PhysicalButton::Key(key)),
			modifiers,
		}
	}

	fn mouse(button: MouseButton) -> Claim {
		Claim {
			input: ClaimedInput::Button(PhysicalButton::Mouse(button)),
			modifiers: ModifierSet::default(),
		}
	}

	fn no_bindings(_context: &str, _claim: &Claim) -> bool {
		false
	}

	fn active(contexts: Vec<InputContext>) -> ActiveContexts {
		ActiveContexts::new(contexts)
	}

	fn global() -> InputContext {
		InputContext::new("global").priority(0)
	}

	fn viewport() -> InputContext {
		InputContext::new("viewport").priority(10)
	}

	fn text_field() -> InputContext {
		InputContext::new("text").priority(20).block(Block::Typing)
	}

	// ## Registry ## //

	#[test]
	fn contexts_are_listed_from_the_highest_priority() {
		let mut contexts = InputContexts::default();
		contexts.register(global()).unwrap();
		contexts.register(text_field()).unwrap();
		contexts.register(viewport()).unwrap();

		let names: Vec<_> = contexts.by_priority().iter().map(|c| c.name()).collect();
		assert_eq!(names, ["text", "viewport", "global"]);
	}

	#[test]
	fn registering_a_name_or_a_priority_twice_is_an_error() {
		let mut contexts = InputContexts::default();
		contexts.register(global()).unwrap();

		let same_name = contexts.register(InputContext::new("global").priority(5));
		assert!(same_name.unwrap_err().contains("already registered"));

		let same_priority = contexts.register(InputContext::new("other").priority(0));
		assert!(same_priority.unwrap_err().contains("same priority"));
	}

	// ## Liveness ## //

	#[test]
	fn a_binding_of_an_inactive_context_is_not_live() {
		let stack = active(vec![global()]);

		assert!(!stack.is_live("viewport", &plain(KeyCode::KeyW), no_bindings));
	}

	#[test]
	fn the_top_context_and_an_unshadowed_lower_one_are_live() {
		let stack = active(vec![global(), viewport()]);

		assert!(stack.is_live("viewport", &plain(KeyCode::KeyW), no_bindings));
		assert!(stack.is_live("global", &plain(KeyCode::KeyW), no_bindings));
	}

	#[test]
	fn a_higher_context_that_binds_the_same_input_shadows_the_lower_one() {
		let stack = active(vec![global(), viewport()]);
		let viewport_binds_w =
			|context: &str, claim: &Claim| context == "viewport" && *claim == plain(KeyCode::KeyW);

		assert!(!stack.is_live("global", &plain(KeyCode::KeyW), viewport_binds_w));
		// Only what it binds: another key, or the same key with other modifiers, is left alone.
		assert!(stack.is_live("global", &plain(KeyCode::KeyS), viewport_binds_w));
		assert!(stack.is_live("global", &ctrl(KeyCode::KeyW), viewport_binds_w));
		// The shadowing context itself stays live.
		assert!(stack.is_live("viewport", &plain(KeyCode::KeyW), viewport_binds_w));
	}

	#[test]
	fn a_shadowing_context_that_is_not_active_shadows_nothing() {
		let stack = active(vec![global()]);
		let viewport_binds_w = |context: &str, _: &Claim| context == "viewport";

		assert!(stack.is_live("global", &plain(KeyCode::KeyW), viewport_binds_w));
	}

	// ## Blocking ## //

	#[test]
	fn block_all_swallows_everything_for_the_contexts_below() {
		let dialog = InputContext::new("dialog").priority(30).block(Block::All);
		let stack = active(vec![global(), dialog]);

		assert!(!stack.is_live("global", &ctrl(KeyCode::KeyS), no_bindings));
		assert!(!stack.is_live("global", &mouse(MouseButton::Right), no_bindings));
		let motion = Claim {
			input: ClaimedInput::MouseMotion,
			modifiers: ModifierSet::default(),
		};
		assert!(!stack.is_live("global", &motion, no_bindings));
		// The blocking context is not blocked by itself.
		assert!(stack.is_live("dialog", &ctrl(KeyCode::KeyS), no_bindings));
	}

	#[test]
	fn a_block_rule_only_affects_the_contexts_below_it() {
		let modal = InputContext::new("modal")
			.priority(20)
			.block(Block::Modifier(Modifier::Ctrl));
		let tool = InputContext::new("tool").priority(30);
		let stack = active(vec![global(), modal, tool]);

		assert!(!stack.is_live("global", &ctrl(KeyCode::KeyS), no_bindings));
		assert!(stack.is_live("global", &plain(KeyCode::KeyS), no_bindings));
		assert!(stack.is_live("modal", &ctrl(KeyCode::KeyS), no_bindings));
		assert!(stack.is_live("tool", &ctrl(KeyCode::KeyS), no_bindings));
	}

	#[test]
	fn a_key_rule_blocks_the_key_with_any_modifiers() {
		let context = InputContext::new("modal")
			.priority(20)
			.block(Block::key(KeyCode::KeyG));
		let stack = active(vec![global(), context]);

		assert!(!stack.is_live("global", &plain(KeyCode::KeyG), no_bindings));
		assert!(!stack.is_live("global", &ctrl(KeyCode::KeyG), no_bindings));
		assert!(stack.is_live("global", &plain(KeyCode::KeyH), no_bindings));
	}

	#[test]
	fn typing_swallows_what_a_text_field_consumes() {
		let stack = active(vec![global(), text_field()]);
		let blocked = |claim: Claim| !stack.is_live("global", &claim, no_bindings);

		assert!(blocked(plain(KeyCode::KeyG)));
		assert!(blocked(shift(KeyCode::KeyG)));
		assert!(blocked(plain(KeyCode::Digit1)));
		assert!(blocked(plain(KeyCode::Space)));
		assert!(blocked(plain(KeyCode::Backspace)));
		assert!(blocked(plain(KeyCode::ArrowLeft)));
		assert!(blocked(ctrl(KeyCode::Backspace)));
		for key in [
			KeyCode::KeyA,
			KeyCode::KeyC,
			KeyCode::KeyV,
			KeyCode::KeyX,
			KeyCode::KeyZ,
			KeyCode::KeyY,
		] {
			assert!(blocked(ctrl(key)), "ctrl+{key:?} must edit the text");
		}
	}

	#[test]
	fn typing_lets_through_shortcuts_the_field_does_not_consume() {
		let stack = active(vec![global(), text_field()]);
		let allowed = |claim: Claim| stack.is_live("global", &claim, no_bindings);

		assert!(allowed(ctrl(KeyCode::KeyS)));
		assert!(allowed(ctrl(KeyCode::KeyO)));
		assert!(allowed(plain(KeyCode::F5)));
		assert!(allowed(plain(KeyCode::Escape)));
		assert!(allowed(plain(KeyCode::Tab)));
		assert!(allowed(mouse(MouseButton::Right)));
	}

	#[test]
	fn a_passed_shortcut_falls_through_a_block_rule() {
		let context = InputContext::new("modal")
			.priority(20)
			.block(Block::Modifier(Modifier::Ctrl))
			.pass(Shortcut::ctrl(KeyCode::KeyS));
		let stack = active(vec![global(), context]);

		assert!(stack.is_live("global", &ctrl(KeyCode::KeyS), no_bindings));
		assert!(!stack.is_live("global", &ctrl(KeyCode::KeyO), no_bindings));
	}

	#[test]
	fn a_binding_of_a_higher_context_wins_over_a_pass_rule() {
		let context = InputContext::new("modal")
			.priority(20)
			.pass(Shortcut::ctrl(KeyCode::KeyS));
		let stack = active(vec![global(), context]);
		let modal_binds_save =
			|context: &str, claim: &Claim| context == "modal" && *claim == ctrl(KeyCode::KeyS);

		assert!(!stack.is_live("global", &ctrl(KeyCode::KeyS), modal_binds_save));
	}

	#[test]
	fn blocking_stops_at_the_first_context_that_matches() {
		// The text field sits between the viewport and the global context.
		let stack = active(vec![
			global(),
			text_field(),
			InputContext::new("tool").priority(30),
		]);

		assert!(!stack.is_live("global", &plain(KeyCode::KeyG), no_bindings));
		assert!(stack.is_live("text", &plain(KeyCode::KeyG), no_bindings));
	}

	// ## Active contexts ## //

	#[test]
	fn the_active_stack_follows_the_entities_that_exist() {
		let mut world = World::new();
		let mut contexts = InputContexts::default();
		contexts.register(global()).unwrap();
		contexts.register(viewport()).unwrap();
		world.insert_resource(contexts);
		world.init_resource::<ActiveContexts>();
		world.spawn(InputContextRef("global"));
		let viewport_entity = world.spawn(InputContextRef("viewport")).id();

		// Two entities of one context are one context, and an unknown name is skipped.
		world.spawn(InputContextRef("viewport"));
		world.spawn(InputContextRef("missing"));
		world.run_system_once(update_active_contexts).unwrap();
		let stack = world.resource::<ActiveContexts>();
		assert!(stack.is_active("global") && stack.is_active("viewport"));
		assert!(!stack.is_active("missing"));

		// One of the two viewport entities going away leaves the context, the last one removes it.
		world.despawn(viewport_entity);
		world.run_system_once(update_active_contexts).unwrap();
		assert!(world.resource::<ActiveContexts>().is_active("viewport"));
	}

	#[test]
	fn a_context_ends_with_its_last_entity() {
		let mut world = World::new();
		let mut contexts = InputContexts::default();
		contexts.register(global()).unwrap();
		world.insert_resource(contexts);
		world.init_resource::<ActiveContexts>();
		let entity = world.spawn(InputContextRef("global")).id();

		world.run_system_once(update_active_contexts).unwrap();
		assert!(world.resource::<ActiveContexts>().is_active("global"));

		world.despawn(entity);
		world.run_system_once(update_active_contexts).unwrap();
		assert!(!world.resource::<ActiveContexts>().is_active("global"));
	}

	#[test]
	fn a_modifier_rule_also_blocks_the_modifier_key_itself() {
		let context = InputContext::new("modal")
			.priority(20)
			.block(Block::Modifier(Modifier::Shift));
		let stack = active(vec![global(), context]);
		let shift_key = Claim {
			input: ClaimedInput::Button(PhysicalButton::Modifier(Modifier::Shift)),
			modifiers: ModifierSet::default(),
		};
		let ctrl_key = Claim {
			input: ClaimedInput::Button(PhysicalButton::Modifier(Modifier::Ctrl)),
			modifiers: ModifierSet::default(),
		};

		assert!(!stack.is_live("global", &shift_key, no_bindings));
		assert!(stack.is_live("global", &ctrl_key, no_bindings));
	}
}
