use std::f32::consts::FRAC_PI_2;

use bevy::{
	input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit},
	input_focus::InputFocus,
	log::{DEFAULT_FILTER, LogPlugin},
	prelude::*,
	ui::FocusPolicy,
	window::PrimaryWindow,
};
use wall_graph::WallGraph;

fn main() {
	// Reset runs first so this frame's orbit/pan/zoom input applies on top of the reset view.
	// `update_camera_gesture` always runs so an unfocused window cancels a drag in progress.
	let camera_system = (
		update_camera_gesture,
		request_camera_reset_from_keyboard.run_if(viewport_keyboard_input_allowed),
		handle_reset_camera,
		apply_editor_camera_input_orbit,
		apply_editor_camera_input_pan,
		apply_editor_camera_input_keyboard_pan.run_if(viewport_keyboard_input_allowed),
		apply_editor_camera_input_zoom.run_if(window_focused.and_then(not(pointer_over_ui))),
		apply_editor_camera,
	);

	App::new()
		.add_plugins(
			DefaultPlugins
				.set(WindowPlugin {
					primary_window: Some(Window {
						name: Some("bevy-dev".to_string()),
						title: "House Editor".to_string(),
						..default()
					}),
					..default()
				})
				// rodio logs a harmless ALSA timestamp error every frame on some setups.
				.set(LogPlugin {
					filter: format!("{DEFAULT_FILTER},rodio=off"),
					..default()
				}),
		)
		.init_resource::<House>()
		.init_resource::<CameraGesture>()
		.add_message::<ResetCamera>()
		.add_systems(Startup, (setup_scene, setup_editor_panel))
		.add_systems(Update, camera_system.chain())
		.run();
}

fn setup_scene(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	commands.spawn((
		Name::new("editor-floor"),
		Mesh3d(meshes.add(Rectangle::new(4.0, 4.0))),
		MeshMaterial3d(materials.add(Color::WHITE)),
		Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
	));

	commands.spawn((
		PointLight {
			shadow_maps_enabled: true,
			..default()
		},
		Transform::from_xyz(4.0, 8.0, 4.0),
	));

	commands.spawn((Camera3d::default(), EditorCamera::default()));
}

fn setup_editor_panel(mut commands: Commands) {
	commands
		.spawn((
			Name::new("editor-panel"),
			Node {
				position_type: PositionType::Absolute,
				left: Val::Px(0.0),
				top: Val::Px(0.0),
				width: Val::Px(240.0),
				height: Val::Percent(100.0),
				padding: UiRect::all(Val::Px(12.0)),
				..default()
			},
			BackgroundColor(Color::srgba(0.1, 0.1, 0.12, 0.9)),
			// Without `Interaction` the node is never hover-tested, and `Block` stops
			// the pointer from also counting as over anything underneath.
			Interaction::default(),
			FocusPolicy::Block,
		))
		.with_child((
			Button,
			Node {
				padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
				..default()
			},
			BackgroundColor(Color::srgb(0.25, 0.25, 0.3)),
			children![Text::new("Test button")],
		));
}

/// Orbit camera state, in radians and metres. The camera's `Transform` is derived from it.
#[derive(Component)]
struct EditorCamera {
	target_point: Vec3,
	azimuth: f32,
	elevation: f32,
	distance_from_focus: f32,
}

impl Default for EditorCamera {
	fn default() -> Self {
		Self {
			target_point: Vec3::ZERO,
			azimuth: 0.0,
			elevation: 0.5,
			distance_from_focus: 10.0,
		}
	}
}

const CAMERA_DISTANCE_MIN: f32 = 0.5;
const CAMERA_DISTANCE_MAX: f32 = 100.0;
const RESET_VIEW_RADIUS: f32 = 5.0;

fn clamp_camera_distance(distance: f32) -> f32 {
	distance.clamp(CAMERA_DISTANCE_MIN, CAMERA_DISTANCE_MAX)
}

