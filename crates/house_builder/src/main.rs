use bevy::prelude::*;
use wall_graph::WallGraph;

const NODE_SIZE: f32 = 14.0;
const WALL_THICKNESS: f32 = 4.0;

fn build_demo_graph() -> WallGraph {
	let mut graph = WallGraph::new();

	let bottom_left = graph.add_node(Vec2::new(-150.0, -100.0)).unwrap();
	let bottom_right = graph.add_node(Vec2::new(150.0, -100.0)).unwrap();
	let top_right = graph.add_node(Vec2::new(150.0, 100.0)).unwrap();
	let top_left = graph.add_node(Vec2::new(-150.0, 100.0)).unwrap();
	let door = graph.add_node(Vec2::new(250.0, 0.0)).unwrap();

	graph
		.add_wall(bottom_left, bottom_right)
		.expect("demo wall");
	graph.add_wall(bottom_right, top_right).expect("demo wall");
	graph.add_wall(top_right, top_left).expect("demo wall");
	graph.add_wall(top_left, bottom_left).expect("demo wall");
	graph
		.add_arc(top_right, door, -std::f32::consts::FRAC_PI_2)
		.expect("demo arc");
	let crossing_bottom = graph.add_node(Vec2::new(200.0, -30.0)).unwrap();
	let crossing_top = graph.add_node(Vec2::new(200.0, 130.0)).unwrap();
	graph
		.add_wall(crossing_bottom, crossing_top)
		.expect("demo crossing");

	graph
}

fn setup(mut commands: Commands) {
	let graph = build_demo_graph();

	commands.spawn(Camera2d);

	for wall in graph.walls() {
		let points = graph.sample_wall(wall, 0.5).expect("valid demo geometry");
		for pair in points.windows(2) {
			let (from, to) = (pair[0], pair[1]);
			let direction = to - from;
			commands.spawn((
				Sprite::from_color(
					Color::srgb(0.8, 0.8, 0.8),
					Vec2::new(direction.length(), WALL_THICKNESS),
				),
				Transform::from_translation(((from + to) / 2.0).extend(0.0))
					.with_rotation(Quat::from_rotation_z(direction.to_angle())),
			));
		}
	}

	for (_, position) in graph.nodes() {
		commands.spawn((
			Sprite::from_color(Color::srgb(1.0, 0.4, 0.2), Vec2::splat(NODE_SIZE)),
			Transform::from_translation(position.extend(1.0)),
		));
	}
}

fn main() {
	App::new()
		.add_plugins(DefaultPlugins.set(WindowPlugin {
			primary_window: Some(Window {
				name: Some("bevy-dev".into()),
				..default()
			}),
			..default()
		}))
		.add_systems(Startup, setup)
		.run();
}

#[cfg(test)]
mod tests {
	#[test]
	fn demo_contains_curved_geometry_and_a_crossing() {
		let graph = super::build_demo_graph();
		assert_eq!(graph.nodes().count(), 8);
		assert_eq!(graph.walls().count(), 8);
		assert!(
			graph
				.walls()
				.any(|wall| graph.sample_wall(wall, 0.5).unwrap().len() > 2)
		);
	}
}
