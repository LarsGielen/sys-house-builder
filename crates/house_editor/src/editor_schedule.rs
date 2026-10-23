use bevy::{camera::CameraUpdateSystems, prelude::*, ui::UiSystems};

use crate::{
	editor_ui::EditorUiSystems, ground_pointer::GroundPointerSystems,
	orbit_camera::OrbitCameraSystems, snapping::SnappingSystems, wall_render::WallRenderSystems,
	wall_tool_active::WallToolActiveSystems, wall_tool_drawing::WallToolDrawingSystems,
	wall_tool_opening::WallToolOpeningSystems, wall_tool_selection::WallToolSelectionSystems,
};

/// Composes the plugins' own system sets into the house editor's frame sequence.
pub struct EditorSchedulePlugin;

impl Plugin for EditorSchedulePlugin {
	fn build(&self, app: &mut App) {
		app.configure_sets(
			Update,
			(EditorUiSystems::Settings, OrbitCameraSystems).chain(),
		)
		// Picking computes the current camera transform without waiting for propagation.
		.configure_sets(
			PostUpdate,
			(
				WallToolActiveSystems,
				GroundPointerSystems::Pick,
				SnappingSystems::Pick,
				WallToolDrawingSystems,
				WallToolOpeningSystems,
				WallToolSelectionSystems::Pick,
			)
				.chain()
				.after(CameraUpdateSystems),
		)
		// Presentation finishes before layout, visibility, and transform propagation.
		.configure_sets(
			PostUpdate,
			(
				GroundPointerSystems::Presentation,
				SnappingSystems::Presentation,
				WallToolSelectionSystems::Presentation,
				WallRenderSystems,
				EditorUiSystems::Readouts,
			)
				.after(WallToolSelectionSystems::Pick)
				.before(UiSystems::Content)
				.before(TransformSystems::Propagate),
		);
	}
}
