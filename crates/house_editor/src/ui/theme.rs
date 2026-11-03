// The theme is a token palette; not every token is used by a widget yet.
#![allow(dead_code)]

use bevy::prelude::*;

pub mod colors {
	use super::*;

	pub const BACKGROUND: Color = Color::srgb(0.045, 0.055, 0.08);
	pub const SURFACE: Color = Color::srgb(0.085, 0.10, 0.14);
	pub const TEXT: Color = Color::srgb(0.95, 0.97, 1.0);
	pub const TEXT_MUTED: Color = Color::srgb(0.65, 0.70, 0.80);
	pub const ACCENT: Color = Color::srgb(0.45, 0.75, 0.95);
	pub const BUTTON: Color = Color::srgb(0.13, 0.17, 0.23);
	pub const BUTTON_HOVERED: Color = Color::srgb(0.20, 0.27, 0.36);
	pub const BUTTON_PRESSED: Color = Color::srgb(0.16, 0.37, 0.48);

	// Editor chrome
	pub const SURFACE_RAISED: Color = Color::srgb(0.115, 0.14, 0.19);
	pub const BORDER: Color = Color::srgb(0.16, 0.195, 0.26);
	pub const SELECTED: Color = Color::srgb(0.13, 0.29, 0.40);
	pub const TEXT_ON_ACCENT: Color = Color::srgb(0.03, 0.07, 0.11);
	pub const TEXT_FAINT: Color = Color::srgb(0.45, 0.50, 0.60);
	pub const TRANSPARENT: Color = Color::NONE;
	pub const OVERLAY: Color = Color::srgba(0.085, 0.10, 0.14, 0.92);
}

pub mod typography {
	pub const TINY: f32 = 11.0;
	pub const SMALL: f32 = 12.5;
	pub const CAPTION: f32 = 14.0;
	pub const BODY: f32 = 16.0;
	pub const BUTTON: f32 = 18.0;
	pub const HEADING: f32 = 42.0;
}

pub mod spacing {
	pub const TINY: f32 = 4.0;
	pub const COMPACT: f32 = 8.0;
	pub const SMALL: f32 = 12.0;
	pub const MEDIUM: f32 = 16.0;
	pub const LARGE: f32 = 24.0;
	pub const EXTRA_LARGE: f32 = 32.0;
}

pub mod controls {
	pub const MIN_HEIGHT: f32 = 52.0;
	pub const HORIZONTAL_PADDING: f32 = 20.0;
	pub const BORDER_WIDTH: f32 = 2.0;
	pub const CORNER_RADIUS: f32 = 10.0;
}

pub const PANEL_CORNER_RADIUS: f32 = 20.0;

pub mod editor_layout {
	pub const TOP_BAR_HEIGHT: f32 = 44.0;
	pub const TOOL_RAIL_WIDTH: f32 = 52.0;
	pub const SIDEBAR_WIDTH: f32 = 340.0;
	pub const SIDEBAR_MIN_WIDTH: f32 = 280.0;
	pub const SIDEBAR_MAX_WIDTH: f32 = 560.0;
	pub const TOOL_BUTTON_SIZE: f32 = 38.0;
	pub const ICON_BUTTON_SIZE: f32 = 32.0;
	pub const ROW_HEIGHT: f32 = 34.0;
	pub const PANEL_RADIUS: f32 = 12.0;
	pub const CONTROL_RADIUS: f32 = 8.0;
}
