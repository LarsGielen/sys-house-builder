use std::f32::consts::{FRAC_PI_2, PI, TAU};

use glam::Vec2;

use super::support::{assert_consistent, graph_with_nodes, neighbours_clockwise, node_at};
use crate::WallError;
use crate::graph::{DISTANCE_TOLERANCE, Wall};

fn close(actual: Vec2, expected: Vec2) {
	assert!(actual.distance(expected) < 1e-5, "{actual} != {expected}");
}

fn reverse(wall: Wall) -> Wall {
	Wall {
		forward: wall.backward,
		backward: wall.forward,
		origin: wall.destination,
		destination: wall.origin,
	}
}

#[test]
fn arc_queries_and_reverse_handles_follow_the_curve() {
	let (mut graph, [a, b]) = graph_with_nodes([(1.0, 0.0), (-1.0, 0.0)]);
	let wall = graph.add_arc_between_nodes(a, b, PI).unwrap()[0];
	close(graph.wall_position(wall, 0.5).unwrap(), Vec2::Y);
	close(graph.wall_tangent(wall, 0.0).unwrap(), Vec2::Y);
	assert!((graph.wall_length(wall).unwrap() - std::f64::consts::PI).abs() < 1e-6);
	for parameter in [0.0, 0.25, 0.5, 1.0] {
		close(
			graph.wall_position(reverse(wall), parameter).unwrap(),
			graph.wall_position(wall, 1.0 - parameter).unwrap(),
		);
		close(
			graph.wall_tangent(reverse(wall), parameter).unwrap(),
			-graph.wall_tangent(wall, 1.0 - parameter).unwrap(),
		);
	}
	assert_consistent(&graph);
}

#[test]
fn line_crosses_an_arc_twice_in_either_order_and_direction() {
	for arc_first in [false, true] {
		for arc_reversed in [false, true] {
			for line_reversed in [false, true] {
				let (mut graph, [a, b, c, d]) =
					graph_with_nodes([(1.0, 0.0), (-1.0, 0.0), (-2.0, 0.5), (2.0, 0.5)]);
				let (arc_start, arc_end, sweep) = if arc_reversed {
					(b, a, -PI)
				} else {
					(a, b, PI)
				};
				let (line_start, line_end) = if line_reversed { (d, c) } else { (c, d) };
				let original;
				let added;
				if arc_first {
					original = graph
						.add_arc_between_nodes(arc_start, arc_end, sweep)
						.unwrap()[0];
					added = graph.add_wall_between_nodes(line_start, line_end).unwrap();
				} else {
					original = graph.add_wall_between_nodes(line_start, line_end).unwrap()[0];
					added = graph
						.add_arc_between_nodes(arc_start, arc_end, sweep)
						.unwrap();
				}
				assert_eq!(added.len(), 3);
				assert_eq!(graph.walls().count(), 6);
				assert_eq!(graph.nodes().count(), 6);
				assert_eq!(graph.remove_wall(original), Err(WallError::UnknownWall));
				assert_eq!(graph.wall_position(original, 0.5), None);
				assert_eq!(
					added[0].origin(),
					if arc_first { line_start } else { arc_start }
				);
				assert_eq!(
					added[2].destination(),
					if arc_first { line_end } else { arc_end }
				);
				for wall in graph.walls() {
					let from = graph.node_position(wall.origin()).unwrap();
					let to = graph.node_position(wall.destination()).unwrap();
					if from.y.abs() < 1e-5
						|| to.y.abs() < 1e-5
						|| (from.x.abs() < 1.0 && to.x.abs() < 1.0)
							&& graph.wall_position(wall, 0.5).unwrap().y > 0.6
					{
						assert!(
							(graph.wall_position(wall, 0.5).unwrap().length() - 1.0).abs() < 1e-5
						);
					}
				}
				assert_consistent(&graph);
			}
		}
	}
}

