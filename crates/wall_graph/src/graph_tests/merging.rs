use std::f32::consts::{FRAC_PI_2, PI};

use glam::Vec2;

use super::support::{assert_consistent, node_at};
use crate::WallGraph;

#[test]
fn extending_a_straight_wall_merges_in_the_requested_direction() {
	for reverse in [false, true] {
		let mut graph = WallGraph::new();
		let original = graph.add_wall(Vec2::ZERO, Vec2::X).unwrap()[0];
		let (from, to) = if reverse {
			(Vec2::new(2.0, 0.0), Vec2::X)
		} else {
			(Vec2::X, Vec2::new(2.0, 0.0))
		};
		let merged = graph.add_wall(from, to).unwrap();

		assert_eq!(merged.len(), 1);
		assert_eq!(graph.walls().count(), 1);
		assert_eq!(graph.nodes().count(), 2);
		assert_eq!(
			graph.wall_position(merged[0], 0.0),
			Some(if reverse { from } else { Vec2::ZERO })
		);
		assert_eq!(
			graph.wall_position(merged[0], 1.0),
			Some(if reverse { Vec2::ZERO } else { to })
		);
		assert_eq!(graph.wall_length(original), None);
		assert_eq!(graph.node_position(original.destination()), None);
		assert_consistent(&graph);
	}
}

#[test]
fn a_wall_bridging_two_collinear_walls_merges_all_three() {
	let mut graph = WallGraph::new();
	let first = graph.add_wall(Vec2::ZERO, Vec2::X).unwrap()[0];
	let last = graph
		.add_wall(Vec2::new(2.0, 0.0), Vec2::new(3.0, 0.0))
		.unwrap()[0];
	let merged = graph.add_wall(Vec2::X, Vec2::new(2.0, 0.0)).unwrap();

	assert_eq!(merged.len(), 1);
	assert_eq!(graph.walls().count(), 1);
	assert_eq!(graph.nodes().count(), 2);
	assert_eq!(graph.wall_length(merged[0]), Some(3.0));
	assert_eq!(graph.wall_length(first), None);
	assert_eq!(graph.wall_length(last), None);
	assert_consistent(&graph);
}

#[test]
fn branches_and_bends_remain_junctions() {
	let mut graph = WallGraph::new();
	graph.add_wall(Vec2::ZERO, Vec2::X).unwrap();
	graph.add_wall(Vec2::X, Vec2::new(2.0, 0.0)).unwrap();
	graph.add_wall(Vec2::X, Vec2::new(1.0, 1.0)).unwrap();
	let extended = graph
		.add_wall(Vec2::new(2.0, 0.0), Vec2::new(3.0, 0.0))
		.unwrap();

	assert_eq!(extended.len(), 1);
	assert_eq!(graph.walls().count(), 3);
	assert_eq!(graph.nodes().count(), 4);
	assert_eq!(graph.wall_length(extended[0]), Some(2.0));
	assert_eq!(graph.node_position(node_at(&graph, Vec2::X)), Some(Vec2::X));
	assert_consistent(&graph);

	let mut bend = WallGraph::new();
	bend.add_wall(Vec2::ZERO, Vec2::X).unwrap();
	bend.add_wall(Vec2::X, Vec2::new(2.0, 1.0)).unwrap();
	assert_eq!(bend.walls().count(), 2);
	assert_consistent(&bend);
}

#[test]
fn compatible_arcs_merge_but_a_full_circle_does_not() {
	let mut graph = WallGraph::new();
	let first = graph.add_arc(Vec2::X, Vec2::Y, FRAC_PI_2).unwrap()[0];
	let merged = graph.add_arc(Vec2::Y, -Vec2::X, FRAC_PI_2).unwrap();
	assert_eq!(merged.len(), 1);
	assert_eq!(graph.walls().count(), 1);
	assert_eq!(graph.wall_length(first), None);
	assert!(
		graph
			.wall_position(merged[0], 0.5)
			.unwrap()
			.distance(Vec2::Y)
			< 1e-5
	);
	assert_consistent(&graph);

	graph.add_arc(-Vec2::X, Vec2::X, PI).unwrap();
	assert_eq!(graph.walls().count(), 2);
	assert_eq!(graph.nodes().count(), 2);
	assert_consistent(&graph);
}

#[test]
fn arcs_with_different_curvature_and_mixed_shapes_do_not_merge() {
	let mut graph = WallGraph::new();
	graph.add_arc(Vec2::X, Vec2::Y, FRAC_PI_2).unwrap();
	graph
		.add_arc(Vec2::Y, Vec2::new(-2.0, -1.0), FRAC_PI_2)
		.unwrap();
	assert_eq!(graph.walls().count(), 2);
	assert_consistent(&graph);

	let mut mixed = WallGraph::new();
	mixed.add_wall(Vec2::ZERO, Vec2::X).unwrap();
	mixed
		.add_arc(Vec2::X, Vec2::new(2.0, 1.0), FRAC_PI_2)
		.unwrap();
	assert_eq!(mixed.walls().count(), 2);
	assert_consistent(&mixed);
}
