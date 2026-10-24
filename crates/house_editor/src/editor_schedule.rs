use bevy::{
	camera::CameraUpdateSystems,
	prelude::*,
	text::{EditableTextSystems, detect_text_needs_rerender},
	ui::{UiSystems, widget::measure_text_system},
};

use crate::{
	arc_fields::ArcFieldsSystems, editor_ui::EditorUiSystems, ground_pointer::GroundPointerSystems,
	history::HistorySystems, opening_fields::OpeningFieldsSystems,
	orbit_camera::OrbitCameraSystems, project::ProjectSystems, snapping::SnappingSystems,
	text_fields::TextFieldsSystems, wall_fields::WallFieldsSystems, wall_render::WallRenderSystems,
	wall_tool_active::WallToolActiveSystems, wall_tool_arc::WallToolArcSystems,
	wall_tool_delete::WallToolDeleteSystems, wall_tool_drawing::WallToolDrawingSystems,
	wall_tool_edit_opening::WallToolEditOpeningSystems,
	wall_tool_edit_wall::WallToolEditWallSystems, wall_tool_handles::WallToolHandlesSystems,
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
				ProjectSystems,
				// Restoring first lets every tool see the restored house and its discarded gestures.
				HistorySystems,
				WallToolActiveSystems,
				GroundPointerSystems::Pick,
				SnappingSystems::Pick,
				WallToolDrawingSystems,
				WallToolArcSystems,
				WallToolOpeningSystems,
				WallToolDeleteSystems,
				WallToolHandlesSystems::Hover,
				WallToolSelectionSystems::Pick,
				WallToolHandlesSystems::Drag,
				WallToolEditOpeningSystems,
				WallToolEditWallSystems,
			)
				.chain()
				.after(CameraUpdateSystems)
				.before(EditableTextSystems),
		)
		// Input widgets queue edits until this set. Submission must see this frame's text.
		.configure_sets(
			PostUpdate,
			(
				TextFieldsSystems,
				OpeningFieldsSystems,
				WallFieldsSystems,
				ArcFieldsSystems,
			)
				.chain()
				.after(EditableTextSystems)
				.after(WallToolEditWallSystems),
		)
		// Readouts are updated before text measurement; presentation precedes propagation.
		.configure_sets(
			PostUpdate,
			(
				GroundPointerSystems::Presentation,
				SnappingSystems::Presentation,
				WallToolSelectionSystems::Presentation,
				WallRenderSystems,
				EditorUiSystems::Readouts,
			)
				.after(ArcFieldsSystems)
				.before(detect_text_needs_rerender)
				.before(measure_text_system)
				.before(UiSystems::Layout)
				.before(TransformSystems::Propagate),
		);
	}
}