#[test]
fn two_arcs_can_cross_twice() {
	for first_reversed in [false, true] {
		for second_reversed in [false, true] {
			let (mut graph, [a, b, c, d]) =
				graph_with_nodes([(-1.0, 0.0), (0.0, -1.0), (2.0, 0.0), (1.0, -1.0)]);
			let (from, to, sweep) = if first_reversed {
				(b, a, 1.5 * PI)
			} else {
				(a, b, -1.5 * PI)
			};
			let original = graph.add_arc_between_nodes(from, to, sweep).unwrap()[0];
			let (from, to, sweep) = if second_reversed {
				(d, c, -1.5 * PI)
			} else {
				(c, d, 1.5 * PI)
			};
			let added = graph.add_arc_between_nodes(from, to, sweep).unwrap();
			assert_eq!(added.len(), 3);
			assert_eq!(graph.walls().count(), 6);
			assert_eq!(graph.wall_length(original), None);
			for wall in added {
				assert!(
					(graph.wall_position(wall, 0.5).unwrap().distance(Vec2::X) - 1.0).abs() < 1e-5
				);
			}
			assert_consistent(&graph);
		}
	}
}

#[test]
fn sharing_an_endpoint_does_not_hide_another_intersection() {
	let (mut graph, [a, b, c]) = graph_with_nodes([(-1.0, 0.0), (0.0, -1.0), (1.0, 2.0)]);
	graph.add_arc_between_nodes(a, b, -1.5 * PI).unwrap();
	let added = graph.add_arc_between_nodes(a, c, PI).unwrap();
	assert_eq!(added.len(), 2);
	let crossing = node_at(&graph, Vec2::X);
	assert_eq!(added[0].destination(), crossing);
	assert_eq!(graph.walls().count(), 4);
	assert_consistent(&graph);
}

#[test]
fn existing_nodes_on_an_arc_become_junctions_in_path_order() {
	let (mut graph, [a, b, middle]) = graph_with_nodes([(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0)]);
	let added = graph.add_arc_between_nodes(a, b, PI).unwrap();
	assert_eq!(added.len(), 2);
	assert_eq!(added[0].destination(), middle);
	assert_eq!(added[1].origin(), middle);
	assert_eq!(graph.nodes().count(), 3);
	assert_consistent(&graph);
}

#[test]
fn ending_on_an_arc_splits_it_and_preserves_its_shape() {
	let (mut graph, [a, b]) = graph_with_nodes([(1.0, 0.0), (-1.0, 0.0)]);
	let original = graph.add_arc_between_nodes(a, b, PI).unwrap()[0];
	let middle = graph.add_node(Vec2::Y).unwrap();
	let outside = graph.add_node(Vec2::new(0.0, 2.0)).unwrap();
	graph.add_wall_between_nodes(middle, outside).unwrap();
	assert_eq!(graph.wall_length(original), None);
	assert_eq!(graph.walls().count(), 3);
	for wall in graph.walls().filter(|wall| wall.destination() != outside) {
		assert!((graph.wall_position(wall, 0.5).unwrap().length() - 1.0).abs() < 1e-5);
	}
	assert_consistent(&graph);
}

#[test]
fn two_endpoints_on_the_same_existing_arc_split_it_once_into_three_pieces() {
	let (mut graph, [a, b]) = graph_with_nodes([(1.0, 0.0), (-1.0, 0.0)]);
	let original = graph.add_arc_between_nodes(a, b, PI).unwrap()[0];
	let left = graph.add_node(Vec2::new(-0.6, 0.8)).unwrap();
	let right = graph.add_node(Vec2::new(0.6, 0.8)).unwrap();
	assert_eq!(graph.add_wall_between_nodes(left, right).unwrap().len(), 1);
	assert_eq!(graph.walls().count(), 4);
	assert_eq!(graph.wall_length(original), None);
	assert_consistent(&graph);
}

