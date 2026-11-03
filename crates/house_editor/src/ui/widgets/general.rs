use bevy::prelude::*;

use crate::ui::theme::{colors, spacing};

pub(super) struct GeneralPlugin;
impl Plugin for GeneralPlugin {
	fn build(&self, _app: &mut App) {}
}

pub fn separator_vertical() -> impl Scene {
	bsn! {
		Node { width: px(1), height: px(24), flex_shrink: 0.0, margin: UiRect::horizontal(px(spacing::TINY)) }
		BackgroundColor(colors::BORDER)
	}
}
