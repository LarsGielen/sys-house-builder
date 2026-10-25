use bevy::{
	input_focus::InputFocus,
	prelude::*,
	window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};

use crate::config::camera::CameraSettings;
use crate::input::{Action, ActionState, ActionsPlugin};

pub struct OrbitCameraPlugin;

/// All camera input and pose updates, in `Update`. Other plugins can order their
/// systems relative to this set without the camera depending on their schedules.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OrbitCameraSystems;

impl Plugin for OrbitCameraPlugin {
	fn build(&self, app: &mut App) {
		ActionsPlugin::ensure(app);
		app.init_resource::<CameraSettings>();
		// Reset runs first so this frame's orbit/pan/zoom input applies on top of the reset view.
		// `update_orbit_gesture` always runs so an unfocused window cancels a drag in progress.
		let orbit_camera_system = (
			update_orbit_gesture,
			request_orbit_camera_reset_from_keyboard.run_if(viewport_keyboard_input_allowed),
			handle_reset_orbit_camera,
			apply_mouse_orbit,
			apply_mouse_pan,
			apply_keyboard_pan.run_if(viewport_keyboard_input_allowed),
			apply_scroll_zoom.run_if(window_focused.and_then(not(pointer_over_ui))),
			apply_orbit_camera,
		);

		app.init_resource::<OrbitGesture>()
			.add_message::<ResetOrbitCamera>()
			.add_systems(
				Update,
				orbit_camera_system.chain().in_set(OrbitCameraSystems),
			);
	}
}

/// Orbit camera state, in radians and metres. The camera's `Transform` is derived from it.
#[derive(Component)]
pub struct OrbitCamera {
	focus_point: Vec3,
	azimuth: f32,
	elevation: f32,
	distance_from_focus: f32,
}

impl Default for OrbitCamera {
	fn default() -> Self {
		Self {
			focus_point: Vec3::ZERO,
			azimuth: 0.0,
			elevation: 0.5,
			distance_from_focus: 10.0,
		}
	}
}

impl OrbitCamera {
	/// Looks at `center` from far enough away that a sphere of `radius` fits in view.
	/// Keeps the current viewing angle. The maximum camera distance can limit the fit.
	fn frame(
		&mut self,
		settings: &CameraSettings,
		center: Vec3,
		radius: f32,
		vertical_fov: f32,
		aspect_ratio: f32,
	) {
		self.focus_point = center;
		let vertical_half_fov = vertical_fov / 2.0;
		let horizontal_half_fov = (vertical_half_fov.tan() * aspect_ratio).atan();
		let limiting_half_fov = vertical_half_fov.min(horizontal_half_fov);
		self.distance_from_focus = settings.clamp_distance(radius / limiting_half_fov.sin());
	}

	/// Returns to the default viewing angle, framing the origin.
	fn reset(&mut self, settings: &CameraSettings, vertical_fov: f32, aspect_ratio: f32) {
		let defaults = Self::default();
		self.azimuth = defaults.azimuth;
		self.elevation = defaults.elevation;
		self.frame(
			settings,
			Vec3::ZERO,
			settings.reset_view_radius(),
			vertical_fov,
			aspect_ratio,
		);
	}
}

#[derive(Message)]
struct ResetOrbitCamera;

fn request_orbit_camera_reset_from_keyboard(
	actions: Res<ActionState>,
	mut requests: MessageWriter<ResetOrbitCamera>,
) {
	if actions.just_pressed(Action::CameraReset) {
		requests.write(ResetOrbitCamera);
	}
}

fn handle_reset_orbit_camera(
	mut requests: MessageReader<ResetOrbitCamera>,
	settings: Res<CameraSettings>,
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
	orbit_camera.reset(&settings, perspective.fov, perspective.aspect_ratio);
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
		orbit_camera.focus_point + rotation * (Vec3::Z * orbit_camera.distance_from_focus);
}

fn window_focused(window: Single<&Window, With<PrimaryWindow>>) -> bool {
	window.focused
}

fn pointer_over_ui(interactions: Query<&Interaction>) -> bool {
	any_ui_interaction_active(&interactions)
}

