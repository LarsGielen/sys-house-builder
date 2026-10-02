use std::f32::consts::FRAC_PI_2;

use bevy::{
	input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit},
	input_focus::InputFocus,
	prelude::*,
	window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};

const CAMERA_DISTANCE_MIN: f32 = 0.5;
const CAMERA_DISTANCE_MAX: f32 = 100.0;
const RESET_VIEW_RADIUS: f32 = 5.0;
const ORBIT_BUTTON: MouseButton = MouseButton::Right;
const PAN_BUTTON: MouseButton = MouseButton::Middle;
const ORBIT_RADIANS_PER_PIXEL: f32 = 0.005;
const CAMERA_ELEVATION_MIN: f32 = 0.3;
const CAMERA_ELEVATION_MAX: f32 = FRAC_PI_2 - 0.01;
const PAN_FOCUS_DISTANCES_PER_PIXEL: f32 = 0.0015;
const KEYBOARD_PAN_FOCUS_DISTANCES_PER_SECOND: f32 = 1.0;
const ZOOM_FACTOR_PER_LINE: f32 = 1.1;
const SCROLL_PIXELS_PER_LINE: f32 = 40.0;

pub struct OrbitCameraPlugin;

impl Plugin for OrbitCameraPlugin {
	fn build(&self, app: &mut App) {
		// Reset runs first so this frame's orbit/pan/zoom input applies on top of the reset view.
		// `update_orbit_gesture` always runs so an unfocused window cancels a drag in progress.
		let orbit_camera_system = (
			update_orbit_gesture,
			request_orbit_camera_reset_from_keyboard.run_if(viewport_keyboard_input_allowed),
			handle_reset_orbit_camera,
			apply_orbit_camera_input_orbit,
			apply_orbit_camera_input_pan,
			apply_orbit_camera_input_keyboard_pan.run_if(viewport_keyboard_input_allowed),
			apply_orbit_camera_input_zoom.run_if(window_focused.and_then(not(pointer_over_ui))),
			apply_orbit_camera,
		);

		app.init_resource::<OrbitGesture>()
			.add_message::<ResetOrbitCamera>()
			.add_systems(Update, orbit_camera_system.chain());
	}
}

/// Orbit camera state, in radians and metres. The camera's `Transform` is derived from it.
#[derive(Component)]
pub struct OrbitCamera {
	target_point: Vec3,
	azimuth: f32,
	elevation: f32,
	distance_from_focus: f32,
}

impl Default for OrbitCamera {
	fn default() -> Self {
		Self {
			target_point: Vec3::ZERO,
			azimuth: 0.0,
			elevation: 0.5,
			distance_from_focus: 10.0,
		}
	}
}

fn clamp_camera_distance(distance: f32) -> f32 {
	distance.clamp(CAMERA_DISTANCE_MIN, CAMERA_DISTANCE_MAX)
}

impl OrbitCamera {
	/// Looks at `center` from far enough away that a sphere of `radius` fits in view.
	/// Keeps the current viewing angle.
	fn frame(&mut self, center: Vec3, radius: f32, vertical_fov: f32, aspect_ratio: f32) {
		self.target_point = center;
		let vertical_half_fov = vertical_fov / 2.0;
		let horizontal_half_fov = (vertical_half_fov.tan() * aspect_ratio).atan();
		let limiting_half_fov = vertical_half_fov.min(horizontal_half_fov);
		self.distance_from_focus = clamp_camera_distance(radius / limiting_half_fov.sin());
	}

	/// Returns to the default viewing angle, framing the origin.
	fn reset(&mut self, vertical_fov: f32, aspect_ratio: f32) {
		let defaults = Self::default();
		self.azimuth = defaults.azimuth;
		self.elevation = defaults.elevation;
		self.frame(Vec3::ZERO, RESET_VIEW_RADIUS, vertical_fov, aspect_ratio);
	}
}

#[derive(Message)]
struct ResetOrbitCamera;

fn request_orbit_camera_reset_from_keyboard(
	keyboard: Res<ButtonInput<KeyCode>>,
	mut requests: MessageWriter<ResetOrbitCamera>,
) {
	if keyboard.just_pressed(KeyCode::Home) {
		requests.write(ResetOrbitCamera);
	}
}

