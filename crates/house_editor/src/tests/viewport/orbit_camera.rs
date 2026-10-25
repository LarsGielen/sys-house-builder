use super::*;
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use std::f32::consts::FRAC_PI_2;

#[test]
fn framing_moves_back_for_a_narrow_viewport() {
	let mut landscape = OrbitCamera::default();
	landscape.frame(&CameraSettings::default(), Vec3::ZERO, 5.0, FRAC_PI_2, 2.0);

	let mut portrait = OrbitCamera::default();
	portrait.frame(&CameraSettings::default(), Vec3::ZERO, 5.0, FRAC_PI_2, 0.5);

	assert!(portrait.distance_from_focus > landscape.distance_from_focus);
}
#[test]
fn plugin_updates_the_camera_without_editor_plugins() {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, OrbitCameraPlugin))
		.init_resource::<ButtonInput<MouseButton>>()
		.init_resource::<ButtonInput<KeyCode>>()
		.init_resource::<AccumulatedMouseMotion>()
		.init_resource::<AccumulatedMouseScroll>();
	app.world_mut().spawn((
		Window {
			focused: true,
			..default()
		},
		PrimaryWindow,
	));
	let camera = app
		.world_mut()
		.spawn((Camera3d::default(), OrbitCamera::default()))
		.id();
	app.update();
	let original_distance = app
		.world()
		.get::<OrbitCamera>(camera)
		.unwrap()
		.distance_from_focus;
	assert!(
		(app.world()
			.get::<Transform>(camera)
			.unwrap()
			.translation
			.length() - original_distance)
			.abs() < 1e-5
	);

	let panel = app.world_mut().spawn(Interaction::Hovered).id();
	app.world_mut()
		.resource_mut::<AccumulatedMouseScroll>()
		.delta
		.y = 1.0;
	app.update();
	assert_eq!(
		app.world()
			.get::<OrbitCamera>(camera)
			.unwrap()
			.distance_from_focus,
		original_distance
	);

	app.world_mut().entity_mut(panel).insert(Interaction::None);
	app.update();
	assert!(
		app.world()
			.get::<OrbitCamera>(camera)
			.unwrap()
			.distance_from_focus
			< original_distance
	);
}
