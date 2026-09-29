use std::f32::consts::FRAC_PI_2;

use glam::Vec2;

use super::support::{assert_consistent, node_at};
use crate::{WallDimensions, WallError, WallGraph};

#[test]
fn walls_have_default_or_explicit_dimensions() {
	let mut graph = WallGraph::new();
	let default = graph.add_wall(Vec2::ZERO, Vec2::X).unwrap()[0];
	assert_eq!(
		graph.wall_dimensions(default),
		Some(WallDimensions::default())
	);
	assert_eq!(WallDimensions::default().thickness, 0.20);
	assert_eq!(WallDimensions::default().height, 2.50);

	let custom = WallDimensions {
		thickness: 0.30,
		height: 3.00,
	};
	let arc = graph
		.add_arc_with_dimensions(Vec2::new(3.0, 0.0), Vec2::new(2.0, 1.0), FRAC_PI_2, custom)
		.unwrap()[0];
	assert_eq!(graph.wall_dimensions(arc), Some(custom));
	assert_consistent(&graph);
}

#[test]
fn split_pieces_inherit_the_original_dimensions() {
	for curved in [false, true] {
		let mut graph = WallGraph::new();
		let dimensions = WallDimensions {
			thickness: 0.31,
			height: 2.80,
		};
		let midpoint = if curved { Vec2::Y } else { Vec2::X };
		let end = if curved {
			-Vec2::X
		} else {
			Vec2::new(2.0, 0.0)
		};
		let original = if curved {
			graph
				.add_arc_with_dimensions(Vec2::X, end, std::f32::consts::PI, dimensions)
				.unwrap()[0]
		} else {
			graph
				.add_wall_with_dimensions(Vec2::ZERO, end, dimensions)
				.unwrap()[0]
		};
		graph.add_wall(midpoint, midpoint + Vec2::Y).unwrap();
		let split = node_at(&graph, midpoint);
		assert!(graph.walls().any(|wall| {
			(wall.origin() == split || wall.destination() == split)
				&& graph.wall_dimensions(wall) == Some(WallDimensions::default())
		}));
		assert_eq!(graph.wall_dimensions(original), None);
		assert_eq!(
			graph
				.walls()
				.filter(|wall| graph.wall_dimensions(*wall) == Some(dimensions))
				.count(),
			2
		);
		assert_consistent(&graph);
	}
}

#[test]
fn only_walls_with_equal_dimensions_merge() {
	let mut graph = WallGraph::new();
	let thick = WallDimensions {
		thickness: 0.30,
		height: 2.50,
	};
	let first = graph
		.add_wall_with_dimensions(Vec2::ZERO, Vec2::X, thick)
		.unwrap()[0];
	let second = graph.add_wall(Vec2::X, Vec2::new(2.0, 0.0)).unwrap()[0];
	assert_eq!(graph.walls().count(), 2);
	assert_eq!(graph.wall_dimensions(first), Some(thick));
	assert_eq!(
		graph.wall_dimensions(second),
		Some(WallDimensions::default())
	);

	let third = graph
		.add_wall_with_dimensions(Vec2::new(-1.0, 0.0), Vec2::ZERO, thick)
		.unwrap()[0];
	assert_eq!(graph.walls().count(), 2);
	assert_eq!(graph.wall_length(third), Some(2.0));
	assert_eq!(graph.wall_dimensions(third), Some(thick));
	assert_consistent(&graph);
}

#[test]
fn height_differences_prevent_arc_merging() {
	let mut graph = WallGraph::new();
	let tall = WallDimensions {
		thickness: 0.20,
		height: 3.00,
	};
	let first = graph
		.add_arc_with_dimensions(Vec2::X, Vec2::Y, FRAC_PI_2, tall)
		.unwrap()[0];
	let second = graph.add_arc(Vec2::Y, -Vec2::X, FRAC_PI_2).unwrap()[0];
	assert_eq!(graph.walls().count(), 2);
	assert_eq!(graph.wall_dimensions(first), Some(tall));
	assert_eq!(
		graph.wall_dimensions(second),
		Some(WallDimensions::default())
	);
	assert_consistent(&graph);
}

#[test]
fn invalid_dimensions_and_stale_handles_leave_state_unchanged() {
	let mut graph = WallGraph::new();
	let wall = graph.add_wall(Vec2::ZERO, Vec2::X).unwrap()[0];
	let next_node_id = graph.next_node_id;
	let next_edge_id = graph.next_edge_id;
	for invalid in [
		WallDimensions {
			thickness: 0.0,
			height: 2.5,
		},
		WallDimensions {
			thickness: 0.2,
			height: -1.0,
		},
		WallDimensions {
			thickness: f32::NAN,
			height: 2.5,
		},
		WallDimensions {
			thickness: 0.2,
			height: f32::INFINITY,
		},
	] {
		assert_eq!(
			graph.set_wall_dimensions(wall, invalid),
			Err(WallError::InvalidDimensions)
		);
		assert_eq!(
			graph.add_wall_with_dimensions(Vec2::new(2.0, 0.0), Vec2::new(3.0, 0.0), invalid),
			Err(WallError::InvalidDimensions)
		);
		assert_eq!(graph.wall_dimensions(wall), Some(WallDimensions::default()));
		assert_eq!(graph.walls().count(), 1);
		assert_eq!(graph.nodes().count(), 2);
		assert_eq!(graph.next_node_id, next_node_id);
		assert_eq!(graph.next_edge_id, next_edge_id);
	}
	graph.remove_wall(wall).unwrap();
	assert_eq!(graph.wall_dimensions(wall), None);
	assert_eq!(
		graph.set_wall_dimensions(wall, WallDimensions::default()),
		Err(WallError::UnknownWall)
	);
	assert_consistent(&graph);
}

#[test]
fn setting_dimensions_and_optimizing_preserve_them() {
	let mut graph = WallGraph::new();
	let removed = graph.add_wall(Vec2::ZERO, Vec2::X).unwrap()[0];
	let wall = graph
		.add_wall(Vec2::new(3.0, 0.0), Vec2::new(4.0, 0.0))
		.unwrap()[0];
	let dimensions = WallDimensions {
		thickness: 0.24,
		height: 2.70,
	};
	graph.set_wall_dimensions(wall, dimensions).unwrap();
	graph.remove_wall(removed).unwrap();
	let ids = graph.optimize();
	assert_eq!(
		graph.wall_dimensions(ids.wall(wall).unwrap()),
		Some(dimensions)
	);
	assert_eq!(ids.wall(removed), None);
	assert_consistent(&graph);
}
