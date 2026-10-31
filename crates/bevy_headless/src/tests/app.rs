use super::*;

const SIZE: u32 = 64;

fn solid_color_app() -> Option<HeadlessApp> {
	// Rendering needs a graphics adapter; machines without one cannot run this test.
	std::panic::catch_unwind(|| {
		HeadlessApp::new(
			HeadlessConfig::default()
				.with_size(SIZE, SIZE)
				.with_software_rendering(true),
			|app| {
				app.insert_resource(ClearColor(Color::srgb_u8(200, 100, 50)))
					.add_systems(Startup, |mut commands: Commands| {
						commands.spawn(Camera2d);
					});
			},
		)
	})
	.ok()
}

#[test]
fn capture_has_the_configured_size_and_clear_color() {
	let Some(mut app) = solid_color_app() else {
		return;
	};
	app.settle();

	let image = app.capture().unwrap();

	assert_eq!((image.width(), image.height()), (SIZE, SIZE));
	let pixel = image.get_color_at(SIZE / 2, SIZE / 2).unwrap().to_srgba();
	assert_eq!(
		[pixel.red, pixel.green, pixel.blue].map(|channel| (channel * 255.0).round() as u8),
		[200, 100, 50]
	);
}

#[test]
fn screenshot_writes_the_file() {
	let Some(mut app) = solid_color_app() else {
		return;
	};
	app.settle();
	let path = std::env::temp_dir().join("bevy_headless_screenshot_test.png");

	app.screenshot(&path).unwrap();

	assert!(path.metadata().unwrap().len() > 0);
	std::fs::remove_file(path).unwrap();
}

#[test]
fn run_until_reports_a_timeout() {
	let Some(mut app) = solid_color_app() else {
		return;
	};

	let result = app.run_until(|_| false, 3);

	assert!(matches!(result, Err(HeadlessError::Timeout { frames: 3 })));
}