#[test]
fn distinct_paths_between_the_same_endpoints_form_two_faces() {
	let (mut graph, [a, b]) = graph_with_nodes([(1.0, 0.0), (-1.0, 0.0)]);
	let line = graph.add_wall_between_nodes(a, b).unwrap()[0];
	let arc = graph.add_arc_between_nodes(a, b, PI).unwrap()[0];
	assert_eq!(graph.walls().count(), 2);
	assert_eq!(graph.edge(line.forward).next, arc.backward);
	assert_eq!(graph.edge(arc.backward).next, line.forward);
	assert_eq!(graph.edge(line.backward).next, arc.forward);
	assert_consistent(&graph);
	graph.remove_wall(arc).unwrap();
	assert_eq!(graph.wall_position(arc, 0.0), None);
	assert_eq!(graph.walls().collect::<Vec<_>>(), vec![line]);
	assert_consistent(&graph);
}

#[test]
fn opposite_semicircles_can_form_a_circle_with_two_nodes() {
	let (mut graph, [a, b]) = graph_with_nodes([(1.0, 0.0), (-1.0, 0.0)]);
	graph.add_arc_between_nodes(a, b, PI).unwrap();
	graph.add_arc_between_nodes(a, b, -PI).unwrap();
	assert_eq!(graph.walls().count(), 2);
	assert_consistent(&graph);
}

#[test]
fn departure_order_uses_tangents_instead_of_chords() {
	let (mut graph, [a, b, north, west]) =
		graph_with_nodes([(0.0, 0.0), (2.0, 0.0), (0.0, 2.0), (-2.0, 0.0)]);
	graph.add_wall_between_nodes(a, north).unwrap();
	graph.add_wall_between_nodes(a, west).unwrap();
	graph.add_arc_between_nodes(a, b, PI).unwrap();
	// The arc's chord points east, but its departure points south.
	assert_eq!(neighbours_clockwise(&graph, a, north), vec![north, b, west]);
	assert_consistent(&graph);
}

#[test]
fn duplicate_arcs_are_rejected_in_both_directions() {
	let (mut graph, [a, b]) = graph_with_nodes([(1.0, 0.0), (-1.0, 0.0)]);
	let original = graph.add_arc_between_nodes(a, b, PI).unwrap()[0];
	assert_eq!(
		graph.add_arc_between_nodes(a, b, PI),
		Err(WallError::Duplicate)
	);
	assert_eq!(
		graph.add_arc_between_nodes(b, a, -PI),
		Err(WallError::Duplicate)
	);
	assert_eq!(graph.walls().collect::<Vec<_>>(), vec![original]);
}

#[test]
fn coincident_arc_overlap_is_rejected_atomically() {
	let (mut graph, [a, b]) = graph_with_nodes([(1.0, 0.0), (-1.0, 0.0)]);
	let original = graph.add_arc_between_nodes(a, b, PI).unwrap()[0];
	let middle = graph.add_node(Vec2::Y).unwrap();
	let ids = (graph.next_node_id, graph.next_edge_id);
	assert_eq!(
		graph.add_arc_between_nodes(a, middle, FRAC_PI_2),
		Err(WallError::Overlapping)
	);
	assert_eq!(
		graph.add_arc_between_nodes(middle, a, -FRAC_PI_2),
		Err(WallError::Overlapping)
	);
	assert_eq!((graph.next_node_id, graph.next_edge_id), ids);
	assert_eq!(graph.walls().collect::<Vec<_>>(), vec![original]);
	assert_consistent(&graph);
}

#[test]
fn tangent_line_contact_is_rejected_before_any_split() {
	let (mut graph, [a, b, c, d]) =
		graph_with_nodes([(1.0, 0.0), (-1.0, 0.0), (-2.0, 1.0), (2.0, 1.0)]);
	let original = graph.add_arc_between_nodes(a, b, PI).unwrap()[0];
	let ids = (graph.next_node_id, graph.next_edge_id);
	assert_eq!(
		graph.add_wall_between_nodes(c, d),
		Err(WallError::TangentialContact)
	);
	assert_eq!((graph.next_node_id, graph.next_edge_id), ids);
	assert_eq!(graph.walls().collect::<Vec<_>>(), vec![original]);
	assert_consistent(&graph);
}

