use std::f32::consts::FRAC_PI_2;

use bevy::{
	input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit},
	log::{DEFAULT_FILTER, LogPlugin},
	prelude::*,
	window::PrimaryWindow,
};
use wall_graph::WallGraph;

fn main() {
	// Reset runs first so this frame's orbit/pan/zoom input applies on top of the reset view.
	let camera_system = (
		request_camera_reset_from_keyboard,
		handle_reset_camera,
		apply_editor_camera_input_orbit,
		apply_editor_camera_input_pan,
		apply_editor_camera_input_keyboard_pan,
		apply_editor_camera_input_zoom,
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
		.add_message::<ResetCamera>()
		.add_systems(Startup, setup_scene)
		.add_systems(Startup, temp_probe)
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
	fn frame(&mut self, center: Vec3, radius: f32, vertical_fov: f32) {
		self.target_point = center;
		// A window wider than tall is limited by the vertical FOV, so fitting that one fits both.
		self.distance_from_focus = clamp_camera_distance(radius / (vertical_fov / 2.0).sin());
	}

	/// Returns to the default viewing angle, framing the origin.
	fn reset(&mut self, vertical_fov: f32) {
		let defaults = Self::default();
		self.azimuth = defaults.azimuth;
		self.elevation = defaults.elevation;
		self.frame(Vec3::ZERO, RESET_VIEW_RADIUS, vertical_fov);
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
	editor_camera.reset(perspective.fov);
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

const ORBIT_RADIANS_PER_PIXEL: f32 = 0.005;
const CAMERA_ELEVATION_MIN: f32 = 0.3;
const CAMERA_ELEVATION_MAX: f32 = FRAC_PI_2 - 0.01;

fn apply_editor_camera_input_orbit(
	mut camera: Single<&mut EditorCamera>,
	mouse_button: Res<ButtonInput<MouseButton>>,
	mouse_motion: Res<AccumulatedMouseMotion>,
) {
	if !mouse_button.pressed(MouseButton::Right) {
		return;
	}

	camera.azimuth -= mouse_motion.delta.x * ORBIT_RADIANS_PER_PIXEL;
	camera.elevation += mouse_motion.delta.y * ORBIT_RADIANS_PER_PIXEL;
	camera.elevation = camera
		.elevation
		.clamp(CAMERA_ELEVATION_MIN, CAMERA_ELEVATION_MAX);
}

fn cursor_ground_point(
	window: &Window,
	camera: &Camera,
	camera_transform: &GlobalTransform,
) -> Option<Vec3> {
	let cursor_position = window.cursor_position()?;
	let ray = camera
		.viewport_to_world(camera_transform, cursor_position)
		.ok()?;
	ray.plane_intersection_point(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))
}

fn apply_editor_camera_input_pan(
	camera: Single<(&mut EditorCamera, &Camera, &GlobalTransform)>,
	window: Single<&Window, With<PrimaryWindow>>,
	mouse_button: Res<ButtonInput<MouseButton>>,
	mut grabbed_ground_point: Local<Option<Vec3>>,
) {
	if !mouse_button.pressed(MouseButton::Middle) {
		*grabbed_ground_point = None;
		return;
	}

	let (mut editor_camera, camera, camera_transform) = camera.into_inner();
	let Some(ground_point) = cursor_ground_point(*window, camera, camera_transform) else {
		return;
	};

	match *grabbed_ground_point {
		None => *grabbed_ground_point = Some(ground_point),
		Some(grabbed_point) => editor_camera.target_point += grabbed_point - ground_point,
	}
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

// Wrapper for the Wallgraph system, needed since wall_graph is seperate from Bevy
#[derive(Resource, Default)]
struct House(WallGraph);

fn temp_probe(house: Res<House>) {
	println!("The house has {} walls", house.0.walls().count());
}
