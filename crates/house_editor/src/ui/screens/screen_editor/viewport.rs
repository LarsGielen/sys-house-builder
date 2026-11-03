use bevy::{picking::hover::Hovered, prelude::*, ui::RelativeCursorPosition};

use crate::ui::theme::{colors, editor_layout};

pub fn viewport_node(camera: Entity) -> impl Scene {
	bsn! {
		~{ViewportNode::new(camera)}
		Node { overflow: Overflow::clip() }
	}
}

pub(super) fn viewport(camera: Entity) -> impl Scene + use<> {
	bsn! {
		Node {
			flex_grow: 1.0,
			flex_basis: px(0),
			min_width: px(0),
			border: UiRect::all(px(1)),
			border_radius: BorderRadius::all(px(editor_layout::PANEL_RADIUS)),
			overflow: Overflow::clip(),
		}
		BorderColor::all(colors::BORDER)
		Children [
			@viewport_node(camera)
			Hovered::default()
			RelativeCursorPosition::default()
			Node {
				width: percent(100),
				height: percent(100),
				border_radius: BorderRadius::all(px(editor_layout::PANEL_RADIUS - 1.0)),
			}
		]
	}
}
