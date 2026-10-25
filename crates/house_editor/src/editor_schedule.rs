use bevy::{
	camera::CameraUpdateSystems,
	prelude::*,
	text::{EditableTextSystems, detect_text_needs_rerender},
	ui::{UiSystems, widget::measure_text_system},
};

use crate::{
	actions::settings::SettingsActionSystems, document::history::HistorySystems,
	document::project::ProjectSystems, tools::active::WallToolActiveSystems,
	tools::arc::WallToolArcSystems, tools::delete::WallToolDeleteSystems,
	tools::drawing::WallToolDrawingSystems, tools::edit_opening::WallToolEditOpeningSystems,
	tools::edit_wall::WallToolEditWallSystems, tools::handles::WallToolHandlesSystems,
	tools::opening::WallToolOpeningSystems, tools::selection::WallToolSelectionSystems,
	ui::arc_fields::ArcFieldsSystems, ui::fields::TextFieldsSystems,
	ui::opening_fields::OpeningFieldsSystems, ui::panel::EditorUiSystems,
	ui::wall_fields::WallFieldsSystems, viewport::ground_pointer::GroundPointerSystems,
	viewport::orbit_camera::OrbitCameraSystems, viewport::snapping::SnappingSystems,
	viewport::wall_render::WallRenderSystems,
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
				SettingsActionSystems,
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
