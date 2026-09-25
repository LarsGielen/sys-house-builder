use bevy::prelude::*;
use wall_graph::WallGraph;

const NODE_SIZE: f32 = 14.0;
const WALL_THICKNESS: f32 = 4.0;

fn build_demo_graph() -> WallGraph {
	let mut graph = WallGraph::new();

	let bottom_left = graph.add_node(Vec2::new(-150.0, -100.0));
	let bottom_right = graph.add_node(Vec2::new(150.0, -100.0));
	let top_right = graph.add_node(Vec2::new(150.0, 100.0));
	let top_left = graph.add_node(Vec2::new(-150.0, 100.0));
	let door = graph.add_node(Vec2::new(250.0, 0.0));

	graph
		.add_wall(bottom_left, bottom_right)
		.expect("demo wall");
	graph.add_wall(bottom_right, top_right).expect("demo wall");
	graph.add_wall(top_right, top_left).expect("demo wall");
	graph.add_wall(top_left, bottom_left).expect("demo wall");
	graph.add_wall(top_right, door).expect("demo wall");

	graph
}

fn setup(mut commands: Commands) {
	let graph = build_demo_graph();

	commands.spawn(Camera2d);

	for wall in graph.walls() {
		let (Some(from), Some(to)) = (
			graph.node_position(wall.origin),
			graph.node_position(wall.destination),
		) else {
			continue;
		};

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
