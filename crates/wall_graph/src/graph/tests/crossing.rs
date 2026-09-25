use glam::Vec2;

use crate::WallError;
use crate::graph::DISTANCE_TOLERANCE;
use crate::graph::test_support::{
	add_single_wall, assert_consistent, graph_with_nodes, neighbours_clockwise, node_at,
	permutations, sorted_pairs, walk, wall_node_pairs,
};

#[test]
fn crossing_a_wall_makes_a_four_way_junction() {
	//        N
	//        |
	//   W ---X--- E
	//        |
	//        S
	for existing_reversed in [false, true] {
		for new_reversed in [false, true] {
			let (mut graph, [w, e, s, n]) =
				graph_with_nodes([(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)]);
			if existing_reversed {
				add_single_wall(&mut graph, e, w);
			} else {
				add_single_wall(&mut graph, w, e);
			}

			let (from, to) = if new_reversed { (n, s) } else { (s, n) };
			let added = graph.add_wall(from, to).unwrap();

			let context =
				format!("existing reversed {existing_reversed}, new reversed {new_reversed}");
			let x = node_at(&graph, Vec2::ZERO);
			let pieces: Vec<_> = added.iter().map(|w| (w.origin, w.destination)).collect();
			assert_eq!(pieces, vec![(from, x), (x, to)], "{context}");
			assert_eq!(
				wall_node_pairs(&graph),
				sorted_pairs(&[(w, x), (x, e), (s, x), (x, n)]),
				"{context}"
			);
			assert_eq!(
				neighbours_clockwise(&graph, x, w),
				vec![w, n, e, s],
				"{context}"
			);
			assert_consistent(&graph);
		}
	}
}

#[test]
fn crossing_several_walls_breaks_the_new_wall_at_each_in_order() {
	//  |   |   |
	//  A---+---+---+---B
	//  |   |   |
	for reversed in [false, true] {
		let (mut graph, [a, b]) = graph_with_nodes([(0.0, 0.0), (4.0, 0.0)]);
		for x in [3.0, 1.0, 2.0] {
			let bottom = graph.add_node(Vec2::new(x, -1.0)).unwrap();
			let top = graph.add_node(Vec2::new(x, 1.0)).unwrap();
			add_single_wall(&mut graph, bottom, top);
		}

		let (from, to) = if reversed { (b, a) } else { (a, b) };
		let added = graph.add_wall(from, to).unwrap();

		let crossings = [1.0, 2.0, 3.0].map(|x| node_at(&graph, Vec2::new(x, 0.0)));
		let mut path = [a, crossings[0], crossings[1], crossings[2], b];
		if reversed {
			path.reverse();
		}
		let pieces: Vec<_> = added.iter().map(|w| (w.origin, w.destination)).collect();
		let expected: Vec<_> = path.windows(2).map(|pair| (pair[0], pair[1])).collect();
		assert_eq!(pieces, expected, "reversed {reversed}");
		assert_eq!(graph.walls().count(), 3 * 2 + 4);
		assert_consistent(&graph);
	}
}

#[test]
fn a_grid_of_crossing_walls_is_the_same_in_any_order() {
	// Three horizontal and three vertical walls crossing at nine points.
	let lines = [
		((0.0, 1.0), (4.0, 1.0)),
		((0.0, 2.0), (4.0, 2.0)),
		((0.0, 3.0), (4.0, 3.0)),
		((1.0, 0.0), (1.0, 4.0)),
		((2.0, 0.0), (2.0, 4.0)),
		((3.0, 0.0), (3.0, 4.0)),
	];
	for order in permutations(vec![0, 1, 2, 3, 4, 5]) {
		let mut graph = crate::WallGraph::new();
		let ends: Vec<_> = lines
			.iter()
			.map(|&((x1, y1), (x2, y2))| {
				(
					graph.add_node(Vec2::new(x1, y1)).unwrap(),
					graph.add_node(Vec2::new(x2, y2)).unwrap(),
				)
			})
			.collect();
		for &i in &order {
			graph.add_wall(ends[i].0, ends[i].1).unwrap();
		}

		assert_consistent(&graph);
		assert_eq!(graph.nodes().count(), 12 + 9, "order {order:?}");
		assert_eq!(graph.walls().count(), 6 * 4, "order {order:?}");
		for x in [1.0, 2.0, 3.0] {
			for y in [1.0, 2.0, 3.0] {
				let centre = node_at(&graph, Vec2::new(x, y));
				let west = node_at(&graph, Vec2::new(x - 1.0, y));
				let north = node_at(&graph, Vec2::new(x, y + 1.0));
				let east = node_at(&graph, Vec2::new(x + 1.0, y));
				let south = node_at(&graph, Vec2::new(x, y - 1.0));
				assert_eq!(
					neighbours_clockwise(&graph, centre, west),
					vec![west, north, east, south],
					"at ({x}, {y}), order {order:?}"
				);
			}
		}
	}
}

