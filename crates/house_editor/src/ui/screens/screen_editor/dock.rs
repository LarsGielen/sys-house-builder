use bevy::prelude::*;

use crate::ui::theme::{colors, editor_layout, spacing};

pub(super) fn dock() -> impl Scene + use<> {
	bsn! {
		Node {
			flex_shrink: 0.0,
			border: UiRect::all(px(1)),
			border_radius: BorderRadius::all(px(editor_layout::PANEL_RADIUS)),
			overflow: Overflow::clip(),
		}
		BackgroundColor(colors::SURFACE)
		BorderColor::all(colors::BORDER)
		Children []
	}
}