fn handle_reset_orbit_camera(
	mut requests: MessageReader<ResetOrbitCamera>,
	camera: Single<(&mut OrbitCamera, &Projection)>,
) {
	if requests.read().count() == 0 {
		return;
	}

	let (mut orbit_camera, projection) = camera.into_inner();
	let Projection::Perspective(perspective) = projection else {
		warn!("Cannot reset the editor camera: projection is not perspective");
		return;
	};
	orbit_camera.reset(perspective.fov, perspective.aspect_ratio);
}

fn apply_orbit_camera(camera: Single<(&mut Transform, &OrbitCamera)>) {
	let (mut transform, orbit_camera) = camera.into_inner();

	// Negated because a positive pitch tilts the view up, but a raised camera must look down.
	let rotation = Quat::from_euler(
		EulerRot::YXZ,
		orbit_camera.azimuth,
		-orbit_camera.elevation,
		0.0,
	);

	// The camera looks along its local -Z, so it sits behind the target along +Z.
	transform.rotation = rotation;
	transform.translation =
		orbit_camera.target_point + rotation * (Vec3::Z * orbit_camera.distance_from_focus);
}

fn window_focused(window: Single<&Window, With<PrimaryWindow>>) -> bool {
	window.focused
}

fn pointer_over_ui(interactions: Query<&Interaction>) -> bool {
	is_ui_interacted(&interactions)
}

fn viewport_keyboard_input_allowed(
	window: Single<(Entity, &Window), With<PrimaryWindow>>,
	interactions: Query<&Interaction>,
	input_focus: Option<Res<InputFocus>>,
) -> bool {
	let (window_entity, window) = *window;
	window.focused
		&& !is_ui_interacted(&interactions)
		&& input_focus.is_none_or(|focus| focus.get().is_none_or(|entity| entity == window_entity))
}

fn is_ui_interacted(interactions: &Query<&Interaction>) -> bool {
	interactions
		.iter()
		.any(|interaction| *interaction != Interaction::None)
}

#[derive(Resource, Default)]
enum OrbitGesture {
	#[default]
	Idle,
	Orbiting {
		cursor_position: Option<Vec2>,
	},
	Panning {
		cursor_position: Option<Vec2>,
	},
}

fn update_orbit_gesture(
	mut gesture: ResMut<OrbitGesture>,
	mut window_and_cursor: Single<(&mut Window, &mut CursorOptions), With<PrimaryWindow>>,
	mouse_button: Res<ButtonInput<MouseButton>>,
	interactions: Query<&Interaction>,
) {
	let (window, cursor) = &mut *window_and_cursor;
	if !window.focused {
		finish_orbit_gesture(&mut gesture, window, cursor);
		return;
	}

	let held_button = match *gesture {
		OrbitGesture::Idle => None,
		OrbitGesture::Orbiting { .. } => Some(ORBIT_BUTTON),
		OrbitGesture::Panning { .. } => Some(PAN_BUTTON),
	};
	if held_button.is_some_and(|button| mouse_button.pressed(button)) {
		return;
	}
	finish_orbit_gesture(&mut gesture, window, cursor);

	if is_ui_interacted(&interactions) {
		return;
	}

	let cursor_position = window.cursor_position();
	if mouse_button.just_pressed(ORBIT_BUTTON) {
		*gesture = OrbitGesture::Orbiting { cursor_position };
	} else if mouse_button.just_pressed(PAN_BUTTON) {
		*gesture = OrbitGesture::Panning { cursor_position };
	} else {
		return;
	}
	cursor.visible = false;
	cursor.grab_mode = CursorGrabMode::Locked;
}

fn finish_orbit_gesture(
	gesture: &mut OrbitGesture,
	window: &mut Window,
	cursor: &mut CursorOptions,
) {
	let cursor_position = match std::mem::take(gesture) {
		OrbitGesture::Idle => return,
		OrbitGesture::Orbiting { cursor_position } | OrbitGesture::Panning { cursor_position } => {
			cursor_position
		}
	};
	cursor.grab_mode = CursorGrabMode::None;
	if let Some(cursor_position) = cursor_position {
		window.set_cursor_position(Some(cursor_position));
	}
	cursor.visible = true;
}

