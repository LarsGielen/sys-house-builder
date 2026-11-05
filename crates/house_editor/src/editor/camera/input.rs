use crate::core::input::{Action, ActionGroup, AxisMode, ValueKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraAction {
	Orbit,
	Pan,
	Look,
	Zoom,
	Move,
}

impl Action for CameraAction {
	// Stored in the keymap file, so they must stay stable even if a variant is renamed.
	fn name(self) -> &'static str {
		match self {
			CameraAction::Orbit => "camera.orbit",
			CameraAction::Pan => "camera.pan",
			CameraAction::Look => "camera.look",
			CameraAction::Zoom => "camera.zoom",
			CameraAction::Move => "camera.move",
		}
	}

	fn value(self) -> ValueKind {
		match self {
			CameraAction::Orbit => ValueKind::Button,
			CameraAction::Pan => ValueKind::Button,
			CameraAction::Look => ValueKind::Axis2D(AxisMode::Relative),
			CameraAction::Zoom => ValueKind::Axis1D(AxisMode::Relative),
			CameraAction::Move => ValueKind::Axis2D(AxisMode::Absolute),
		}
	}
}

impl ActionGroup for CameraAction {
	fn all() -> &'static [Self] {
		&[Self::Orbit, Self::Pan, Self::Look, Self::Zoom, Self::Move]
	}
}
