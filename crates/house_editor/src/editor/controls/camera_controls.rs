use bevy::{
	ecs::system::SystemParam,
	input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit},
	picking::hover::Hovered,
	prelude::*,
	window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};

use crate::editor::{camera::OrbitCamera, controls::CameraAction, input::Keymap};

const ORBIT_RADIANS_PER_PIXEL: f32 = 0.01;
const PAN_PER_PIXEL: f32 = 0.0011;
const ZOOM_PER_SCROLL_LINE: f32 = 0.1;
const ZOOM_PER_SCROLL_PIXEL: f32 = 0.0025;

/// A drag stays tied to the camera it started on, even if the cursor leaves the viewport.
#[derive(Clone, Copy)]
struct CameraDrag {
	camera: Entity,
	control: CameraAction,
}

/// The drag in progress, if any. A resource so that `update_cursor_lock` can follow it.
#[derive(Resource, Default)]
pub(super) struct ActiveCameraDrag(Option<CameraDrag>);

/// Everything the camera controls read from the mouse, keyboard and keymap.
#[derive(SystemParam)]
pub struct CameraInput<'w> {
	keys: Res<'w, ButtonInput<KeyCode>>,
	mouse_buttons: Res<'w, ButtonInput<MouseButton>>,
	keymap: Res<'w, Keymap>,
	mouse_motion: Res<'w, AccumulatedMouseMotion>,
	mouse_scroll: Res<'w, AccumulatedMouseScroll>,
}

impl CameraInput<'_> {
	fn control_pressed(&self) -> Option<CameraAction> {
		self.keymap
			.camera_control_pressed(&self.keys, &self.mouse_buttons)
	}

	fn control_held(&self, control: CameraAction) -> bool {
		self.keymap
			.camera_control_held(control, &self.keys, &self.mouse_buttons)
	}
}

pub fn update_camera_controls(
	input: CameraInput,
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
		&& !input.control_held(drag.control)
	{
		active_drag.0 = None;
	}

	if active_drag.0.is_none()
		&& let Some(camera) = hovered_camera
		&& let Some(control) = input.control_pressed()
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
		let delta = input.mouse_motion.delta;
		if delta != Vec2::ZERO {
			match drag.control {
				// Dragging right turns the scene right, so the camera moves the other way.
				CameraAction::Orbit => camera.orbit(
					-delta.x * ORBIT_RADIANS_PER_PIXEL,
					delta.y * ORBIT_RADIANS_PER_PIXEL,
				),
				// The ground follows the cursor, so the focus moves opposite to the horizontal drag.
				CameraAction::Pan => camera.pan(-delta.x * PAN_PER_PIXEL, delta.y * PAN_PER_PIXEL),
			}
		}
	}

	if let Some(camera_entity) = hovered_camera
		&& input.mouse_scroll.delta.y != 0.0
		&& let Ok(mut camera) = cameras.get_mut(camera_entity)
	{
		camera.zoom(zoom_factor(
			input.mouse_scroll.unit,
			input.mouse_scroll.delta.y,
		));
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

fn zoom_factor(unit: MouseScrollUnit, scroll_y: f32) -> f32 {
	let zoom_per_unit = match unit {
		MouseScrollUnit::Line => ZOOM_PER_SCROLL_LINE,
		MouseScrollUnit::Pixel => ZOOM_PER_SCROLL_PIXEL,
	};
	(-scroll_y * zoom_per_unit).exp()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn scrolling_up_zooms_in_and_down_zooms_out() {
		assert!(zoom_factor(MouseScrollUnit::Line, 1.0) < 1.0);
		assert!(zoom_factor(MouseScrollUnit::Line, -1.0) > 1.0);
	}

	#[test]
	fn opposite_scrolls_cancel_out() {
		let factor =
			zoom_factor(MouseScrollUnit::Pixel, 30.0) * zoom_factor(MouseScrollUnit::Pixel, -30.0);
		assert!((factor - 1.0).abs() < 1e-6);
	}

	#[test]
	fn a_scroll_line_zooms_more_than_a_scroll_pixel() {
		assert!(zoom_factor(MouseScrollUnit::Line, 1.0) < zoom_factor(MouseScrollUnit::Pixel, 1.0));
	}
}