fn apply_orbit_camera_input_orbit(
	mut camera: Single<&mut OrbitCamera>,
	gesture: Res<OrbitGesture>,
	mouse_motion: Res<AccumulatedMouseMotion>,
) {
	if !matches!(*gesture, OrbitGesture::Orbiting { .. }) {
		return;
	}

	camera.azimuth -= mouse_motion.delta.x * ORBIT_RADIANS_PER_PIXEL;
	camera.elevation += mouse_motion.delta.y * ORBIT_RADIANS_PER_PIXEL;
	camera.elevation = camera
		.elevation
		.clamp(CAMERA_ELEVATION_MIN, CAMERA_ELEVATION_MAX);
}

fn apply_orbit_camera_input_pan(
	mut camera: Single<&mut OrbitCamera>,
	gesture: Res<OrbitGesture>,
	mouse_motion: Res<AccumulatedMouseMotion>,
) {
	if !matches!(*gesture, OrbitGesture::Panning { .. }) {
		return;
	}

	// A ground ray becomes unstable near the horizon; pointer motion stays bounded.
	let yaw = Quat::from_rotation_y(camera.azimuth);
	let pointer_motion = mouse_motion.delta;
	let ground_motion = yaw * Vec3::new(-pointer_motion.x, 0.0, -pointer_motion.y);
	let pan_distance = camera.distance_from_focus * PAN_FOCUS_DISTANCES_PER_PIXEL;
	camera.target_point += ground_motion * pan_distance;
}

fn apply_orbit_camera_input_keyboard_pan(
	mut camera: Single<&mut OrbitCamera>,
	keyboard: Res<ButtonInput<KeyCode>>,
	time: Res<Time>,
) {
	let axis = |positive: KeyCode, negative: KeyCode| {
		f32::from(keyboard.pressed(positive)) - f32::from(keyboard.pressed(negative))
	};
	let direction = Vec2::new(
		axis(KeyCode::KeyD, KeyCode::KeyA),
		axis(KeyCode::KeyW, KeyCode::KeyS),
	);

	// Normalised so diagonal movement is not faster, and zero when opposing keys cancel out.
	let direction = direction.normalize_or_zero();
	if direction == Vec2::ZERO {
		return;
	}

	// Same yaw-only frame as the mouse pan: the focus stays on the ground at any elevation.
	let yaw = Quat::from_rotation_y(camera.azimuth);
	let distance =
		camera.distance_from_focus * KEYBOARD_PAN_FOCUS_DISTANCES_PER_SECOND * time.delta_secs();
	camera.target_point += yaw * Vec3::new(direction.x, 0.0, -direction.y) * distance;
}

fn apply_orbit_camera_input_zoom(
	mut camera: Single<&mut OrbitCamera>,
	mouse_scroll: Res<AccumulatedMouseScroll>,
) {
	let scroll_lines = match mouse_scroll.unit {
		MouseScrollUnit::Line => mouse_scroll.delta.y,
		MouseScrollUnit::Pixel => mouse_scroll.delta.y / SCROLL_PIXELS_PER_LINE,
	};

	if scroll_lines == 0.0 {
		return;
	}

	// Scrolling up (positive) zooms in, so the exponent is negated.
	camera.distance_from_focus = clamp_camera_distance(
		camera.distance_from_focus * ZOOM_FACTOR_PER_LINE.powf(-scroll_lines),
	);
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn framing_moves_back_for_a_narrow_viewport() {
		let mut landscape = OrbitCamera::default();
		landscape.frame(Vec3::ZERO, 5.0, FRAC_PI_2, 2.0);

		let mut portrait = OrbitCamera::default();
		portrait.frame(Vec3::ZERO, 5.0, FRAC_PI_2, 0.5);

		assert!(portrait.distance_from_focus > landscape.distance_from_focus);
	}
}
