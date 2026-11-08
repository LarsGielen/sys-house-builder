use bevy::prelude::*;

mod buttons;
mod general;
mod toggle;
mod tooltip;

pub use buttons::{ButtonOptions, icon_button};
pub use general::separator_vertical;
pub use toggle::toggle;

pub struct WidgetsPlugin;

impl Plugin for WidgetsPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((
			buttons::ButtonPlugin,
			general::GeneralPlugin,
			tooltip::TooltipPlugin,
		));
	}
}
