use bevy::prelude::*;
use wall_graph::{OpeningSpec, Wall, WallGraph};

const NODE_SIZE: f32 = 14.0;
const METRES_TO_PIXELS: f32 = 50.0;
const OPENING_MARKER_THICKNESS: f32 = 3.0;

fn build_demo_graph() -> WallGraph {
	let mut graph = WallGraph::new();

	let bottom_left = Vec2::new(-3.0, -2.0);
	let bottom_right = Vec2::new(3.0, -2.0);
	let top_right = Vec2::new(3.0, 2.0);
	let top_left = Vec2::new(-3.0, 2.0);
	let arc_end = Vec2::new(5.0, 0.0);

	let bottom = graph
		.add_wall(bottom_left, bottom_right)
		.expect("demo wall")[0];
	graph.add_wall(bottom_right, top_right).expect("demo wall");
	let top = graph.add_wall(top_right, top_left).expect("demo wall")[0];
	graph.add_wall(top_left, bottom_left).expect("demo wall");
	graph
		.add_arc(top_right, arc_end, -std::f32::consts::FRAC_PI_2)
		.expect("demo arc");
	graph
		.add_wall(Vec2::new(4.0, -0.6), Vec2::new(4.0, 2.6))
		.expect("demo crossing");

	graph
		.add_opening(
			bottom,
			OpeningSpec {
				center_distance: 3.0,
				width: 1.0,
				bottom: 0.0,
				height: 2.1,
			},
		)
		.expect("demo door opening");
	graph
		.add_opening(
			top,
			OpeningSpec {
				center_distance: 3.0,
				width: 1.2,
				bottom: 0.9,
				height: 1.2,
			},
		)
		.expect("demo window opening");
	let curved_wall = graph
		.walls()
		.find(|&wall| {
			(graph.wall_position(wall, 0.0) == Some(arc_end)
				|| graph.wall_position(wall, 1.0) == Some(arc_end))
				&& graph
					.sample_wall(wall, 0.01)
					.expect("valid demo wall")
					.len() > 2
		})
		.expect("arc piece ending at demo endpoint");
	graph
		.add_opening(
			curved_wall,
			OpeningSpec {
				center_distance: graph.wall_length(curved_wall).unwrap() * 0.5,
				width: 0.5,
				bottom: 0.8,
				height: 1.0,
			},
		)
		.expect("demo curved opening");

	graph
}

fn draw_span(
	commands: &mut Commands,
	graph: &WallGraph,
	wall: Wall,
	start: f64,
	end: f64,
	color: Color,
	thickness: f32,
) {
	let length = graph.wall_length(wall).expect("current demo wall");
	let curved = graph
		.sample_wall(wall, 0.01)
		.expect("valid demo geometry")
		.len() > 2;
	let segments = if curved {
		((end - start) / 0.1).ceil() as usize
	} else {
		1
	}
	.max(1);
	for index in 0..segments {
		let from_distance = start + (end - start) * index as f64 / segments as f64;
		let to_distance = start + (end - start) * (index + 1) as f64 / segments as f64;
		let from = graph
			.wall_position(wall, (from_distance / length) as f32)
			.unwrap() * METRES_TO_PIXELS;
		let to = graph
			.wall_position(wall, (to_distance / length) as f32)
			.unwrap() * METRES_TO_PIXELS;
		let direction = to - from;
		commands.spawn((
			Sprite::from_color(color, Vec2::new(direction.length(), thickness)),
			Transform::from_translation(((from + to) / 2.0).extend(0.0))
				.with_rotation(Quat::from_rotation_z(direction.to_angle())),
		));
	}
}

fn setup(mut commands: Commands) {
	let graph = build_demo_graph();

	commands.spawn(Camera2d);

	for wall in graph.walls() {
		let length = graph.wall_length(wall).expect("current demo wall");
		let openings: Vec<_> = graph
			.openings()
			.filter(|(_, opening)| opening.wall == wall)
			.map(|(_, opening)| opening.spec)
			.collect();
		let mut boundaries = vec![0.0, length];
		for opening in &openings {
			boundaries.push(opening.center_distance - opening.width as f64 * 0.5);
			boundaries.push(opening.center_distance + opening.width as f64 * 0.5);
		}
		boundaries.sort_by(f64::total_cmp);
		boundaries.dedup();
		for pair in boundaries.windows(2) {
			let middle = (pair[0] + pair[1]) * 0.5;
			if openings.iter().any(|opening| {
				(opening.center_distance - opening.width as f64 * 0.5
					..opening.center_distance + opening.width as f64 * 0.5)
					.contains(&middle)
			}) {
				continue;
			}
			draw_span(
				&mut commands,
				&graph,
				wall,
				pair[0],
				pair[1],
				Color::srgb(0.8, 0.8, 0.8),
				graph.wall_dimensions(wall).unwrap().thickness * METRES_TO_PIXELS,
			);
		}
		for opening in openings {
			draw_span(
				&mut commands,
				&graph,
				wall,
				opening.center_distance - opening.width as f64 * 0.5,
				opening.center_distance + opening.width as f64 * 0.5,
				Color::srgb(0.2, 0.8, 0.9),
				OPENING_MARKER_THICKNESS,
			);
		}
	}

	for (_, position) in graph.nodes() {
		commands.spawn((
			Sprite::from_color(Color::srgb(1.0, 0.4, 0.2), Vec2::splat(NODE_SIZE)),
			Transform::from_translation((position * METRES_TO_PIXELS).extend(1.0)),
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
	fn demo_contains_curved_geometry_a_crossing_and_openings() {
		let graph = super::build_demo_graph();
		assert_eq!(graph.nodes().count(), 8);
		assert_eq!(graph.walls().count(), 8);
		assert_eq!(graph.openings().count(), 3);
		assert!(
			graph
				.openings()
				.any(|(_, opening)| graph.sample_wall(opening.wall, 0.01).unwrap().len() > 2)
		);
		assert!(
			graph
				.walls()
				.any(|wall| graph.sample_wall(wall, 0.01).unwrap().len() > 2)
		);
	}
}
