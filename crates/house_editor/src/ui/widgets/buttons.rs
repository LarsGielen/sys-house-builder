use bevy::{
	ecs::query::QueryData,
	input_focus::{InputFocus, InputFocusVisible, tab_navigation::TabIndex},
	picking::{cursor::EntityCursor, hover::Hovered},
	prelude::*,
	ui::Pressed,
	ui_widgets::{Activate, Button},
	window::SystemCursorIcon,
};

use crate::{
	core::input::{UiAction, UiActivated},
	ui::{
		theme::{colors, editor_layout},
		widgets::tooltip::TooltipText,
	},
};

pub(super) struct ButtonPlugin;
impl Plugin for ButtonPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, update_buttons);
	}
}

#[derive(Component, Default, Clone, Copy, PartialEq, Eq)]
pub enum ButtonStyle {
	#[default]
	Chip,
	// Primary,
	// Outline,
}

pub fn icon_image(icon: Handle<Image>, size: f32, color: Color) -> impl Scene {
	bsn! {
		Node { width: px(size), height: px(size), flex_shrink: 0.0 }
		ImageNode { image: icon, color }
	}
}

#[derive(Default)]
pub struct ButtonOptions {
	pub style: Option<ButtonStyle>,
	pub hint: Option<&'static str>,
	pub color: Option<Color>,
}

pub fn button(action: impl Into<UiAction>, options: ButtonOptions) -> impl Scene {
	let action = action.into();
	let style = options.style.unwrap_or_default();
	let hint = options.hint;
	bsn! {
		action
		Button
		Hovered::default()
		TabIndex(0)
		EntityCursor::System(SystemCursorIcon::Pointer)
		style
		TooltipText(hint)
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

pub fn icon_button(
	action: impl Into<UiAction>,
	icon: Handle<Image>,
	size: f32,
	options: ButtonOptions,
) -> impl Scene {
	let icon_color = options.color.unwrap_or(colors::TEXT);
	bsn! {
		@button(action, options)
		Node { width: px(size), height: px(size), flex_shrink: 0.0 }
		Children [
			@icon_image(icon, size * 0.55, icon_color)
		]
	}
}

pub fn run_action_on_activate(
	activate: On<Activate>,
	actions: Query<&UiAction>,
	mut commands: Commands,
) {
	let Ok(UiAction(Some(name))) = actions.get(activate.entity) else {
		return;
	};

	commands.trigger(UiActivated::button(name));
}

#[derive(QueryData)]
#[query_data(mutable)]
struct ButtonQuery {
	entity: Entity,
	style: &'static ButtonStyle,
	hovered: &'static Hovered,
	pressed: Has<Pressed>,
	background: &'static mut BackgroundColor,
	border: &'static mut BorderColor,
}

fn update_buttons(
	focus: Res<InputFocus>,
	focus_visible: Res<InputFocusVisible>,
	mut buttons: Query<ButtonQuery>,
) {
	for mut button in &mut buttons {
		let selected = false;
		let (fill, ring) = resolve_button_style(
			*button.style,
			selected,
			button.hovered.get(),
			button.pressed,
		);
		if let Some(fill) = fill {
			button.background.set_if_neq(BackgroundColor(fill));
		}
		let focused = focus_visible.0 && focus.get() == Some(button.entity);
		button
			.border
			.set_if_neq(BorderColor::all(if focused { colors::TEXT } else { ring }));
	}
}

fn resolve_button_style(
	style: ButtonStyle,
	selected: bool,
	hovered: bool,
	pressed: bool,
) -> (Option<Color>, Color) {
	match style {
		ButtonStyle::Chip => {
			let idle = if style == ButtonStyle::Chip {
				colors::BUTTON
			} else {
				colors::TRANSPARENT
			};
			let background = match (selected, pressed, hovered) {
				(_, true, _) => colors::BUTTON_PRESSED,
				(true, _, true) => colors::SELECTED.mix(&colors::ACCENT, 0.15),
				(true, _, false) => colors::SELECTED,
				(false, _, true) => colors::BUTTON_HOVERED,
				_ => idle,
			};
			let border = if selected {
				colors::ACCENT
			} else {
				Color::NONE
			};
			(Some(background), border)
		} // ButtonStyle::Primary => {
		  // 	let background = match (pressed, hovered) {
		  // 		(true, _) => colors::ACCENT.mix(&Color::BLACK, 0.25),
		  // 		(_, true) => colors::ACCENT.mix(&Color::WHITE, 0.2),
		  // 		_ => colors::ACCENT,
		  // 	};
		  // 	(Some(background), Color::NONE)
		  // }
		  // ButtonStyle::Outline => {
		  // 	let border = match (selected, hovered) {
		  // 		(true, _) => colors::ACCENT,
		  // 		(_, true) => colors::TEXT_FAINT,
		  // 		_ => colors::BORDER,
		  // 	};
		  // 	(None, border)
		  // }
	}
}
