use glam::Vec2;

use super::support::{add_single_wall, graph_with_nodes};
use crate::WallNodeId;

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
#[test]
fn closest_point_follows_line_and_arc_geometry() {
	use crate::WallGraph;
	use glam::Vec2;

	let mut graph = WallGraph::new();
	let line = graph
		.add_wall(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0))
		.unwrap()[0];
	let (parameter, point) = graph.wall_closest_point(line, Vec2::new(4.0, 3.0)).unwrap();
	assert!((parameter - 0.4).abs() < 1e-6);
	assert_eq!(point, Vec2::new(4.0, 0.0));

	let arc = graph
		.add_arc(
			Vec2::new(20.0, 0.0),
			Vec2::new(18.0, 0.0),
			std::f32::consts::PI,
		)
		.unwrap()[0];
	let (parameter, point) = graph.wall_closest_point(arc, Vec2::new(19.0, 2.0)).unwrap();
	assert!((parameter - 0.5).abs() < 1e-6);
	assert!(point.distance(Vec2::new(19.0, 1.0)) < 1e-5);
	assert!(
		graph
			.wall_closest_point(arc, Vec2::new(f32::NAN, 0.0))
			.is_none()
	);
}
