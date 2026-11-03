use bevy::prelude::*;

/// Systems that apply the `OrbitCamera` state to the camera. Whatever changes that state must run before this set.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrbitCameraSystems;

pub struct OrbitCameraPlugin;

impl Plugin for OrbitCameraPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, update_camera_transform.in_set(OrbitCameraSystems));
	}
}

#[derive(Component)]
pub struct OrbitCamera {
	yaw: f32,
	pitch: f32,
	distance: f32,
}

impl Default for OrbitCamera {
	fn default() -> Self {
		Self {
			yaw: 0.6,
			pitch: 0.6,
			distance: 15.0,
		}
	}
}

impl OrbitCamera {
	const FOCUS: Vec3 = Vec3::new(0.0, 0.5, 0.0);
	const MIN_PITCH: f32 = 0.1;
	const MAX_PITCH: f32 = 1.5;
	const MIN_DISTANCE: f32 = 4.0;
	const MAX_DISTANCE: f32 = 40.0;

	pub fn orbit(&mut self, yaw_delta: f32, pitch_delta: f32) {
		self.yaw += yaw_delta;
		self.pitch = (self.pitch + pitch_delta).clamp(Self::MIN_PITCH, Self::MAX_PITCH);
	}

	pub fn zoom(&mut self, factor: f32) {
		self.distance = (self.distance * factor).clamp(Self::MIN_DISTANCE, Self::MAX_DISTANCE);
	}

	fn transform(&self) -> Transform {
		let rotation = Quat::from_euler(EulerRot::YXZ, self.yaw, -self.pitch, 0.0);
		Transform::from_translation(Self::FOCUS + rotation * Vec3::Z * self.distance)
			.looking_at(Self::FOCUS, Vec3::Y)
	}
}

fn update_camera_transform(
	mut orbit_cameras: Query<(&mut Transform, &OrbitCamera), Changed<OrbitCamera>>,
) {
	for (mut transform, orbit_camera) in &mut orbit_cameras {
		*transform = orbit_camera.transform();
	}
}
