use bevy::prelude::*;

mod buttons;
mod general;

pub use buttons::{ButtonOptions, ButtonStyle, icon_button};
pub use general::separator_vertical;

pub struct WidgetsPlugin;

impl Plugin for WidgetsPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((buttons::ButtonPlugin, general::GeneralPlugin));
	}
}
