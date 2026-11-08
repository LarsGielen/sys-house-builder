use bevy::prelude::*;

use crate::{
	core::input::UiAction,
	ui::{
		theme::{colors, spacing, typography},
		widgets::{ButtonOptions, buttons::button},
	},
};

/// A labelled button for a toggle action. It looks selected while the toggle is on, which every
/// button of a toggle does, and a click flips the value like its shortcut does.
pub fn toggle(
	action: impl Into<UiAction>,
	label: &'static str,
	options: ButtonOptions,
) -> impl Scene {
	bsn! {
		@button(action, options)
		Node { padding: UiRect::horizontal(px(spacing::COMPACT)) }
		Children [
			Text::new(label)
			TextFont { font_size: FontSize::Px(typography::SMALL) }
			TextColor(colors::TEXT)
			Pickable::IGNORE
		]
	}
}