impl EditorCamera {
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
struct ResetCamera;

fn request_camera_reset_from_keyboard(
	keyboard: Res<ButtonInput<KeyCode>>,
	mut requests: MessageWriter<ResetCamera>,
) {
	if keyboard.just_pressed(KeyCode::Home) {
		requests.write(ResetCamera);
	}
}

fn handle_reset_camera(
	mut requests: MessageReader<ResetCamera>,
	camera: Single<(&mut EditorCamera, &Projection)>,
) {
	if requests.read().count() == 0 {
		return;
	}

	let (mut editor_camera, projection) = camera.into_inner();
	let Projection::Perspective(perspective) = projection else {
		warn!("Cannot reset the editor camera: projection is not perspective");
		return;
	};
	editor_camera.reset(perspective.fov, perspective.aspect_ratio);
}

fn apply_editor_camera(camera: Single<(&mut Transform, &EditorCamera)>) {
	let (mut transform, editor_camera) = camera.into_inner();

	// Negated because a positive pitch tilts the view up, but a raised camera must look down.
	let rotation = Quat::from_euler(
		EulerRot::YXZ,
		editor_camera.azimuth,
		-editor_camera.elevation,
		0.0,
	);

	// The camera looks along its local -Z, so it sits behind the target along +Z.
	transform.rotation = rotation;
	transform.translation =
		editor_camera.target_point + rotation * (Vec3::Z * editor_camera.distance_from_focus);
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

#[derive(Resource, Default, PartialEq)]
enum CameraGesture {
	#[default]
	Idle,
	Orbiting,
	Panning,
}

const ORBIT_BUTTON: MouseButton = MouseButton::Right;
const PAN_BUTTON: MouseButton = MouseButton::Middle;

fn update_camera_gesture(
	mut gesture: ResMut<CameraGesture>,
	window: Single<&Window, With<PrimaryWindow>>,
	mouse_button: Res<ButtonInput<MouseButton>>,
	interactions: Query<&Interaction>,
) {
	if !window.focused {
		*gesture = CameraGesture::Idle;
		return;
	}

	let held_button = match *gesture {
		CameraGesture::Idle => None,
		CameraGesture::Orbiting => Some(ORBIT_BUTTON),
		CameraGesture::Panning => Some(PAN_BUTTON),
	};
	if held_button.is_some_and(|button| mouse_button.pressed(button)) {
		return;
	}
	*gesture = CameraGesture::Idle;

	if is_ui_interacted(&interactions) {
		return;
	}

	if mouse_button.just_pressed(ORBIT_BUTTON) {
		*gesture = CameraGesture::Orbiting;
	} else if mouse_button.just_pressed(PAN_BUTTON) {
		*gesture = CameraGesture::Panning;
	}
}

const ORBIT_RADIANS_PER_PIXEL: f32 = 0.005;
const CAMERA_ELEVATION_MIN: f32 = 0.3;
const CAMERA_ELEVATION_MAX: f32 = FRAC_PI_2 - 0.01;

fn apply_editor_camera_input_orbit(
	mut camera: Single<&mut EditorCamera>,
	gesture: Res<CameraGesture>,
	mouse_motion: Res<AccumulatedMouseMotion>,
) {
	if *gesture != CameraGesture::Orbiting {
		return;
	}

	camera.azimuth -= mouse_motion.delta.x * ORBIT_RADIANS_PER_PIXEL;
	camera.elevation += mouse_motion.delta.y * ORBIT_RADIANS_PER_PIXEL;
	camera.elevation = camera
		.elevation
		.clamp(CAMERA_ELEVATION_MIN, CAMERA_ELEVATION_MAX);
}

const PAN_FOCUS_DISTANCES_PER_PIXEL: f32 = 0.0015;

fn apply_editor_camera_input_pan(
	mut camera: Single<&mut EditorCamera>,
	gesture: Res<CameraGesture>,
	mouse_motion: Res<AccumulatedMouseMotion>,
) {
	if *gesture != CameraGesture::Panning {
		return;
	}

	// A ground ray becomes unstable near the horizon; pointer motion stays bounded.
	let yaw = Quat::from_rotation_y(camera.azimuth);
	let pointer_motion = mouse_motion.delta;
	let ground_motion = yaw * Vec3::new(-pointer_motion.x, 0.0, -pointer_motion.y);
	let pan_distance = camera.distance_from_focus * PAN_FOCUS_DISTANCES_PER_PIXEL;
	camera.target_point += ground_motion * pan_distance;
}

const KEYBOARD_PAN_FOCUS_DISTANCES_PER_SECOND: f32 = 1.0;

fn apply_editor_camera_input_keyboard_pan(
	mut camera: Single<&mut EditorCamera>,
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

const ZOOM_FACTOR_PER_LINE: f32 = 1.1;
const SCROLL_PIXELS_PER_LINE: f32 = 40.0;

fn apply_editor_camera_input_zoom(
	mut camera: Single<&mut EditorCamera>,
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

/// Keeps house geometry independent of Bevy entities.
#[derive(Resource, Default)]
struct House(
	#[expect(dead_code, reason = "wall placement will use the graph in step 2")] WallGraph,
);

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn framing_moves_back_for_a_narrow_viewport() {
		let mut landscape = EditorCamera::default();
		landscape.frame(Vec3::ZERO, 5.0, FRAC_PI_2, 2.0);

		let mut portrait = EditorCamera::default();
		portrait.frame(Vec3::ZERO, 5.0, FRAC_PI_2, 0.5);

		assert!(portrait.distance_from_focus > landscape.distance_from_focus);
	}
}
