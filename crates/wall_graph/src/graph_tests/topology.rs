use super::support::{
	add_single_wall, add_walls, assert_consistent, flip_combinations, graph_with_nodes,
	permutations, walk,
};

#[test]
fn isolated_wall_links_form_a_two_edge_loop() {
	let (mut graph, [a, b]) = graph_with_nodes([(0.0, 0.0), (1.0, 0.0)]);
	let wall = add_single_wall(&mut graph, a, b);

	assert_ne!(wall.forward, wall.backward);
	assert_eq!(graph.node(a).outgoing_edge, Some(wall.forward));
	assert_eq!(graph.node(b).outgoing_edge, Some(wall.backward));

	let forward = graph.edge(wall.forward);
	let backward = graph.edge(wall.backward);

	assert_eq!(forward.origin, a);
	assert_eq!(backward.origin, b);

	assert_eq!(forward.twin, wall.backward);
	assert_eq!(backward.twin, wall.forward);

	assert_eq!(forward.next, wall.backward);
	assert_eq!(backward.next, wall.forward);
	assert_eq!(forward.previous, wall.backward);
	assert_eq!(backward.previous, wall.forward);
}

#[test]
fn add_wall_returns_the_edges_running_each_way() {
	let (mut graph, [a, b, c]) = graph_with_nodes([(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)]);
	graph.add_wall(a, b).unwrap();
	let wall = add_single_wall(&mut graph, c, b);

	assert_eq!(graph.edge(wall.forward).origin, c);
	assert_eq!(graph.half_edge_destination(wall.forward), b);
	assert_eq!(graph.edge(wall.backward).origin, b);
	assert_eq!(graph.half_edge_destination(wall.backward), c);
	assert_eq!((wall.origin, wall.destination), (c, b));
}

#[test]
fn two_walls_sharing_a_node_form_one_four_edge_loop() {
	let (mut graph, [a, b, c]) = graph_with_nodes([(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)]);
	graph.add_wall(a, b).unwrap();
	graph.add_wall(b, c).unwrap();

	assert_consistent(&graph);

	// A->B, turn onto B->C, tip at C, back C->B, turn onto B->A, tip at A.
	assert_eq!(walk(&graph, a, b), vec![(a, b), (b, c), (c, b), (b, a)]);
}

#[test]
fn joining_two_existing_walls_forms_one_six_edge_loop() {
	let (mut graph, [a, b, c, d]) =
		graph_with_nodes([(0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (3.0, 0.0)]);
	graph.add_wall(a, b).unwrap();
	graph.add_wall(c, d).unwrap();
	graph.add_wall(b, c).unwrap();

	assert_consistent(&graph);

	// Path A-B-C-D: out along one side, tip at D, back along the other, tip at A.
	assert_eq!(
		walk(&graph, a, b),
		vec![(a, b), (b, c), (c, d), (d, c), (c, b), (b, a)]
	);
}

#[test]
fn t_junction_orders_walls_by_angle_regardless_of_insertion_order_and_direction() {
	//        D (0,1)
	//        |
	// A -----B----- C      B has three walls: west (A), north (D), east (C)
	for order in permutations(vec![0, 1, 2]) {
		for flipped in flip_combinations(3) {
			let (mut graph, [a, b, c, d]) =
				graph_with_nodes([(-1.0, 0.0), (0.0, 0.0), (1.0, 0.0), (0.0, 1.0)]);
			add_walls(&mut graph, &[(a, b), (b, c), (b, d)], &order, &flipped);

			assert_consistent(&graph);

			// Face on the left: at B, arriving from A you turn left onto D; from D onto C; from C onto A.
			assert_eq!(
				walk(&graph, a, b),
				vec![(a, b), (b, d), (d, b), (b, c), (c, b), (b, a)],
				"insertion order {order:?}, flipped {flipped:?}"
			);
		}
	}
}

#[test]
fn four_way_crossing_turns_left_at_every_arm_in_any_order_and_direction() {
	//        N
	//        |
	//   W ---C--- E
	//        |
	//        S
	for order in permutations(vec![0, 1, 2, 3]) {
		for flipped in flip_combinations(4) {
			let (mut graph, [c, w, n, e, s]) =
				graph_with_nodes([(0.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (1.0, 0.0), (0.0, -1.0)]);
			add_walls(
				&mut graph,
				&[(c, w), (c, n), (c, e), (c, s)],
				&order,
				&flipped,
			);

			assert_consistent(&graph);
			assert_eq!(
				walk(&graph, w, c),
				vec![
					(w, c),
					(c, n),
					(n, c),
					(c, e),
					(e, c),
					(c, s),
					(s, c),
					(c, w)
				],
				"insertion order {order:?}, flipped {flipped:?}"
			);
		}
	}
}

#[test]
fn closed_triangle_has_separate_interior_and_exterior_loops() {
	//  C
	//  | \
	//  A--B
	for order in permutations(vec![0, 1, 2]) {
		for flipped in flip_combinations(3) {
			let (mut graph, [a, b, c]) = graph_with_nodes([(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)]);
			add_walls(&mut graph, &[(a, b), (b, c), (c, a)], &order, &flipped);

			assert_consistent(&graph);
			assert_eq!(
				walk(&graph, a, b),
				vec![(a, b), (b, c), (c, a)],
				"interior, insertion order {order:?}, flipped {flipped:?}"
			);
			assert_eq!(
				walk(&graph, b, a),
				vec![(b, a), (a, c), (c, b)],
				"exterior, insertion order {order:?}, flipped {flipped:?}"
			);
		}
	}
}
