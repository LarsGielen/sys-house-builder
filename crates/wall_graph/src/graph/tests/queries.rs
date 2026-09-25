use glam::Vec2;

use crate::WallNodeId;
use crate::graph::test_support::{add_single_wall, graph_with_nodes};

#[test]
fn nodes_lists_every_node_with_its_position() {
	let (graph, [a, b, c]) = graph_with_nodes([(0.0, 0.0), (1.0, 2.0), (-3.0, 4.0)]);

	let mut nodes: Vec<(WallNodeId, Vec2)> = graph.nodes().collect();
	nodes.sort_by_key(|(id, _)| *id);

	assert_eq!(
		nodes,
		vec![
			(a, Vec2::new(0.0, 0.0)),
			(b, Vec2::new(1.0, 2.0)),
			(c, Vec2::new(-3.0, 4.0)),
		]
	);
}

#[test]
fn walls_yields_each_wall_once() {
	let (mut graph, [a, b, c]) = graph_with_nodes([(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)]);
	let first = add_single_wall(&mut graph, a, b);
	let second = add_single_wall(&mut graph, c, b);

	let mut walls: Vec<_> = graph.walls().collect();
	walls.sort_by_key(|wall| wall.forward);

	assert_eq!(walls, vec![first, second]);
}

#[test]
fn an_empty_graph_has_no_nodes_or_walls() {
	let graph = crate::WallGraph::default();

	assert_eq!(graph.nodes().count(), 0);
	assert_eq!(graph.walls().count(), 0);
}

#[test]
fn node_position_is_none_for_a_node_of_another_graph() {
	let (graph, [a]) = graph_with_nodes([(2.0, 3.0)]);
	let (_, [_, ghost]) = graph_with_nodes([(0.0, 0.0), (1.0, 0.0)]);

	assert_eq!(graph.node_position(a), Some(Vec2::new(2.0, 3.0)));
	assert_eq!(graph.node_position(ghost), None);
}
