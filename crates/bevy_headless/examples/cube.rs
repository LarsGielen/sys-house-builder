//! Renders a lit cube with a UI label to a PNG without a window:
//! `cargo run -p bevy_headless --example cube -- --out cube.png [--software]`

use bevy::prelude::*;
use bevy_headless::{CaptureArgs, HeadlessApp};

fn main() {
	let args = CaptureArgs::from_env();
	let mut app = HeadlessApp::new(args.config, |app| {
		app.add_systems(Startup, spawn_scene);
	});
	app.settle();
	app.screenshot(&args.out).expect("screenshot");
}

// Spawned exactly as a windowed app would: the cameras name no render target.
fn spawn_scene(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	commands.spawn((
		Mesh3d(meshes.add(Cuboid::new(2.0, 2.0, 2.0))),
		MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
		Transform::from_xyz(0.0, 1.0, 0.0),
	));
	commands.spawn((
		PointLight {
			shadow_maps_enabled: true,
			..default()
		},
		Transform::from_xyz(4.0, 8.0, 4.0),
	));
	commands.spawn((
		Camera3d::default(),
		Transform::from_xyz(5.0, 4.0, 6.0).looking_at(Vec3::Y, Vec3::Y),
	));
	commands.spawn((
		Node {
			position_type: PositionType::Absolute,
			left: Val::Px(16.0),
			top: Val::Px(16.0),
			padding: UiRect::all(Val::Px(8.0)),
			..default()
		},
		BackgroundColor(Color::srgb(0.9, 0.2, 0.2)),
		children![Text::new("headless ui")],
	));
}
