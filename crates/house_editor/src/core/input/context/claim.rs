use crate::core::input::{button_combo::PhysicalButton, modifiers::ModifierSet};

/// An input as contexts see it: the thing a binding takes, whatever action it drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClaimedInput {
	Button(PhysicalButton),
	MouseMotion,
	Scroll,
}

/// An input with the exact modifiers a binding needs. A context that binds a claim handles it,
/// so lower contexts never see it; `ctrl+s` and `s` are different claims.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Claim {
	pub input: ClaimedInput,
	pub modifiers: ModifierSet,
}
