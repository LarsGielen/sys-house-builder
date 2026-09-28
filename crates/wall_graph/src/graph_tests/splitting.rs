use glam::Vec2;

use super::support::{
	add_single_wall, assert_consistent, graph_with_nodes, neighbours_clockwise, sorted_pairs, walk,
	wall_node_pairs,
};
use crate::graph::DISTANCE_TOLERANCE;

#[test]
fn a_split_wall_keeps_its_direction_in_both_halves() {
	//      C
	//      |
	//  A<--M<--B    the existing wall runs from B to A
	let (mut graph, [a, b, c]) = graph_with_nodes([(0.0, 0.0), (2.0, 0.0), (1.0, 1.0)]);
	add_single_wall(&mut graph, b, a);
	let m = graph.add_node(Vec2::new(1.0, 0.0)).unwrap();

	add_single_wall(&mut graph, m, c);

	let mut walls: Vec<_> = graph.walls().map(|w| (w.origin, w.destination)).collect();
	walls.sort();
	let mut expected = vec![(b, m), (m, a), (m, c)];
	expected.sort();
	assert_eq!(walls, expected);
	assert_consistent(&graph);
}

#[test]
fn splitting_keeps_the_order_of_walls_around_both_ends_of_the_split_wall() {
	//        N       T
	//        |       |
	//   W ---A-------M------- E ---F
	//                         |
	//                         S
	let (mut graph, [a, w, n, e, f, s, t]) = graph_with_nodes([
		(0.0, 0.0),
		(-1.0, 0.0),
		(0.0, 1.0),
		(2.0, 0.0),
		(3.0, 0.0),
		(2.0, -1.0),
		(1.0, 1.0),
	]);
	add_single_wall(&mut graph, a, w);
	add_single_wall(&mut graph, a, n);
	add_single_wall(&mut graph, a, e);
	add_single_wall(&mut graph, e, f);
	add_single_wall(&mut graph, e, s);
	let m = graph.add_node(Vec2::new(1.0, 0.0)).unwrap();

	add_single_wall(&mut graph, t, m);

	assert_consistent(&graph);
	assert_eq!(neighbours_clockwise(&graph, a, w), vec![w, n, m]);
	assert_eq!(neighbours_clockwise(&graph, e, m), vec![m, f, s]);
	assert_eq!(neighbours_clockwise(&graph, m, a), vec![a, t, e]);
}

#[test]
fn a_wall_ending_on_a_room_from_outside_keeps_the_room_closed() {
	//  C
	//  | \
	//  A-M-B
	//    |
	//    S
	let (mut graph, [a, b, c, s]) =
		graph_with_nodes([(0.0, 0.0), (2.0, 0.0), (0.0, 2.0), (1.0, -1.0)]);
	add_single_wall(&mut graph, a, b);
	add_single_wall(&mut graph, b, c);
	add_single_wall(&mut graph, c, a);
	let m = graph.add_node(Vec2::new(1.0, 0.0)).unwrap();

	add_single_wall(&mut graph, s, m);

	assert_consistent(&graph);
	assert_eq!(walk(&graph, a, m), vec![(a, m), (m, b), (b, c), (c, a)]);
	assert_eq!(
		walk(&graph, m, a),
		vec![(m, a), (a, c), (c, b), (b, m), (m, s), (s, m)]
	);
}

#[test]
fn a_wall_can_be_split_again_and_again() {
	//      T2  T3  T1
	//      |   |   |
	//  A---+---+---+---B    walls from above land on A-B in the order T1, T3, T2
	let (mut graph, [a, b]) = graph_with_nodes([(0.0, 0.0), (4.0, 0.0)]);
	add_single_wall(&mut graph, a, b);
	let [at_2, at_3, at_1] = [2.0, 3.0, 1.0].map(|x| {
		let top = graph.add_node(Vec2::new(x, 1.0)).unwrap();
		let on_wall = graph.add_node(Vec2::new(x, 0.0)).unwrap();
		add_single_wall(&mut graph, top, on_wall);
		on_wall
	});

	assert_consistent(&graph);
	assert_eq!(graph.walls().count(), 4 + 3);
	// The loop under the wall runs west along its whole length.
	assert_eq!(
		walk(&graph, b, at_3)[..4],
		[(b, at_3), (at_3, at_2), (at_2, at_1), (at_1, a)]
	);
}

