//! The house editor as a Bevy plugin: viewport, tools, panel, and project handling over the
//! engine-independent `house_document` and `house_editor_core` crates.
//!
//! [`HouseEditorPlugin`] installs everything. Controls come from [`Bindings`], and any system
//! can read [`ActionState`] to see what the user is doing in the editor's terms.

mod actions;
mod config;
mod document;
mod editor_schedule;
mod input;
mod scene;
mod tools;
mod ui;
mod viewport;

pub use actions::{ActionSender, EditorAction};
pub use config::camera::{CameraSettings, InvalidCameraSetting};
pub use config::interaction::InteractionConfig;
pub use document::house::House;
pub use input::{Action, ActionState, Bindings};
pub use scene::setup_scene;
pub use viewport::orbit_camera::OrbitCamera;

use bevy::prelude::*;
use document::history::HistoryPlugin;
use document::project::ProjectPlugin;
use editor_schedule::EditorSchedulePlugin;
use tools::active::WallToolActivePlugin;
use tools::arc::WallToolArcPlugin;
use tools::delete::WallToolDeletePlugin;
use tools::drawing::WallToolDrawingPlugin;
use tools::edit_opening::WallToolEditOpeningPlugin;
use tools::edit_wall::WallToolEditWallPlugin;
use tools::handles::WallToolHandlesPlugin;
use tools::opening::WallToolOpeningPlugin;
use tools::selection::WallToolSelectionPlugin;
use ui::arc_fields::ArcFieldsPlugin;
use ui::fields::TextFieldsPlugin;
use ui::opening_fields::OpeningFieldsPlugin;
use ui::panel::EditorUiPlugin;
use ui::wall_fields::WallFieldsPlugin;
use viewport::ground_pointer::GroundPointerPlugin;
use viewport::orbit_camera::OrbitCameraPlugin;
use viewport::snapping::SnappingPlugin;
use viewport::wall_render::WallRenderPlugin;

/// Installs the whole editor. The host adds Bevy's default plugins and a scene. It should set
/// `close_when_requested: false` on its window plugin, since closing asks about unsaved changes
/// first.
pub struct HouseEditorPlugin;

impl Plugin for HouseEditorPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((
			EditorSchedulePlugin,
			EditorUiPlugin,
			HistoryPlugin,
			OrbitCameraPlugin,
			WallRenderPlugin,
			GroundPointerPlugin,
			SnappingPlugin,
			WallToolActivePlugin,
			WallToolDrawingPlugin,
			WallToolOpeningPlugin,
			WallToolDeletePlugin,
			WallToolSelectionPlugin,
		))
		.add_plugins((
			WallToolEditOpeningPlugin,
			WallToolEditWallPlugin,
			TextFieldsPlugin,
			OpeningFieldsPlugin,
			WallFieldsPlugin,
			WallToolArcPlugin,
			ArcFieldsPlugin,
			WallToolHandlesPlugin,
			ProjectPlugin,
		))
		.init_resource::<House>();
	}
}

#[cfg(test)]
#[path = "tests/editor.rs"]
mod tests;
