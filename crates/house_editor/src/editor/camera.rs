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
	focus: Vec3,
	yaw: f32,
	pitch: f32,
	distance: f32,
}

impl Default for OrbitCamera {
	fn default() -> Self {
		Self {
			focus: Vec3::new(0.0, 0.5, 0.0),
			yaw: 0.6,
			pitch: 0.6,
			distance: 15.0,
		}
	}
}

impl OrbitCamera {
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

	pub fn pan(&mut self, right_amount: f32, forward_amount: f32) {
		let yaw_rotation = Quat::from_rotation_y(self.yaw);
		let right = yaw_rotation * Vec3::X;
		let forward = yaw_rotation * Vec3::NEG_Z;
		self.focus += (right * right_amount + forward * forward_amount) * self.distance;
	}

	fn transform(&self) -> Transform {
		let rotation = Quat::from_euler(EulerRot::YXZ, self.yaw, -self.pitch, 0.0);
		Transform::from_translation(self.focus + rotation * Vec3::Z * self.distance)
			.looking_at(self.focus, Vec3::Y)
	}
}

fn update_camera_transform(
	mut orbit_cameras: Query<(&mut Transform, &OrbitCamera), Changed<OrbitCamera>>,
) {
	for (mut transform, orbit_camera) in &mut orbit_cameras {
		*transform = orbit_camera.transform();
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn camera_facing_negative_z(distance: f32) -> OrbitCamera {
		OrbitCamera {
			yaw: 0.0,
			distance,
			..default()
		}
	}

	#[test]
	fn pans_along_the_view_direction_on_the_ground_plane() {
		let mut camera = camera_facing_negative_z(10.0);
		let start = camera.focus;

		camera.pan(0.1, 0.2);

		assert!((camera.focus - start - Vec3::new(1.0, 0.0, -2.0)).length() < 1e-5);
	}

	#[test]
	fn pan_follows_the_yaw() {
		let mut camera = camera_facing_negative_z(10.0);
		camera.yaw = std::f32::consts::FRAC_PI_2;
		let start = camera.focus;

		// A quarter turn makes "forward" point along negative X.
		camera.pan(0.0, 0.1);

		assert!((camera.focus - start - Vec3::new(-1.0, 0.0, 0.0)).length() < 1e-5);
	}

	#[test]
	fn pan_distance_scales_with_zoom() {
		let mut near = camera_facing_negative_z(5.0);
		let mut far = camera_facing_negative_z(20.0);
		let start = near.focus;

		near.pan(0.1, 0.0);
		far.pan(0.1, 0.0);

		assert!(((far.focus - start).length() / (near.focus - start).length() - 4.0).abs() < 1e-5);
	}
}