#[test]
fn a_wall_ending_on_another_wall_splits_it_there() {
	//      C             (or below, at (1, -1))
	//      |
	//  A---M---B
	for c_y in [1.0, -1.0] {
		for wall_reversed in [false, true] {
			for new_reversed in [false, true] {
				let (mut graph, [a, b, c]) = graph_with_nodes([(0.0, 0.0), (2.0, 0.0), (1.0, c_y)]);
				if wall_reversed {
					add_single_wall(&mut graph, b, a);
				} else {
					add_single_wall(&mut graph, a, b);
				}
				let m = graph.add_node(Vec2::new(1.0, 0.0)).unwrap();

				let (from, to) = if new_reversed { (c, m) } else { (m, c) };
				let added = graph.add_wall_between_nodes(from, to).unwrap();

				let context = format!(
					"c_y {c_y}, wall reversed {wall_reversed}, new reversed {new_reversed}"
				);
				assert_eq!(added.len(), 1, "{context}");
				assert_eq!(
					(added[0].origin, added[0].destination),
					(from, to),
					"{context}"
				);
				assert_eq!(
					wall_node_pairs(&graph),
					sorted_pairs(&[(a, m), (m, b), (m, c)]),
					"{context}"
				);
				assert_consistent(&graph);
				let clockwise = if c_y > 0.0 {
					vec![a, c, b]
				} else {
					vec![a, b, c]
				};
				assert_eq!(neighbours_clockwise(&graph, m, a), clockwise, "{context}");
			}
		}
	}
}

#[test]
fn a_wall_between_two_walls_splits_both() {
	//  A---P---B
	//      |
	//  C---Q---D
	let (mut graph, [a, b, c, d]) =
		graph_with_nodes([(0.0, 1.0), (2.0, 1.0), (0.0, 0.0), (2.0, 0.0)]);
	add_single_wall(&mut graph, a, b);
	add_single_wall(&mut graph, c, d);
	let p = graph.add_node(Vec2::new(1.0, 1.0)).unwrap();
	let q = graph.add_node(Vec2::new(0.5, 0.0)).unwrap();

	add_single_wall(&mut graph, q, p);

	assert_eq!(
		wall_node_pairs(&graph),
		sorted_pairs(&[(a, p), (p, b), (c, q), (q, d), (p, q)])
	);
	assert_consistent(&graph);
}

#[test]
fn a_wall_from_one_side_of_a_room_to_the_other_divides_it_in_two() {
	//  D---Q---C
	//  |   |   |
	//  A---P---B
	let (mut graph, [a, b, c, d]) =
		graph_with_nodes([(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0)]);
	add_single_wall(&mut graph, a, b);
	add_single_wall(&mut graph, b, c);
	add_single_wall(&mut graph, c, d);
	add_single_wall(&mut graph, d, a);
	let p = graph.add_node(Vec2::new(1.0, 0.0)).unwrap();
	let q = graph.add_node(Vec2::new(1.0, 2.0)).unwrap();

	add_single_wall(&mut graph, p, q);

	assert_consistent(&graph);
	assert_eq!(walk(&graph, a, p), vec![(a, p), (p, q), (q, d), (d, a)]);
	assert_eq!(walk(&graph, p, b), vec![(p, b), (b, c), (c, q), (q, p)]);
	assert_eq!(
		walk(&graph, b, p),
		vec![(b, p), (p, a), (a, d), (d, q), (q, c), (c, b)]
	);
}

#[test]
fn an_end_just_off_a_wall_within_tolerance_still_splits_it() {
	let (mut graph, [a, b, c]) = graph_with_nodes([(0.0, 0.0), (2.0, 0.0), (1.0, 1.0)]);
	add_single_wall(&mut graph, a, b);
	let m = graph
		.add_node(Vec2::new(1.0, DISTANCE_TOLERANCE / 2.0))
		.unwrap();

	add_single_wall(&mut graph, m, c);

	assert_eq!(graph.walls().count(), 3);
	assert_consistent(&graph);
}

#[test]
fn an_end_near_but_not_on_a_wall_does_not_split_it() {
	for (x, y, why) in [
		(1.0, 0.01, "just above it"),
		(3.0, 0.0, "on its line past its end"),
		(-1.0, 0.0, "on its line before its start"),
		(
			2.0 - DISTANCE_TOLERANCE / 2.0,
			0.0,
			"within tolerance of its end",
		),
	] {
		let (mut graph, [a, b, m, c]) =
			graph_with_nodes([(0.0, 0.0), (2.0, 0.0), (x, y), (x, y + 1.0)]);
		add_single_wall(&mut graph, a, b);

		add_single_wall(&mut graph, m, c);

		assert_eq!(
			wall_node_pairs(&graph),
			sorted_pairs(&[(a, b), (m, c)]),
			"{why}"
		);
		assert_consistent(&graph);
	}
}