#[test]
fn two_walls_across_a_room_divide_it_into_four() {
	//  D---+---C
	//  |   |   |
	//  +---X---+
	//  |   |   |
	//  A---+---B
	let (mut graph, [a, b, c, d]) =
		graph_with_nodes([(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0)]);
	for (from, to) in [(a, b), (b, c), (c, d), (d, a)] {
		add_single_wall(&mut graph, from, to);
	}
	let [south, north, west, east] = [(1.0, 0.0), (1.0, 2.0), (0.0, 1.0), (2.0, 1.0)]
		.map(|(x, y)| graph.add_node(Vec2::new(x, y)).unwrap());

	graph.add_wall(south, north).unwrap();
	graph.add_wall(west, east).unwrap();

	assert_consistent(&graph);
	let x = node_at(&graph, Vec2::new(1.0, 1.0));
	assert_eq!(
		walk(&graph, a, south),
		vec![(a, south), (south, x), (x, west), (west, a)]
	);
	assert_eq!(
		walk(&graph, south, b),
		vec![(south, b), (b, east), (east, x), (x, south)]
	);
	assert_eq!(
		walk(&graph, east, c),
		vec![(east, c), (c, north), (north, x), (x, east)]
	);
	assert_eq!(
		walk(&graph, north, d),
		vec![(north, d), (d, west), (west, x), (x, north)]
	);
	assert_eq!(
		walk(&graph, south, a),
		vec![
			(south, a),
			(a, west),
			(west, d),
			(d, north),
			(north, c),
			(c, east),
			(east, b),
			(b, south)
		],
		"outside"
	);
}

#[test]
fn a_wall_through_an_existing_junction_joins_it_without_a_new_node() {
	//        N
	//        |
	//   W ---J--- E     J already has a wall to N; W-E passes straight through J
	let (mut graph, [w, e, j, n]) =
		graph_with_nodes([(-1.0, 0.0), (1.0, 0.0), (0.0, 0.0), (0.0, 1.0)]);
	add_single_wall(&mut graph, j, n);

	let added = graph.add_wall(w, e).unwrap();

	let pieces: Vec<_> = added.iter().map(|w| (w.origin, w.destination)).collect();
	assert_eq!(pieces, vec![(w, j), (j, e)]);
	assert_eq!(graph.nodes().count(), 4);
	assert_eq!(neighbours_clockwise(&graph, j, w), vec![w, n, e]);
	assert_consistent(&graph);
}

#[test]
fn a_wall_through_the_centre_of_a_crossing_uses_its_node() {
	// A diagonal through the middle of an existing + makes an eight-way junction at its centre.
	let (mut graph, [c, w, n, e, s, sw, ne]) = graph_with_nodes([
		(0.0, 0.0),
		(-1.0, 0.0),
		(0.0, 1.0),
		(1.0, 0.0),
		(0.0, -1.0),
		(-1.0, -1.0),
		(1.0, 1.0),
	]);
	for arm in [w, n, e, s] {
		add_single_wall(&mut graph, c, arm);
	}

	let added = graph.add_wall(sw, ne).unwrap();

	assert_eq!(added.len(), 2);
	assert_eq!(graph.nodes().count(), 7);
	assert_eq!(neighbours_clockwise(&graph, c, w), vec![w, n, ne, e, s, sw]);
	assert_consistent(&graph);
}

#[test]
fn a_wall_through_an_isolated_node_picks_it_up() {
	let (mut graph, [a, m, b]) = graph_with_nodes([(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)]);

	let added = graph.add_wall(a, b).unwrap();

	let pieces: Vec<_> = added.iter().map(|w| (w.origin, w.destination)).collect();
	assert_eq!(pieces, vec![(a, m), (m, b)]);
	assert_consistent(&graph);
}

