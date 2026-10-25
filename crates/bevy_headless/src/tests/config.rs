use super::*;

#[test]
fn physical_size_applies_the_scale_factor() {
	let config = HeadlessConfig::default()
		.with_size(800, 600)
		.with_scale_factor(1.5);

	assert_eq!(config.physical_size(), (1200, 900));
}

#[test]
fn physical_size_is_never_empty() {
	let config = HeadlessConfig::default()
		.with_size(0, 0)
		.with_scale_factor(0.5);

	assert_eq!(config.physical_size(), (1, 1));
}
