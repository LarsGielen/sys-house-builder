use bevy::{camera::CameraUpdateSystems, prelude::*, ui::UiSystems};

use crate::{
	editor_ui::EditorUiSystems, ground_pointer::GroundPointerSystems,
	orbit_camera::OrbitCameraSystems, snapping::SnappingSystems, wall_drawing::WallDrawingSystems,
	wall_render::WallRenderSystems,
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
				GroundPointerSystems::Pick,
				SnappingSystems::Pick,
				WallDrawingSystems,
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
				WallRenderSystems,
				EditorUiSystems::Readouts,
			)
				.after(WallDrawingSystems)
				.before(UiSystems::Content)
				.before(TransformSystems::Propagate),
		);
	}
}
