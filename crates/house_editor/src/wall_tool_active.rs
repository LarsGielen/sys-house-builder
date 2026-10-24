use bevy::prelude::*;

/// What a left click in the viewport does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tool {
	#[default]
	Select,
	DrawWall,
	DrawArc,
	PlaceOpening,
	Delete,
}

/// Owns which tool is active. Tools read it; only [`ActivateTool`] changes it.
#[derive(Resource, Default, PartialEq)]
pub struct ActiveTool(pub Tool);

/// Asks for a tool to become active. The tool system does not know what triggers it, so
/// buttons, shortcuts, or other tools can all send it.
#[derive(Message)]
pub struct ActivateTool(pub Tool);

pub struct WallToolActivePlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WallToolActiveSystems;

impl Plugin for WallToolActivePlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<ActiveTool>()
			.add_message::<ActivateTool>()
			.add_systems(PostUpdate, activate_tool.in_set(WallToolActiveSystems));
	}
}

/// Run condition for systems that belong to one tool.
pub fn tool_active(tool: Tool) -> impl FnMut(Res<ActiveTool>) -> bool + Clone {
	move |active_tool| active_tool.0 == tool
}

fn activate_tool(mut messages: MessageReader<ActivateTool>, mut active_tool: ResMut<ActiveTool>) {
	if let Some(&ActivateTool(tool)) = messages.read().last() {
		active_tool.set_if_neq(ActiveTool(tool));
	}
}

#[cfg(test)]
#[path = "tests/wall_tool_active.rs"]
mod tests;
