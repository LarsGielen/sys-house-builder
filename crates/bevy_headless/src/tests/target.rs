use bevy::camera::ImageRenderTarget;

use super::*;

fn app() -> App {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.insert_resource(HeadlessTarget {
			image: Handle::default(),
			scale_factor: 2.0,
		})
		.add_plugins(HeadlessTargetPlugin);
	app
}

fn is_headless_image(target: &RenderTarget) -> bool {
	matches!(target, RenderTarget::Image(ImageRenderTarget { scale_factor, .. }) if *scale_factor == 2.0)
}

#[test]
fn window_camera_is_redirected_to_the_image() {
	let mut app = app();
	let camera = app.world_mut().spawn(Camera::default()).id();

	app.update();

	assert!(is_headless_image(
		app.world().get::<RenderTarget>(camera).unwrap()
	));
}

#[test]
fn camera_with_its_own_target_is_untouched() {
	let mut app = app();
	let own_target = RenderTarget::None {
		size: UVec2::new(8, 8),
	};
	let camera = app.world_mut().spawn((Camera::default(), own_target)).id();

	app.update();

	assert!(matches!(
		app.world().get::<RenderTarget>(camera),
		Some(RenderTarget::None { size }) if *size == UVec2::new(8, 8)
	));
	assert!(app.world().get::<IsDefaultUiCamera>(camera).is_none());
}

#[test]
fn camera_spawned_later_is_redirected_too() {
	let mut app = app();
	app.update();
	let camera = app.world_mut().spawn(Camera::default()).id();

	app.update();

	assert!(is_headless_image(
		app.world().get::<RenderTarget>(camera).unwrap()
	));
}

#[test]
fn ui_defaults_to_the_highest_order_redirected_camera() {
	let mut app = app();
	let _background = app.world_mut().spawn(Camera::default()).id();
	let overlay = app
		.world_mut()
		.spawn(Camera {
			order: 1,
			..default()
		})
		.id();

	app.update();

	assert!(app.world().get::<IsDefaultUiCamera>(overlay).is_some());
}

#[test]
fn an_explicit_ui_camera_is_kept() {
	let mut app = app();
	let chosen = app
		.world_mut()
		.spawn((Camera::default(), IsDefaultUiCamera))
		.id();
	let other = app
		.world_mut()
		.spawn(Camera {
			order: 1,
			..default()
		})
		.id();

	app.update();

	assert!(app.world().get::<IsDefaultUiCamera>(chosen).is_some());
	assert!(app.world().get::<IsDefaultUiCamera>(other).is_none());
}
