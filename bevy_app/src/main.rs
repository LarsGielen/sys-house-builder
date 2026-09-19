use bevy::{DefaultPlugins, app::{App, PluginGroup, Startup}, camera::Camera2d, color::Color, ecs::system::Commands, math::Vec2, sprite::Sprite, transform::components::Transform, utils::default, window::{Window, WindowPlugin}};

#[derive(Debug, Clone, Copy)]
struct Corner {
	x: f32,
	y: f32,
}

fn setup(mut commands: Commands) {
	let list = vec![Corner{x: 0.0, y: 0.0}, Corner{x: 100.0, y: 100.0}];

	commands.spawn(Camera2d);

	for corner in &list {
		commands.spawn((
			Sprite::from_color(Color::srgb(1.0, 1.0, 1.0), Vec2::new(20.0, 20.0)),
			Transform::from_xyz(corner.x, corner.y, 0.0),
		));
	}
}

fn main() {
	App::new()
		.add_plugins(DefaultPlugins.set(WindowPlugin {
			    primary_window: Some(Window { name: Some("bevy-dev".into()), ..default() }),
			    ..default()
		}))
		.add_systems(Startup, setup)
		.run();
}
