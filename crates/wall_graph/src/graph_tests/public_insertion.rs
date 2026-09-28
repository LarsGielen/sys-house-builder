use std::f32::consts::{FRAC_PI_2, PI, TAU};

use glam::Vec2;

use super::support::{assert_consistent, node_at};
use crate::{WallError, WallGraph};

#[test]
fn a_new_wall_creates_only_its_endpoint_nodes() {
	let mut graph = WallGraph::new();
	let first = graph.add_wall(Vec2::ZERO, Vec2::X).unwrap()[0];

	assert_eq!(graph.nodes().count(), 2);
	assert_eq!(graph.walls().count(), 1);
	assert_eq!(graph.node_position(first.origin()), Some(Vec2::ZERO));
	assert_eq!(graph.node_position(first.destination()), Some(Vec2::X));
	assert!(
		graph
			.nodes()
			.all(|(id, _)| graph.node(id).outgoing_edge.is_some())
	);
	assert_consistent(&graph);
}

#[test]
fn later_walls_reuse_existing_endpoint_nodes() {
	let mut graph = WallGraph::new();
	let first = graph.add_wall(Vec2::ZERO, Vec2::X).unwrap()[0];
	let second = graph.add_wall(Vec2::X, Vec2::Y).unwrap()[0];
	assert_eq!(second.origin(), first.destination());
	assert_eq!(graph.wall_length(first), Some(1.0));
	assert_eq!(graph.nodes().count(), 3);

	let near_origin = Vec2::new(0.00005, 0.0);
	let third = graph.add_wall(near_origin, -Vec2::Y).unwrap()[0];
	assert_eq!(third.origin(), first.origin());
	assert_eq!(graph.nodes().count(), 4);
	assert_eq!(graph.node_position(third.origin()), Some(Vec2::ZERO));
	assert_consistent(&graph);
}

#[test]
fn a_wall_end_on_an_existing_wall_becomes_a_shared_junction() {
	let mut graph = WallGraph::new();
	let original = graph.add_wall(Vec2::ZERO, Vec2::new(2.0, 0.0)).unwrap()[0];
	let added = graph.add_wall(Vec2::new(1.0, 0.0), Vec2::Y).unwrap();

	assert_eq!(added.len(), 1);
	let middle = node_at(&graph, Vec2::X);
	assert_eq!(added[0].origin(), middle);
	assert_eq!(graph.walls().count(), 3);
	assert_eq!(graph.nodes().count(), 4);
	assert_eq!(graph.wall_length(original), None);
	assert_eq!(graph.node_position(original.origin()), Some(Vec2::ZERO));
	assert_consistent(&graph);
}

#[test]
fn crossing_creates_one_junction_and_splits_both_paths() {
	let mut graph = WallGraph::new();
	let existing = graph.add_wall(Vec2::new(-1.0, 0.0), Vec2::X).unwrap()[0];
	let added = graph.add_wall(-Vec2::Y, Vec2::Y).unwrap();

	assert_eq!(added.len(), 2);
	assert_eq!(added[0].destination(), added[1].origin());
	assert_eq!(
		graph.node_position(added[0].destination()),
		Some(Vec2::ZERO)
	);
	assert_eq!(graph.walls().count(), 4);
	assert_eq!(graph.nodes().count(), 5);
	assert_eq!(graph.wall_length(existing), None);
	assert_consistent(&graph);
}

#[test]
fn rejected_wall_does_not_create_endpoints_or_consume_identifiers() {
	let mut graph = WallGraph::new();
	let original = graph.add_wall(Vec2::ZERO, Vec2::new(2.0, 0.0)).unwrap()[0];
	let identifiers = (graph.next_node_id, graph.next_edge_id);

	assert_eq!(
		graph.add_wall(Vec2::X, Vec2::new(3.0, 0.0)),
		Err(WallError::Overlapping)
	);
	assert_eq!((graph.next_node_id, graph.next_edge_id), identifiers);
	assert_eq!(graph.nodes().count(), 2);
	assert_eq!(graph.walls().collect::<Vec<_>>(), vec![original]);
	assert_eq!(graph.node_position(original.origin()), Some(Vec2::ZERO));
	assert_consistent(&graph);
}

