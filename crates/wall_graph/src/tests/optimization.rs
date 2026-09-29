use glam::Vec2;

use super::support::{assert_consistent, walk};
use crate::{WallError, WallGraph};

#[test]
fn optimize_compacts_ids_and_maps_surviving_geometry() {
	let mut graph = WallGraph::new();
	let removed = graph.add_wall(Vec2::ZERO, Vec2::X).unwrap()[0];
	let removed_node = removed.origin();
	let arc = graph
		.add_arc(
			Vec2::new(3.0, 0.0),
			Vec2::new(1.0, 0.0),
			std::f32::consts::PI,
		)
		.unwrap()[0];
	let line = graph
		.add_wall(Vec2::new(5.0, 0.0), Vec2::new(7.0, 0.0))
		.unwrap()[0];
	graph.remove_wall(removed).unwrap();

	let nodes: Vec<_> = graph.nodes().collect();
	let walls: Vec<_> = graph
		.walls()
		.map(|wall| {
			(
				wall,
				graph.wall_position(wall, 0.0).unwrap(),
				graph.wall_position(wall, 0.5).unwrap(),
				graph.wall_position(wall, 1.0).unwrap(),
				graph.wall_length(wall).unwrap(),
			)
		})
		.collect();
	let map = graph.optimize();

	assert_eq!(map.node(removed_node), None);
	assert_eq!(map.wall(removed), None);
	assert_eq!(graph.next_node_id, nodes.len());
	assert_eq!(graph.next_edge_id, walls.len() * 2);
	let mut node_ids: Vec<_> = graph.nodes.keys().map(|id| id.0).collect();
	node_ids.sort();
	assert_eq!(node_ids, (0..nodes.len()).collect::<Vec<_>>());
	let mut edge_ids: Vec<_> = graph.edges.keys().map(|id| id.0).collect();
	edge_ids.sort();
	assert_eq!(edge_ids, (0..walls.len() * 2).collect::<Vec<_>>());
	for (old, position) in nodes {
		assert_eq!(graph.node_position(map.node(old).unwrap()), Some(position));
	}
	for (old, start, middle, end, length) in walls {
		let new = map.wall(old).unwrap();
		assert_eq!(graph.wall_position(new, 0.0), Some(start));
		assert_eq!(graph.wall_position(new, 0.5), Some(middle));
		assert_eq!(graph.wall_position(new, 1.0), Some(end));
		assert_eq!(graph.wall_length(new), Some(length));
	}
	assert!(map.wall(arc).is_some());
	assert!(map.wall(line).is_some());
	assert_consistent(&graph);
}

#[test]
fn optimize_resets_counters_after_every_wall_is_removed() {
	let mut graph = WallGraph::new();
	let wall = graph.add_wall(Vec2::ZERO, Vec2::X).unwrap()[0];
	graph.remove_wall(wall).unwrap();
	let map = graph.optimize();
	assert_eq!(map.wall(wall), None);
	assert_eq!(graph.next_node_id, 0);
	assert_eq!(graph.next_edge_id, 0);
	let fresh = graph.add_wall(Vec2::ZERO, Vec2::X).unwrap()[0];
	assert_eq!(fresh.origin().0, 0);
	assert_eq!(fresh.destination().0, 1);
	assert_consistent(&graph);
}

#[test]
fn optimize_preserves_face_boundary_walks() {
	let mut graph = WallGraph::new();
	let a = Vec2::ZERO;
	let b = Vec2::new(2.0, 0.0);
	let c = Vec2::new(1.0, 2.0);
	let side = graph.add_wall(a, b).unwrap()[0];
	graph.add_wall(b, c).unwrap();
	graph.add_wall(c, a).unwrap();
	let discarded = graph
		.add_wall(Vec2::new(4.0, 0.0), Vec2::new(5.0, 0.0))
		.unwrap()[0];
	graph.remove_wall(discarded).unwrap();
	let boundaries = [
		walk(&graph, side.origin(), side.destination()),
		walk(&graph, side.destination(), side.origin()),
	];

	let map = graph.optimize();
	for boundary in boundaries {
		let mapped: Vec<_> = boundary
			.iter()
			.map(|&(from, to)| (map.node(from).unwrap(), map.node(to).unwrap()))
			.collect();
		assert_eq!(walk(&graph, mapped[0].0, mapped[0].1), mapped);
	}
	assert_consistent(&graph);
}

#[test]
fn identifier_exhaustion_returns_an_error_without_mutation() {
	let mut nodes_exhausted = WallGraph::new();
	nodes_exhausted.next_node_id = usize::MAX;
	assert_eq!(
		nodes_exhausted.add_wall(Vec2::ZERO, Vec2::X),
		Err(WallError::IdExhausted)
	);
	assert_eq!(nodes_exhausted.next_node_id, usize::MAX);
	assert_eq!(nodes_exhausted.nodes().count(), 0);
	nodes_exhausted.optimize();
	assert_eq!(nodes_exhausted.next_node_id, 0);
	assert!(nodes_exhausted.add_wall(Vec2::ZERO, Vec2::X).is_ok());

	let mut edges_exhausted = WallGraph::new();
	edges_exhausted.next_edge_id = usize::MAX - 1;
	assert_eq!(
		edges_exhausted.add_wall(Vec2::ZERO, Vec2::X),
		Err(WallError::IdExhausted)
	);
	assert_eq!(edges_exhausted.next_node_id, 0);
	assert_eq!(edges_exhausted.next_edge_id, usize::MAX - 1);
	assert_eq!(edges_exhausted.walls().count(), 0);

	let mut merge_exhausted = WallGraph::new();
	let original = merge_exhausted.add_wall(Vec2::ZERO, Vec2::X).unwrap()[0];
	merge_exhausted.next_edge_id = usize::MAX - 2;
	assert_eq!(
		merge_exhausted.add_wall(Vec2::X, Vec2::new(2.0, 0.0)),
		Err(WallError::IdExhausted)
	);
	assert_eq!(merge_exhausted.next_edge_id, usize::MAX - 2);
	assert_eq!(merge_exhausted.walls().collect::<Vec<_>>(), vec![original]);
	assert_eq!(merge_exhausted.nodes().count(), 2);
	assert_consistent(&merge_exhausted);
}
