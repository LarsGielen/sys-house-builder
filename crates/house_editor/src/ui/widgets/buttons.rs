use std::default;

use bevy::{
	input_focus::tab_navigation::TabIndex,
	picking::{cursor::EntityCursor, hover::Hovered},
	prelude::*,
	ui_widgets::{Activate, Button},
	window::SystemCursorIcon,
};

use crate::ui::theme::{colors, editor_layout};

#[derive(Component, Default, Clone, Copy, PartialEq, Eq)]
pub enum ButtonStyle {
	#[default]
	Chip,
	Primary,
	Outline,
}

pub fn button(style: ButtonStyle, hint: &'static str) -> impl Scene {
	bsn! {
		Button
		Hovered::default()
		TabIndex(0)
		EntityCursor::System(SystemCursorIcon::Pointer)
		style
		on(run_action_on_activate)
		Node {
			border: px(2),
			border_radius: BorderRadius::all(px(editor_layout::CONTROL_RADIUS)),
			align_items: AlignItems::Center,
			justify_content: JustifyContent::Center,
		}
		BackgroundColor(colors::TRANSPARENT)
		BorderColor::all(Color::NONE)
	}
}

pub fn icon_button(icon: Handle<Image>, size: f32, style: ButtonStyle, hint: &'static str) {}

pub fn run_action_on_activate(activate: On<Activate>) {}