#[test]
fn tangent_departures_at_a_shared_endpoint_are_rejected() {
	let (mut graph, [a, b, south]) = graph_with_nodes([(0.0, 0.0), (2.0, 0.0), (0.0, -2.0)]);
	graph.add_arc_between_nodes(a, b, PI).unwrap();
	assert_eq!(
		graph.add_wall_between_nodes(a, south),
		Err(WallError::TangentialContact)
	);
	assert_consistent(&graph);
}

#[test]
fn invalid_arc_sweeps_do_not_mutate_the_graph() {
	let (mut graph, [a, b]) = graph_with_nodes([(0.0, 0.0), (1.0, 0.0)]);
	let ids = (graph.next_node_id, graph.next_edge_id);
	for sweep in [0.0, 1e-7, -1e-7, TAU, -TAU, f32::NAN, f32::INFINITY] {
		assert_eq!(
			graph.add_arc_between_nodes(a, b, sweep),
			Err(WallError::InvalidArc)
		);
	}
	assert_eq!((graph.next_node_id, graph.next_edge_id), ids);
	assert_eq!(graph.walls().count(), 0);
	assert_consistent(&graph);
}

#[test]
fn sampling_changes_no_topology_and_validates_queries() {
	let (mut graph, [a, b]) = graph_with_nodes([(1.0, 0.0), (-1.0, 0.0)]);
	let wall = graph.add_arc_between_nodes(a, b, PI).unwrap()[0];
	let coarse = graph.sample_wall(wall, 0.1).unwrap();
	let fine = graph.sample_wall(wall, 0.001).unwrap();
	assert!(fine.len() > coarse.len());
	assert_eq!(fine[0], Vec2::X);
	assert_eq!(*fine.last().unwrap(), -Vec2::X);
	assert_eq!(graph.nodes().count(), 2);
	assert_eq!(graph.walls().count(), 1);
	for parameter in [-1.0, 2.0, f32::NAN, f32::INFINITY] {
		assert_eq!(graph.wall_position(wall, parameter), None);
		assert_eq!(graph.wall_tangent(wall, parameter), None);
	}
	for deviation in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::MIN_POSITIVE] {
		assert_eq!(
			graph.sample_wall(wall, deviation),
			Err(WallError::InvalidParameter)
		);
	}
	graph.remove_wall(reverse(wall)).unwrap();
	assert_eq!(graph.sample_wall(wall, 0.1), Err(WallError::UnknownWall));
	assert!(graph.wall_data.is_empty());
	assert_consistent(&graph);
}

#[test]
fn an_endpoint_near_an_arc_within_tolerance_can_split_it() {
	let (mut graph, [a, b]) = graph_with_nodes([(1.0, 0.0), (-1.0, 0.0)]);
	graph.add_arc_between_nodes(a, b, PI).unwrap();
	let middle = graph
		.add_node(Vec2::new(0.0, 1.0 + DISTANCE_TOLERANCE / 2.0))
		.unwrap();
	let outside = graph.add_node(Vec2::new(0.0, 2.0)).unwrap();
	graph.add_wall_between_nodes(middle, outside).unwrap();
	assert_eq!(graph.walls().count(), 3);
	assert_consistent(&graph);
}