#[test]
fn rejected_crossing_after_a_valid_contact_preserves_all_original_walls() {
	let mut graph = WallGraph::new();
	let crossing = graph
		.add_wall(Vec2::new(1.0, -1.0), Vec2::new(1.0, 1.0))
		.unwrap()[0];
	let overlapping = graph
		.add_wall(Vec2::new(3.0, 0.0), Vec2::new(5.0, 0.0))
		.unwrap()[0];
	let identifiers = (graph.next_node_id, graph.next_edge_id);

	assert_eq!(
		graph.add_wall(Vec2::ZERO, Vec2::new(4.0, 0.0)),
		Err(WallError::Overlapping)
	);
	assert_eq!((graph.next_node_id, graph.next_edge_id), identifiers);
	assert_eq!(graph.nodes().count(), 4);
	assert!(graph.wall_length(crossing).is_some());
	assert!(graph.wall_length(overlapping).is_some());
	assert_consistent(&graph);
}

#[test]
fn arc_insertion_is_atomic_and_reuses_existing_nodes() {
	let mut graph = WallGraph::new();
	let straight = graph.add_wall(Vec2::X, -Vec2::X).unwrap()[0];
	let curved = graph.add_arc(Vec2::X, -Vec2::X, PI).unwrap()[0];
	assert_eq!(curved.origin(), straight.origin());
	assert_eq!(curved.destination(), straight.destination());
	assert_eq!(graph.nodes().count(), 2);
	assert_eq!(graph.walls().count(), 2);

	let identifiers = (graph.next_node_id, graph.next_edge_id);
	assert_eq!(
		graph.add_arc(Vec2::new(0.0, 1.0), Vec2::new(-1.0, 0.0), FRAC_PI_2),
		Err(WallError::Overlapping)
	);
	assert_eq!((graph.next_node_id, graph.next_edge_id), identifiers);
	assert_eq!(graph.nodes().count(), 2);
	assert_eq!(graph.walls().count(), 2);
	assert_consistent(&graph);
}

#[test]
fn invalid_positions_and_sweeps_leave_the_graph_empty() {
	let mut graph = WallGraph::new();
	for invalid in [Vec2::new(f32::NAN, 0.0), Vec2::new(0.0, f32::INFINITY)] {
		assert_eq!(
			graph.add_wall(invalid, Vec2::X),
			Err(WallError::InvalidPosition)
		);
		assert_eq!(
			graph.add_arc(Vec2::ZERO, invalid, PI),
			Err(WallError::InvalidPosition)
		);
	}
	for invalid in [0.0, TAU, f32::NAN] {
		assert_eq!(
			graph.add_arc(Vec2::ZERO, Vec2::X, invalid),
			Err(WallError::InvalidArc)
		);
	}
	assert_eq!(
		graph.add_wall(Vec2::ZERO, Vec2::ZERO),
		Err(WallError::ZeroLength)
	);
	assert_eq!(graph.nodes().count(), 0);
	assert_eq!(graph.walls().count(), 0);
	assert_eq!(graph.next_node_id, 0);
	assert_eq!(graph.next_edge_id, 0);
}

#[test]
fn removal_cleans_up_only_nodes_left_without_walls() {
	let mut graph = WallGraph::new();
	let first = graph.add_wall(Vec2::ZERO, Vec2::X).unwrap()[0];
	let second = graph.add_wall(Vec2::X, Vec2::new(2.0, 0.0)).unwrap()[0];

	graph.remove_wall(first).unwrap();
	assert_eq!(graph.node_position(first.origin()), None);
	assert_eq!(graph.node_position(first.destination()), Some(Vec2::X));
	assert_eq!(graph.nodes().count(), 2);
	graph.remove_wall(second).unwrap();
	assert_eq!(graph.nodes().count(), 0);
	assert_consistent(&graph);
}

#[test]
fn two_requested_endpoints_that_snap_to_one_node_are_rejected_atomically() {
	let mut graph = WallGraph::new();
	let original = graph.add_wall(Vec2::ZERO, Vec2::new(-1.0, 0.0)).unwrap()[0];
	let identifiers = (graph.next_node_id, graph.next_edge_id);
	let offset = 0.75 * crate::graph::DISTANCE_TOLERANCE;
	assert_eq!(
		graph.add_wall(Vec2::new(-offset, 0.0), Vec2::new(offset, 0.0)),
		Err(WallError::ZeroLength)
	);
	assert_eq!((graph.next_node_id, graph.next_edge_id), identifiers);
	assert_eq!(graph.nodes().count(), 2);
	assert_eq!(graph.walls().collect::<Vec<_>>(), vec![original]);
	assert_consistent(&graph);
}
