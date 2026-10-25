use super::*;

#[test]
fn the_defaults_are_valid() {
	let defaults = InteractionSettings::default();
	assert_eq!(
		InteractionSettings::new(
			defaults.drag_threshold_pixels(),
			defaults.snap_acquire_radius_pixels(),
			defaults.snap_release_radius_pixels()
		),
		Ok(defaults)
	);
}

#[test]
fn distances_must_be_positive_and_finite() {
	for bad in [0.0, -1.0, f32::NAN, f32::INFINITY] {
		assert_eq!(
			InteractionSettings::new(bad, 12.0, 20.0),
			Err(InvalidInteractionSetting::NotPositive)
		);
		assert_eq!(
			InteractionSettings::new(6.0, bad, 20.0),
			Err(InvalidInteractionSetting::NotPositive)
		);
		assert_eq!(
			InteractionSettings::new(6.0, 12.0, bad),
			Err(InvalidInteractionSetting::NotPositive)
		);
	}
}

#[test]
fn releasing_must_not_be_easier_than_acquiring() {
	assert_eq!(
		InteractionSettings::new(6.0, 20.0, 12.0),
		Err(InvalidInteractionSetting::ReleaseBelowAcquire)
	);
	assert!(InteractionSettings::new(6.0, 15.0, 15.0).is_ok());
}
