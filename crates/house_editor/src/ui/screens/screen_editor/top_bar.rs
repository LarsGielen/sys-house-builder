use bevy::prelude::*;

use crate::{
	editor::EditorAction,
	ui::{
		icons::{Icon, Icons},
		theme::{colors, editor_layout, spacing},
		widgets::{ButtonOptions, icon_button, separator_vertical},
	},
};

pub(super) fn top_bar(icons: &Icons) -> impl Scene + use<> {
	let size = editor_layout::ICON_BUTTON_SIZE;
	bsn! {
		Node {
			height: px(editor_layout::TOP_BAR_HEIGHT),
			flex_shrink: 0.0,
			padding: UiRect::horizontal(px(spacing::COMPACT)),
			column_gap: px(spacing::COMPACT),
			align_items: AlignItems::Center,
			border: UiRect::bottom(px(1)),
		}
		BackgroundColor(colors::SURFACE)
		BorderColor::all(colors::BORDER)
		Children [
			@icon_button(EditorAction::None, icons.get(Icon::House), size, ButtonOptions { ..default() })
			--
			@separator_vertical()
			--
			@icon_button(EditorAction::Undo, icons.get(Icon::Undo), size, ButtonOptions { ..default() })
			--
			@icon_button(EditorAction::Redo, icons.get(Icon::Redo), size, ButtonOptions { ..default() })
			--
			@separator_vertical()
			--
			@icon_button(EditorAction::Save, icons.get(Icon::Save), size, ButtonOptions { ..default() })
			--
			@icon_button(EditorAction::Open, icons.get(Icon::Open), size, ButtonOptions { ..default() })
		]
	}
}