#[test]
fn an_isolated_node_at_a_crossing_becomes_the_junction() {
	// The centre node predates both walls. Crossing must reuse it rather than add a coincident node.
	let (mut graph, [west, east]) = graph_with_nodes([(-1.0, 0.0), (1.0, 0.0)]);
	add_single_wall(&mut graph, west, east);
	let centre = graph.add_node(Vec2::ZERO).unwrap();
	let south = graph.add_node(Vec2::new(0.0, -1.0)).unwrap();
	let north = graph.add_node(Vec2::new(0.0, 1.0)).unwrap();

	let added = graph.add_wall(south, north).unwrap();

	assert_eq!(graph.nodes().count(), 5);
	assert_eq!(
		added
			.into_iter()
			.map(|wall| (wall.origin, wall.destination))
			.collect::<Vec<_>>(),
		vec![(south, centre), (centre, north)]
	);
	assert_eq!(
		wall_node_pairs(&graph),
		sorted_pairs(&[
			(west, centre),
			(centre, east),
			(south, centre),
			(centre, north),
		])
	);
	assert_consistent(&graph);
}

#[test]
fn nearby_nodes_on_a_new_wall_are_one_breakpoint() {
	let (mut graph, [start, first, second, end]) = graph_with_nodes([
		(0.0, 0.0),
		(1.0, 0.0),
		(1.0 + DISTANCE_TOLERANCE / 2.0, 0.0),
		(2.0, 0.0),
	]);

	let added = graph.add_wall(start, end).unwrap();

	assert_eq!(
		added
			.into_iter()
			.map(|wall| (wall.origin, wall.destination))
			.collect::<Vec<_>>(),
		vec![(start, first), (first, end)]
	);
	assert!(graph.walls().all(|wall| {
		wall.origin != second
			&& wall.destination != second
			&& graph
				.node_position(wall.origin)
				.unwrap()
				.distance(graph.node_position(wall.destination).unwrap())
				> DISTANCE_TOLERANCE
	}));
	assert_consistent(&graph);
}

#[test]
fn a_crossing_that_grazes_a_wall_end_joins_at_that_node() {
	// The wall C-D ends just short of (within tolerance of) the new wall, so the new wall runs through C instead of adding a node beside it.
	let offset = DISTANCE_TOLERANCE / 2.0;
	let (mut graph, [c, d, w, e]) =
		graph_with_nodes([(0.0, offset), (0.0, 1.0), (-1.0, 0.0), (1.0, 0.0)]);
	add_single_wall(&mut graph, c, d);

	let added = graph.add_wall(w, e).unwrap();

	let pieces: Vec<_> = added.iter().map(|w| (w.origin, w.destination)).collect();
	assert_eq!(pieces, vec![(w, c), (c, e)]);
	assert_eq!(graph.nodes().count(), 4);
	assert_consistent(&graph);
}

#[test]
fn walls_crossing_at_a_shallow_angle_still_meet_at_a_node() {
	let (mut graph, [a, b, c, d]) =
		graph_with_nodes([(0.0, 0.0), (10.0, 0.0), (0.0, -0.1), (10.0, 0.1)]);
	add_single_wall(&mut graph, a, b);

	let added = graph.add_wall(c, d).unwrap();

	assert_eq!(added.len(), 2);
	let x = node_at(&graph, Vec2::new(5.0, 0.0));
	assert_eq!(added[0].destination, x);
	assert_eq!(graph.walls().count(), 4);
	assert_consistent(&graph);
}

#[test]
fn a_wall_ending_on_one_wall_and_crossing_another_splits_both() {
	//  A-------M-------B      the new wall runs from M, on A-B, down across C-D to S
	//          |
	//  C-------X-------D
	//          |
	//          S
	let (mut graph, [a, b, c, d, s]) =
		graph_with_nodes([(0.0, 2.0), (2.0, 2.0), (0.0, 1.0), (2.0, 1.0), (1.0, 0.0)]);
	add_single_wall(&mut graph, a, b);
	add_single_wall(&mut graph, c, d);
	let m = graph.add_node(Vec2::new(1.0, 2.0)).unwrap();

	let added = graph.add_wall(m, s).unwrap();

	let x = node_at(&graph, Vec2::new(1.0, 1.0));
	let pieces: Vec<_> = added.iter().map(|w| (w.origin, w.destination)).collect();
	assert_eq!(pieces, vec![(m, x), (x, s)]);
	assert_eq!(
		wall_node_pairs(&graph),
		sorted_pairs(&[(a, m), (m, b), (c, x), (x, d), (m, x), (x, s)])
	);
	assert_consistent(&graph);
}