fn viewport_keyboard_input_allowed(
	window: Single<(Entity, &Window), With<PrimaryWindow>>,
	interactions: Query<&Interaction>,
	input_focus: Option<Res<InputFocus>>,
) -> bool {
	let (window_entity, window) = *window;
	window.focused
		&& !any_ui_interaction_active(&interactions)
		&& input_focus.is_none_or(|focus| focus.get().is_none_or(|entity| entity == window_entity))
}

fn any_ui_interaction_active(interactions: &Query<&Interaction>) -> bool {
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
	actions: Res<ActionState>,
	interactions: Query<&Interaction>,
) {
	let (window, cursor) = &mut *window_and_cursor;
	if !window.focused {
		finish_orbit_gesture(&mut gesture, window, cursor);
		return;
	}

	let held_action = match *gesture {
		OrbitGesture::Idle => None,
		OrbitGesture::Orbiting { .. } => Some(Action::Orbit),
		OrbitGesture::Panning { .. } => Some(Action::Pan),
	};
	if held_action.is_some_and(|action| actions.pressed(action)) {
		return;
	}
	finish_orbit_gesture(&mut gesture, window, cursor);

	if any_ui_interaction_active(&interactions) {
		return;
	}

	let cursor_position = window.cursor_position();
	if actions.just_pressed(Action::Orbit) {
		*gesture = OrbitGesture::Orbiting { cursor_position };
	} else if actions.just_pressed(Action::Pan) {
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

fn apply_mouse_orbit(
	settings: Res<CameraSettings>,
	mut camera: Single<&mut OrbitCamera>,
	gesture: Res<OrbitGesture>,
	actions: Res<ActionState>,
) {
	if !matches!(*gesture, OrbitGesture::Orbiting { .. }) {
		return;
	}

	camera.azimuth -= actions.pointer_motion().x * settings.orbit_radians_per_pixel();
	camera.elevation += actions.pointer_motion().y * settings.orbit_radians_per_pixel();
	camera.elevation = settings.clamp_elevation(camera.elevation);
}

fn apply_mouse_pan(
	settings: Res<CameraSettings>,
	mut camera: Single<&mut OrbitCamera>,
	gesture: Res<OrbitGesture>,
	actions: Res<ActionState>,
) {
	if !matches!(*gesture, OrbitGesture::Panning { .. }) {
		return;
	}

	// A ground ray becomes unstable near the horizon; pointer motion stays bounded.
	let yaw = Quat::from_rotation_y(camera.azimuth);
	let pointer_motion = actions.pointer_motion();
	let ground_motion = yaw * Vec3::new(-pointer_motion.x, 0.0, -pointer_motion.y);
	let pan_distance = camera.distance_from_focus * settings.pan_focus_distances_per_pixel();
	camera.focus_point += ground_motion * pan_distance;
}

fn apply_keyboard_pan(
	settings: Res<CameraSettings>,
	mut camera: Single<&mut OrbitCamera>,
	actions: Res<ActionState>,
	time: Res<Time>,
) {
	let axis = |positive: Action, negative: Action| {
		f32::from(actions.pressed(positive)) - f32::from(actions.pressed(negative))
	};
	let direction = Vec2::new(
		axis(Action::PanRight, Action::PanLeft),
		axis(Action::PanForward, Action::PanBack),
	);

	// Normalised so diagonal movement is not faster, and zero when opposing keys cancel out.
	let direction = direction.normalize_or_zero();
	if direction == Vec2::ZERO {
		return;
	}

	// Same yaw-only frame as the mouse pan: the focus stays on the ground at any elevation.
	let yaw = Quat::from_rotation_y(camera.azimuth);
	let distance = camera.distance_from_focus
		* settings.keyboard_pan_focus_distances_per_second()
		* time.delta_secs();
	camera.focus_point += yaw * Vec3::new(direction.x, 0.0, -direction.y) * distance;
}

fn apply_scroll_zoom(
	settings: Res<CameraSettings>,
	mut camera: Single<&mut OrbitCamera>,
	actions: Res<ActionState>,
) {
	let scroll_lines = actions.scroll_lines();

	if scroll_lines == 0.0 {
		return;
	}

	// Scrolling up (positive) zooms in, so the exponent is negated.
	camera.distance_from_focus = settings.clamp_distance(
		camera.distance_from_focus * settings.zoom_factor_per_line().powf(-scroll_lines),
	);
}

#[cfg(test)]
#[path = "../tests/viewport/orbit_camera.rs"]
mod tests;