#[test]
fn near_tangent_arcs_are_rejected_in_either_insertion_order() {
	for reversed_order in [false, true] {
		let (mut graph, [a, b, c, d]) =
			graph_with_nodes([(1.0, 0.0), (-1.0, 0.0), (1.0, 2.0), (-1.0, 2.0)]);
		let (first_start, first_end, first_sweep, second_start, second_end, second_sweep) =
			if reversed_order {
				(c, d, -PI, a, b, PI)
			} else {
				(a, b, PI, c, d, -PI)
			};
		let original = graph
			.add_arc_between_nodes(first_start, first_end, first_sweep)
			.unwrap()[0];
		let identifiers = (graph.next_node_id, graph.next_edge_id);
		assert_eq!(
			graph.add_arc_between_nodes(second_start, second_end, second_sweep),
			Err(WallError::TangentialContact)
		);
		assert_eq!((graph.next_node_id, graph.next_edge_id), identifiers);
		assert_eq!(graph.walls().collect::<Vec<_>>(), vec![original]);
		assert_consistent(&graph);
	}
}

#[test]
fn a_later_tangency_rejects_earlier_crossings_without_mutation() {
	let (mut graph, [a, b, bottom, top, start, end]) = graph_with_nodes([
		(1.0, 0.0),
		(-1.0, 0.0),
		(-2.0, 0.0),
		(-2.0, 2.0),
		(-3.0, 1.0),
		(3.0, 1.0),
	]);
	let arc = graph.add_arc_between_nodes(a, b, PI).unwrap()[0];
	let line = graph.add_wall_between_nodes(bottom, top).unwrap()[0];
	let identifiers = (graph.next_node_id, graph.next_edge_id);
	assert_eq!(
		graph.add_wall_between_nodes(start, end),
		Err(WallError::TangentialContact)
	);
	assert_eq!((graph.next_node_id, graph.next_edge_id), identifiers);
	assert_eq!(graph.walls().count(), 2);
	assert!(graph.wall_length(arc).is_some());
	assert!(graph.wall_length(line).is_some());
	assert_consistent(&graph);
}

#[test]
fn major_arcs_that_overflow_the_coordinate_type_are_rejected() {
	let (mut graph, [a, b]) = graph_with_nodes([(0.0, 0.0), (1e38, 0.0)]);
	assert_eq!(
		graph.add_arc_between_nodes(a, b, TAU - 0.01),
		Err(WallError::InvalidArc)
	);
	assert_eq!(graph.walls().count(), 0);
}

#[test]
fn a_shallow_arc_remains_curved_and_can_be_split() {
	let (mut graph, [a, b]) = graph_with_nodes([(0.0, 0.0), (100.0, 0.0)]);
	let wall = graph.add_arc_between_nodes(a, b, 1e-4).unwrap()[0];
	let midpoint = graph.wall_position(wall, 0.5).unwrap();
	assert!(midpoint.y < -0.001);
	let below = graph.add_node(Vec2::new(50.0, -1.0)).unwrap();
	let above = graph.add_node(Vec2::new(50.0, 1.0)).unwrap();
	assert_eq!(graph.add_wall_between_nodes(below, above).unwrap().len(), 2);
	assert_eq!(graph.walls().count(), 4);
	assert_eq!(graph.wall_length(wall), None);
	assert_consistent(&graph);
}

#[test]
fn snapping_that_distorts_a_major_arc_is_rejected_atomically() {
	let (mut graph, [a, b]) = graph_with_nodes([(1.0, 0.0), (0.0, -1.0)]);
	let sweep = 1.5 * PI;
	let curve = crate::geometry::Curve::new(
		Vec2::X,
		-Vec2::Y,
		crate::geometry::CurveShape::CircularArc {
			sweep: sweep as f64,
		},
	);
	let position = curve.position(0.01).as_vec2() * (1.0 + DISTANCE_TOLERANCE * 0.9);
	graph.add_node(position).unwrap();
	let identifiers = (graph.next_node_id, graph.next_edge_id);
	assert_eq!(
		graph.add_arc_between_nodes(a, b, sweep),
		Err(WallError::InconsistentJunction)
	);
	assert_eq!((graph.next_node_id, graph.next_edge_id), identifiers);
	assert_eq!(graph.walls().count(), 0);
	assert_consistent(&graph);
}
