use super::*;

#[test]
fn defaults_clamp_like_the_original_camera() {
	let settings = CameraSettings::default();
	assert_eq!(settings.clamp_distance(0.1), 0.5);
	assert_eq!(settings.clamp_distance(500.0), 100.0);
	assert_eq!(settings.clamp_elevation(0.0), 0.3);
	assert!(settings.clamp_elevation(10.0) < FRAC_PI_2);
}

#[test]
fn refused_values_leave_the_setting_unchanged() {
	let mut settings = CameraSettings::default();
	let before = settings;
	for bad in [0.0, -1.0, f32::NAN, f32::INFINITY] {
		assert_eq!(
			settings.set_orbit_radians_per_pixel(bad),
			Err(InvalidCameraSetting::NotPositive)
		);
		assert_eq!(
			settings.set_distance_limits(bad, 10.0),
			Err(InvalidCameraSetting::NotPositive)
		);
	}
	assert_eq!(
		settings.set_distance_limits(10.0, 2.0),
		Err(InvalidCameraSetting::EmptyRange)
	);
	assert_eq!(
		settings.set_elevation_limits(0.2, 2.0),
		Err(InvalidCameraSetting::ElevationOutOfRange)
	);
	assert_eq!(
		settings.set_zoom_factor_per_line(1.0),
		Err(InvalidCameraSetting::ZoomFactorTooSmall)
	);
	assert_eq!(settings, before);
}

#[test]
fn accepted_values_take_effect() {
	let mut settings = CameraSettings::default();
	settings.set_distance_limits(1.0, 20.0).unwrap();
	assert_eq!(settings.clamp_distance(50.0), 20.0);
	settings.set_zoom_factor_per_line(1.5).unwrap();
	assert_eq!(settings.zoom_factor_per_line(), 1.5);
}
