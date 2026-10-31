use bevy::prelude::*;
use house_document::OpeningCategory;

use house_editor_core::ArcMode;

use crate::actions::EditorActionPlugin;
use crate::tools::drawing::StepDirection;

/// A request to change an editor setting. Buttons and fields send these, and the module that
/// owns each setting applies them, so no widget writes a setting itself.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SettingsAction {
	StepWallHeight(StepDirection),
	StepWallThickness(StepDirection),
	SetArcMode(ArcMode),
	SetOpeningCategory(OpeningCategory),
	StepGridSpacing(isize),
	/// A gap that may be invalid; the owner refuses it and says why.
	SetOpeningGap(f32),
}

/// Applies settings before tools run, so a change takes effect in the frame it was asked for.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SettingsActionSystems;

pub struct SettingsActionPlugin;

impl SettingsActionPlugin {
	pub fn ensure(app: &mut App) {
		if !app.is_plugin_added::<Self>() {
			app.add_plugins(Self);
		}
	}
}

impl Plugin for SettingsActionPlugin {
	fn build(&self, app: &mut App) {
		EditorActionPlugin::ensure(app);
	}
}
