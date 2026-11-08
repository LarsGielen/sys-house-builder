use crate::core::input::{
	AxisInput, Scale, Shortcut, context::Claim, interpretation::Interpretation,
	modifiers::ModifierSet,
};

/// A shortcut bound to a button action in one context.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonBinding {
	pub shortcut: Shortcut,
	pub action: &'static str,
	pub context: &'static str,
	pub interpretation: Interpretation,
}

/// An axis input bound to an action by name, with the scale applied to its value.
///
/// It only contributes while the held modifiers equal `modifiers` exactly, so `scroll` stops
/// while Shift is held and `shift+scroll` takes over. Unlike a shortcut it is not latched: it
/// is evaluated every frame, so pressing or releasing a modifier switches binding immediately.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisBinding {
	pub input: AxisInput,
	pub modifiers: ModifierSet,
	pub scale: Scale,
	pub action: &'static str,
	/// The context whose section of the keymap holds this binding.
	pub context: &'static str,
}

impl AxisBinding {
	/// Every input and modifier combination this binding takes, for contexts to decide who
	/// handles it.
	pub(crate) fn claims(&self) -> Vec<Claim> {
		self.input
			.claimed_inputs()
			.into_iter()
			.map(|input| Claim {
				input,
				modifiers: self.modifiers,
			})
			.collect()
	}
}
