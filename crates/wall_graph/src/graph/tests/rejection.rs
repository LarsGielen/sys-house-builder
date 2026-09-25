use glam::Vec2;

use crate::WallError;
use crate::WallGraph;
use crate::graph::test_support::{
	assert_consistent, flip_combinations, graph_with_nodes, permutations,
};

#[test]
fn non_finite_node_positions_are_rejected() {
	let mut graph = WallGraph::new();

	for position in [
		Vec2::new(f32::NAN, 0.0),
		Vec2::new(f32::INFINITY, 0.0),
		Vec2::new(0.0, f32::NEG_INFINITY),
	] {
		assert_eq!(graph.add_node(position), Err(WallError::InvalidPosition));
	}

	assert_eq!(graph.nodes().count(), 0);
}

#[test]
fn wall_to_a_node_of_another_graph_is_rejected() {
	let (mut graph, [a]) = graph_with_nodes([(0.0, 0.0)]);
	let (_, [_, ghost]) = graph_with_nodes([(0.0, 0.0), (1.0, 0.0)]);

	assert_eq!(graph.add_wall(a, ghost), Err(WallError::UnknownNode));
	assert_eq!(graph.add_wall(ghost, a), Err(WallError::UnknownNode));
	assert!(graph.edges.is_empty());
}

#[test]
fn wall_from_a_node_to_itself_is_rejected() {
	let (mut graph, [a, b]) = graph_with_nodes([(0.0, 0.0), (1.0, 0.0)]);
	graph.add_wall(a, b).unwrap();

	assert_eq!(graph.add_wall(a, a), Err(WallError::SameNode));
	assert_eq!(graph.add_wall(b, b), Err(WallError::SameNode));
	assert_eq!(graph.edges.len(), 2);
	assert_consistent(&graph);
}

#[test]
fn duplicate_wall_is_rejected_in_either_direction() {
	let (mut graph, [a, b]) = graph_with_nodes([(0.0, 0.0), (1.0, 0.0)]);
	graph.add_wall(a, b).unwrap();

	assert_eq!(graph.add_wall(a, b), Err(WallError::Duplicate));
	assert_eq!(graph.add_wall(b, a), Err(WallError::Duplicate));
	assert_eq!(graph.edges.len(), 2);
	assert_consistent(&graph);
}

#[test]
fn duplicate_wall_is_rejected_at_a_busy_node() {
	//        D
	//        |
	// A -----B----- C
	let (mut graph, [a, b, c, d]) =
		graph_with_nodes([(-1.0, 0.0), (0.0, 0.0), (1.0, 0.0), (0.0, 1.0)]);
	graph.add_wall(a, b).unwrap();
	graph.add_wall(b, c).unwrap();
	graph.add_wall(b, d).unwrap();

	for (from, to) in [(a, b), (b, a), (b, c), (c, b), (b, d), (d, b)] {
		assert_eq!(graph.add_wall(from, to), Err(WallError::Duplicate));
	}
	assert_eq!(graph.edges.len(), 6);
	assert_consistent(&graph);
}

#[test]
fn overlapping_walls_are_rejected_in_any_order_and_direction() {
	// A --- B ------- C   all on one line, so A-B and B-C both lie inside A-C
	let pairs = [
		[(0, 1), (0, 2)],
		[(0, 2), (0, 1)],
		[(1, 2), (0, 2)],
		[(0, 2), (1, 2)],
	];
	let positions = [(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)];
	for [first, second] in pairs {
		for flipped in flip_combinations(2) {
			// The node left out of the first wall is added after it, so A-C doesn't simply run through B.
			let mut graph = WallGraph::new();
			let mut nodes = [None; 3];
			for i in [first.0, first.1] {
				nodes[i] = Some(graph.add_node(Vec2::from(positions[i])).unwrap());
			}
			let (from, to) = if flipped[0] {
				(first.1, first.0)
			} else {
				first
			};
			graph
				.add_wall(nodes[from].unwrap(), nodes[to].unwrap())
				.unwrap();
			let unused = (0..3).find(|&i| i != first.0 && i != first.1).unwrap();
			nodes[unused] = Some(graph.add_node(Vec2::from(positions[unused])).unwrap());
			let nodes = nodes.map(Option::unwrap);

			let (from, to) = if flipped[1] {
				(nodes[second.1], nodes[second.0])
			} else {
				(nodes[second.0], nodes[second.1])
			};
			assert_eq!(
				graph.add_wall(from, to),
				Err(WallError::Overlapping),
				"{first:?} then {second:?}, flipped {flipped:?}"
			);

			assert_eq!(graph.edges.len(), 2);
			assert_consistent(&graph);

			// a rejected wall must not leave the graph in a state that breaks later walls
			let d = graph.add_node(Vec2::new(0.0, 1.0)).unwrap();
			graph.add_wall(nodes[0], d).unwrap();
			assert_eq!(graph.edges.len(), 4);
			assert_consistent(&graph);
		}
	}
}

#[test]
fn overlap_is_detected_among_many_walls_at_a_node() {
	// Three walls leave A (west, north, east); a fourth collinear with any one of them overlaps it.
	let arms = [
		((-1.0, 0.0), (-2.0, 0.0)),
		((0.0, 1.0), (0.0, 2.0)),
		((1.0, 0.0), (2.0, 0.0)),
	];
	for (arm, beyond) in arms {
		for order in permutations(vec![0, 1, 2]) {
			let (mut graph, [a, west, north, east]) =
				graph_with_nodes([(0.0, 0.0), arms[0].0, arms[1].0, arms[2].0]);
			let arm_nodes = [west, north, east];
			for &i in &order {
				graph.add_wall(a, arm_nodes[i]).unwrap();
			}

			let far = graph.add_node(Vec2::new(beyond.0, beyond.1)).unwrap();
			assert_eq!(
				graph.add_wall(a, far),
				Err(WallError::Overlapping),
				"beyond {beyond:?} ({arm:?}), insertion order {order:?}"
			);
			assert_eq!(graph.edges.len(), 6);
			assert_consistent(&graph);
		}
	}
}

#[test]
fn nearly_collinear_walls_are_rejected_despite_float_noise() {
	// (0.7, 0.1) and (2.1, 0.3) are on one line through the origin, but their f32 angles differ in the last bits.
	let (mut graph, [a, b, c]) = graph_with_nodes([(0.0, 0.0), (0.7, 0.1), (2.1, 0.3)]);
	graph.add_wall(a, b).unwrap();

	assert_eq!(graph.add_wall(a, c), Err(WallError::Overlapping));
	assert_eq!(graph.edges.len(), 2);
}

#[test]
fn overlap_is_detected_across_the_pi_boundary() {
	// Both walls point west. atan2 gives +PI for the first and -PI for the second, which are the same direction.
	let (mut graph, [a, b, c]) = graph_with_nodes([(0.0, 0.0), (-1.0, 0.0), (-2.0, -1e-7)]);
	graph.add_wall(a, b).unwrap();

	assert_eq!(graph.add_wall(a, c), Err(WallError::Overlapping));
	assert_eq!(graph.edges.len(), 2);
}

#[test]
fn walls_at_a_small_but_real_angle_are_accepted() {
	// 0.02 radians apart: close, but not overlapping.
	let (mut graph, [a, b, c]) = graph_with_nodes([(0.0, 0.0), (10.0, 0.0), (10.0, 0.2)]);

	assert!(graph.add_wall(a, b).is_ok());
	assert!(graph.add_wall(a, c).is_ok());
	assert_consistent(&graph);
}
