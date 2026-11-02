use bevy::prelude::*;

use crate::ui::theme::{colors, editor_layout, spacing};

pub(super) fn top_bar() -> impl Scene + use<> {
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
	}
}