#[test]
fn collinear_walls_that_only_touch_or_are_apart_are_accepted() {
	// A---B  C---D   and then B-C fills the gap end to end
	let (mut graph, [a, b, c, d]) =
		graph_with_nodes([(0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (3.0, 0.0)]);
	add_single_wall(&mut graph, a, b);
	add_single_wall(&mut graph, c, d);
	add_single_wall(&mut graph, b, c);

	assert_eq!(graph.walls().count(), 3);
	assert_consistent(&graph);
}

#[test]
fn a_wall_along_part_of_an_unconnected_wall_is_rejected_and_changes_nothing() {
	// A=======C        existing
	//      B=======D   new, sharing the stretch B..C but no node with A-C
	for (from, to, why) in [
		((1.0, 0.0), (3.0, 0.0), "partly overlapping"),
		((0.5, 0.0), (1.5, 0.0), "inside it"),
		((-1.0, 0.0), (3.0, 0.0), "covering it"),
	] {
		let (mut graph, [a, c]) = graph_with_nodes([(0.0, 0.0), (2.0, 0.0)]);
		let existing = add_single_wall(&mut graph, a, c);
		let b = graph.add_node(Vec2::new(from.0, from.1)).unwrap();
		let d = graph.add_node(Vec2::new(to.0, to.1)).unwrap();

		assert_eq!(graph.add_wall(b, d), Err(WallError::Overlapping), "{why}");
		assert_eq!(
			graph.add_wall(d, b),
			Err(WallError::Overlapping),
			"{why}, reversed"
		);
		assert_eq!(graph.walls().collect::<Vec<_>>(), vec![existing], "{why}");
		assert_eq!(graph.nodes().count(), 4, "{why}");
		assert_consistent(&graph);
	}
}

#[test]
fn a_rejected_wall_does_not_split_the_walls_it_would_have_crossed() {
	//      |           the new wall crosses the vertical wall first, then overlaps P-Q
	//  A---+---P===Q
	//      |
	let (mut graph, [bottom, top, p, q]) =
		graph_with_nodes([(1.0, -1.0), (1.0, 1.0), (3.0, 0.0), (5.0, 0.0)]);
	add_single_wall(&mut graph, bottom, top);
	add_single_wall(&mut graph, p, q);
	let a = graph.add_node(Vec2::new(0.0, 0.0)).unwrap();
	let far = graph.add_node(Vec2::new(4.0, 0.0)).unwrap();

	assert_eq!(graph.add_wall(a, far), Err(WallError::Overlapping));

	assert_eq!(
		wall_node_pairs(&graph),
		sorted_pairs(&[(bottom, top), (p, q)])
	);
	assert_eq!(graph.nodes().count(), 6);
	assert_consistent(&graph);
}

#[test]
fn a_wall_between_nodes_at_the_same_position_is_rejected() {
	for offset in [0.0, DISTANCE_TOLERANCE / 2.0] {
		let (mut graph, [a, b]) = graph_with_nodes([(1.0, 1.0), (1.0 + offset, 1.0)]);

		assert_eq!(
			graph.add_wall(a, b),
			Err(WallError::ZeroLength),
			"offset {offset}"
		);
		assert!(graph.edges.is_empty());
	}
}

#[test]
fn walls_that_miss_each_other_are_left_alone() {
	for (from, to, why) in [
		((3.0, -1.0), (3.0, 1.0), "past the end of the wall"),
		((1.0, 0.5), (1.0, 2.0), "stopping short of the wall"),
		((0.0, 1.0), (2.0, 1.0), "parallel to the wall"),
	] {
		let (mut graph, [a, b]) = graph_with_nodes([(0.0, 0.0), (2.0, 0.0)]);
		add_single_wall(&mut graph, a, b);
		let c = graph.add_node(Vec2::new(from.0, from.1)).unwrap();
		let d = graph.add_node(Vec2::new(to.0, to.1)).unwrap();

		let added = graph.add_wall(c, d).unwrap();

		assert_eq!(added.len(), 1, "{why}");
		assert_eq!(
			wall_node_pairs(&graph),
			sorted_pairs(&[(a, b), (c, d)]),
			"{why}"
		);
		assert_consistent(&graph);
	}
}

#[test]
fn a_removed_wall_is_no_longer_crossed() {
	let (mut graph, [w, e, s, n]) =
		graph_with_nodes([(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)]);
	let existing = add_single_wall(&mut graph, w, e);
	graph.remove_wall(existing).unwrap();

	let added = graph.add_wall(s, n).unwrap();

	assert_eq!(added.len(), 1);
	assert_eq!(graph.nodes().count(), 2);
	assert_consistent(&graph);
}
