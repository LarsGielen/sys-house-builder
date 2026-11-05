use bevy::{
	picking::hover::Hovered,
	prelude::*,
	window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};

use crate::editor::{camera::OrbitCamera, controls::CameraAction, input::ActionState};

const ORBIT_RADIANS_PER_PIXEL: f32 = 0.01;
const PAN_PER_PIXEL: f32 = 0.0011;
const ZOOM_PER_SCROLL_LINE: f32 = 0.1;

/// A drag stays tied to the camera it started on, even if the cursor leaves the viewport.
#[derive(Clone, Copy)]
struct CameraDrag {
	camera: Entity,
	control: CameraAction,
}

/// The drag in progress, if any. A resource so that `update_cursor_lock` can follow it.
#[derive(Resource, Default)]
pub(super) struct ActiveCameraDrag(Option<CameraDrag>);

pub fn update_camera_controls(
	state: Res<ActionState>,
	viewports: Query<(&ViewportNode, &Hovered)>,
	mut cameras: Query<&mut OrbitCamera>,
	mut active_drag: ResMut<ActiveCameraDrag>,
) {
	let hovered_camera = viewports
		.iter()
		.find(|(_, hovered)| hovered.get())
		.and_then(|(viewport, _)| viewport.camera);

	// Checked before starting a drag so that a missed release can never leave a drag stuck.
	if let Some(drag) = active_drag.0
		&& !state.pressed(drag.control)
	{
		active_drag.0 = None;
	}

	if active_drag.0.is_none()
		&& let Some(camera) = hovered_camera
		&& let Some(control) = [CameraAction::Orbit, CameraAction::Pan]
			.into_iter()
			.find(|&action| state.just_pressed(action))
	{
		active_drag.0 = Some(CameraDrag { camera, control });
	}

	if let Some(drag) = active_drag.0 {
		let Ok(mut camera) = cameras.get_mut(drag.camera) else {
			// The camera was despawned during the drag.
			active_drag.0 = None;
			return;
		};

		// Skipping zero deltas keeps `Changed<OrbitCamera>` from firing on every idle frame.
		let delta = state.axis2d(CameraAction::Look);
		if delta != Vec2::ZERO {
			match drag.control {
				// Dragging right turns the scene right, so the camera moves the other way.
				CameraAction::Orbit => camera.orbit(
					-delta.x * ORBIT_RADIANS_PER_PIXEL,
					delta.y * ORBIT_RADIANS_PER_PIXEL,
				),
				// The ground follows the cursor, so the focus moves opposite to the horizontal drag.
				CameraAction::Pan => camera.pan(-delta.x * PAN_PER_PIXEL, delta.y * PAN_PER_PIXEL),
				// Axes are never bound to shortcuts, so they cannot start a drag. This goes away
				// when the camera reads the action state.
				CameraAction::Look | CameraAction::Zoom | CameraAction::Move => {}
			}
		}
	}

	let zoom = state.axis1d(CameraAction::Zoom);
	if let Some(camera_entity) = hovered_camera
		&& zoom != 0.0
		&& let Ok(mut camera) = cameras.get_mut(camera_entity)
	{
		camera.zoom(zoom_factor(zoom));
	}
}

/// Hides and locks the cursor while dragging, so the camera follows raw mouse motion without a
/// cursor that disagrees with it, and the drag is not limited by the edge of the screen.
pub(super) fn update_cursor_lock(
	active_drag: Res<ActiveCameraDrag>,
	mut cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
	let is_dragging = active_drag.0.is_some();
	cursor_options.visible = !is_dragging;
	cursor_options.grab_mode = if is_dragging {
		CursorGrabMode::Locked
	} else {
		CursorGrabMode::None
	};
}

fn zoom_factor(lines: f32) -> f32 {
	(-lines * ZOOM_PER_SCROLL_LINE).exp()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn scrolling_up_zooms_in_and_down_zooms_out() {
		assert!(zoom_factor(1.0) < 1.0);
		assert!(zoom_factor(-1.0) > 1.0);
	}

	#[test]
	fn opposite_scrolls_cancel_out() {
		let factor = zoom_factor(30.0) * zoom_factor(-30.0);
		assert!((factor - 1.0).abs() < 1e-6);
	}
}
