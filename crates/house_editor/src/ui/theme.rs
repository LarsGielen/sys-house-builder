//! The editor's colours: a dark navy chrome with an amber accent for whatever is active.

use bevy::prelude::*;

/// Top bar and bottom dock.
pub const CHROME: Color = Color::srgb(0.059, 0.098, 0.184);
/// Floating panels over the viewport.
pub const PANEL: Color = Color::srgba(0.063, 0.106, 0.2, 0.94);
pub const PANEL_BORDER: Color = Color::srgb(0.17, 0.25, 0.4);
pub const BUTTON: Color = Color::srgb(0.098, 0.157, 0.29);
pub const BUTTON_DISABLED: Color = Color::srgb(0.07, 0.11, 0.2);
/// A chosen option inside a panel; text keeps its normal colour on it.
pub const SELECTED: Color = Color::srgb(0.2, 0.38, 0.72);
pub const FIELD: Color = Color::srgb(0.047, 0.078, 0.15);
pub const ACCENT: Color = Color::srgb(0.96, 0.67, 0.13);
pub const TEXT: Color = Color::srgb(0.9, 0.93, 0.98);
pub const TEXT_ON_ACCENT: Color = Color::srgb(0.1, 0.08, 0.03);
/// The viewport behind the house, like blueprint paper.
pub const VIEWPORT: Color = Color::srgb(0.2, 0.26, 0.47);

pub const RADIUS: Val = Val::Px(8.0);
