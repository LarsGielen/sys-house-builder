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

#[test]
fn node_walls_lists_departures_counterclockwise_from_the_smallest_angle() {
	use crate::WallGraph;

	let mut graph = WallGraph::new();
	let east = graph.add_wall(Vec2::ZERO, Vec2::new(1.0, 0.0)).unwrap()[0];
	let north = graph.add_wall(Vec2::new(0.0, 1.0), Vec2::ZERO).unwrap()[0];
	let west = graph.add_wall(Vec2::ZERO, Vec2::new(-1.0, 0.0)).unwrap()[0];
	let south = graph.add_wall(Vec2::new(0.0, -1.0), Vec2::ZERO).unwrap()[0];
	let center = east.origin();

	// Departure angles: south -PI/2, east 0, north PI/2, west PI.
	assert_eq!(
		graph.node_walls(center),
		Some(vec![south, east, north, west])
	);
	assert_eq!(graph.node_walls(east.destination()), Some(vec![east]));
}

#[test]
fn node_walls_orders_arcs_by_departure_tangent_rather_than_chord() {
	use crate::WallGraph;

	let mut graph = WallGraph::new();
	// Both chords point east, but the arcs leave northward and southward.
	let upper = graph
		.add_arc(
			Vec2::ZERO,
			Vec2::new(2.0, 0.0),
			-std::f32::consts::FRAC_PI_2,
		)
		.unwrap()[0];
	let lower = graph
		.add_arc(Vec2::ZERO, Vec2::new(2.0, 0.0), std::f32::consts::FRAC_PI_2)
		.unwrap()[0];
	let straight = graph.add_wall(Vec2::ZERO, Vec2::new(-1.0, 0.5)).unwrap()[0];

	assert_eq!(
		graph.node_walls(upper.origin()),
		Some(vec![lower, upper, straight])
	);
}

#[test]
fn node_walls_is_none_for_an_unknown_node() {
	let (graph, _) = graph_with_nodes([(0.0, 0.0)]);
	let (_, [_, ghost]) = graph_with_nodes([(0.0, 0.0), (1.0, 0.0)]);

	assert_eq!(graph.node_walls(ghost), None);
}

#[test]
fn wall_curve_describes_lines_and_arcs_in_handle_direction() {
	use crate::{WallCurve, WallGraph};
	use glam::DVec2;

	let mut graph = WallGraph::new();
	let line = graph
		.add_wall(Vec2::new(1.0, 2.0), Vec2::new(4.0, 6.0))
		.unwrap()[0];
	assert_eq!(
		graph.wall_curve(line),
		Some(WallCurve::Straight {
			start: DVec2::new(1.0, 2.0),
			end: DVec2::new(4.0, 6.0),
		})
	);

	// A clockwise major arc around (10, 0) from (11, 0) to (10, 1).
	let arc = graph
		.add_arc(
			Vec2::new(11.0, 0.0),
			Vec2::new(10.0, 1.0),
			-1.5 * std::f32::consts::PI,
		)
		.unwrap()[0];
	let Some(WallCurve::CircularArc {
		center,
		radius,
		start_angle,
		sweep,
	}) = graph.wall_curve(arc)
	else {
		panic!("expected an arc");
	};
	let origin = graph.node_position(arc.origin()).unwrap().as_dvec2();
	let destination = graph.node_position(arc.destination()).unwrap().as_dvec2();
	assert!(center.distance(DVec2::new(10.0, 0.0)) < 1e-6);
	assert!((radius - 1.0).abs() < 1e-6);
	assert!((center + DVec2::from_angle(start_angle) * radius).distance(origin) < 1e-6);
	assert!(
		(center + DVec2::from_angle(start_angle + sweep) * radius).distance(destination) < 1e-6
	);
	assert!((sweep.abs() - 1.5 * std::f64::consts::PI).abs() < 1e-6);
	assert!((graph.wall_length(arc).unwrap() - radius * sweep.abs()).abs() < 1e-9);
}

#[test]
fn wall_curve_is_none_for_a_stale_handle() {
	use crate::WallGraph;

	let mut graph = WallGraph::new();
	let wall = graph.add_wall(Vec2::ZERO, Vec2::new(1.0, 0.0)).unwrap()[0];
	graph.remove_wall(wall).unwrap();

	assert_eq!(graph.wall_curve(wall), None);
}
