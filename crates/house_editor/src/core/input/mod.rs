//! Turns raw device input into named actions.
//!
//! One frame runs in `PreUpdate`, after Bevy has processed the devices (see [`ActionSystems`]):
//!
//! 1. `Begin`: last frame's contributions are cleared and the active contexts are rebuilt from the
//!    entities that exist.
//! 2. `Sources`: each source reads its device and *contributes* signals to actions: keyboard and
//!    mouse through the [`Keymap`], and UI widgets by naming their action directly.
//! 3. `Resolve`: the contributions of each action are combined into one value.
//!
//! Consumers read the resolved [`ActionState`] in `Update` or later.
//!
//! # Vocabulary
//!
//! - **Action**: something that can happen, with a stable name such as `camera.orbit` and a
//!   [`ValueKind`]. An action says *what* happens, never which input causes it.
//! - **Source**: the kind of device an input comes from. Inputs of one source combine (W and S
//!   cancel out); different sources compete for an axis, and the first to leave neutral owns it
//!   until it returns to neutral.
//! - **Binding**: a keymap entry that connects an input to an action within one context.
//! - **Context**: a named layer with a priority that exists while an entity with an
//!   [`InputContextRef`] exists, such as a dialog or the viewport.
//! - **Claim**: an input together with the exact modifiers a binding needs. `ctrl+s` and `s` are
//!   different claims.
//! - **Live**: a binding may act only if its context is active and no higher context shadows or
//!   blocks its claim.
//! - **Shadow**: a higher context binds the same claim, so it handles the input and the lower
//!   binding stays quiet.
//! - **Block** and **pass**: a context can swallow a whole class of input for the contexts below it
//!   even where it binds nothing (a text field swallowing typing), except for the claims it
//!   passes through.

// Much of this public API is only exercised by future features, so unused items and re-exports are
// expected.
#![allow(dead_code, unused_imports)]

mod action;
mod axis_input;
mod binding;
mod button_combo;
mod context;
mod hints;
mod input_names;
mod interpretation;
mod keymap;
mod modifiers;
mod plugin;
mod sources;
mod state;
mod value;

pub use action::{Action, ActionGroup};
pub use button_combo::ButtonCombo;
pub use context::{Block, InputContext, InputContextRef, InputContexts};
pub use hints::{Availability, InputHints, ShortcutHint};
pub use keymap::Keymap;
pub use modifiers::Modifier;
pub use plugin::{ActionSystems, InputAppExt, InputPlugin};
pub use sources::{UiAction, UiActivated};
pub use state::ActionState;
pub use value::{AxisMode, ValueKind};
