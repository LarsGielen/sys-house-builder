use bevy::prelude::*;
use wall_graph::{OpeningId, OpeningSpec, Wall, WallDimensions};

use crate::actions::{EditorAction, EditorActionPlugin};
use crate::document::edit::EditPreview;

/// A typed property edit. Field widgets send these, naming the target and the value they read,
/// and the owner of each property validates and applies them. Nothing else about the widget
/// reaches the document, so any other source can make the same edit.
///
/// Every edit names the document revision it was made against and is refused if the document
/// has changed since, so a request that was delayed or queued behind an undo cannot land on a
/// document it was not made for.
///
/// A submit carries its own value: it is applied only if the preview for exactly that value is
/// valid, never because an earlier preview happened to be.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PropertyAction {
	PreviewWall {
		/// The document revision the value was read against.
		revision: u64,
		wall: Wall,
		property: WallProperty,
		value: f64,
	},
	SubmitWall {
		/// The document revision the value was read against.
		revision: u64,
		wall: Wall,
		property: WallProperty,
		value: f64,
	},
	SubmitOpening {
		/// The document revision the value was read against.
		revision: u64,
		opening: OpeningId,
		property: OpeningProperty,
		value: f64,
	},
	/// For the arc being drawn between `start` and `end`.
	PreviewArc {
		/// The document revision the value was read against.
		revision: u64,
		start: Vec2,
		end: Vec2,
		property: ArcProperty,
		value: f64,
	},
	SubmitArc {
		/// The document revision the value was read against.
		revision: u64,
		start: Vec2,
		end: Vec2,
		property: ArcProperty,
		value: f64,
	},
	/// Drop the draft a field was building, and any typed arc value.
	CancelDraft,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum WallProperty {
	Height,
	Thickness,
}

impl WallProperty {
	pub fn write(self, dimensions: WallDimensions, value: f64) -> WallDimensions {
		match self {
			Self::Height => WallDimensions {
				height: value as f32,
				..dimensions
			},
			Self::Thickness => WallDimensions {
				thickness: value as f32,
				..dimensions
			},
		}
	}

	pub fn is_representable(self, value: f64) -> bool {
		value.is_finite() && (value as f32).is_finite()
	}
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpeningProperty {
	/// Distance of the centre along the host wall.
	CenterDistance,
	Width,
	Height,
	Bottom,
}

impl OpeningProperty {
	pub fn write(self, spec: OpeningSpec, value: f64) -> OpeningSpec {
		match self {
			Self::CenterDistance => OpeningSpec {
				center_distance: value,
				..spec
			},
			Self::Width => OpeningSpec {
				width: value as f32,
				..spec
			},
			Self::Height => OpeningSpec {
				height: value as f32,
				..spec
			},
			Self::Bottom => OpeningSpec {
				bottom: value as f32,
				..spec
			},
		}
	}

	pub fn is_representable(self, value: f64) -> bool {
		value.is_finite() && (self == Self::CenterDistance || (value as f32).is_finite())
	}
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArcProperty {
	Angle,
	Radius,
}

pub struct PropertyActionPlugin;

impl PropertyActionPlugin {
	pub fn ensure(app: &mut App) {
		if !app.is_plugin_added::<Self>() {
			app.add_plugins(Self);
		}
	}
}

impl Plugin for PropertyActionPlugin {
	fn build(&self, app: &mut App) {
		EditorActionPlugin::ensure(app);
	}
}

/// A cancelled field draft leaves no preview behind.
pub fn discard_cancelled_draft(
	mut actions: MessageReader<EditorAction>,
	mut preview: ResMut<EditPreview>,
) {
	for action in actions.read().filter_map(|action| action.property()) {
		if action == PropertyAction::CancelDraft && preview.holds_field_draft() {
			preview.state = None;
		}
	}
}
