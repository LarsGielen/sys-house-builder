use bevy::prelude::*;

mod action;
mod signal;
mod state;

pub struct InputPlugin;

impl Plugin for InputPlugin {
	fn build(&self, app: &mut App) {}
}
