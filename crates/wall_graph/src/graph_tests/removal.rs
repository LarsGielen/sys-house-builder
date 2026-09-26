use glam::Vec2;

use super::support::{
	add_single_wall, add_walls, assert_consistent, find_edge, flip_combinations, graph_with_nodes,
	neighbours_clockwise, node_at, permutations, walk, wall_between, wall_node_pairs,
};
use crate::WallError;
use crate::graph::Wall;

fn reversed(wall: Wall) -> Wall {
	Wall {
		forward: wall.backward,
		backward: wall.forward,
		origin: wall.destination,
		destination: wall.origin,
	}
}

#[test]
fn removing_the_only_wall_removes_both_its_nodes() {
	let (mut graph, [a, b]) = graph_with_nodes([(0.0, 0.0), (1.0, 0.0)]);
	let wall = add_single_wall(&mut graph, a, b);

	graph.remove_wall(wall).unwrap();

	assert!(graph.edges.is_empty());
	assert_eq!(graph.walls().count(), 0);
	assert_eq!(graph.nodes().count(), 0);
	assert_consistent(&graph);
}

#[test]
fn removing_a_wall_keeps_an_end_that_other_walls_still_reach() {
	// A---B---C   removing A-B leaves B, which still has B-C
	let (mut graph, [a, b, c]) = graph_with_nodes([(0.0, 0.0), (1.0, 0.0), (2.0, 1.0)]);
	let wall = add_single_wall(&mut graph, a, b);
	add_single_wall(&mut graph, b, c);

	graph.remove_wall(wall).unwrap();

	assert_eq!(graph.node_position(a), None);
	assert_eq!(graph.node_position(b), Some(Vec2::new(1.0, 0.0)));
	assert_eq!(wall_node_pairs(&graph), vec![(b, c)]);
	assert_consistent(&graph);
}

#[test]
fn removing_a_wall_leaves_nodes_that_were_never_on_it() {
	let (mut graph, [a, b, loose]) = graph_with_nodes([(0.0, 0.0), (1.0, 0.0), (5.0, 5.0)]);
	let wall = add_single_wall(&mut graph, a, b);

	graph.remove_wall(wall).unwrap();

	let nodes: Vec<_> = graph.nodes().map(|(id, _)| id).collect();
	assert_eq!(nodes, vec![loose]);
	assert_consistent(&graph);
}

#[test]
fn a_removed_node_can_no_longer_be_used() {
	let (mut graph, [a, b, c]) = graph_with_nodes([(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)]);
	let wall = add_single_wall(&mut graph, a, b);
	add_single_wall(&mut graph, a, c);

	graph.remove_wall(wall).unwrap();

	assert_eq!(graph.node_position(b), None);
	assert_eq!(graph.add_wall(a, b), Err(WallError::UnknownNode));
	assert_eq!(graph.add_wall(b, c), Err(WallError::UnknownNode));
	assert_consistent(&graph);
}

#[test]
fn a_crossing_node_goes_once_its_last_wall_is_removed() {
	//        N
	//        |
	//   W ---X--- E    X is made by adding S-N across W-E
	//        |
	//        S
	for order in permutations(vec![0, 1, 2, 3]) {
		let (mut graph, [w, e, s, n]) =
			graph_with_nodes([(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)]);
		add_single_wall(&mut graph, w, e);
		graph.add_wall(s, n).unwrap();
		let x = node_at(&graph, Vec2::ZERO);
		let arms = [w, n, e, s];

		for (step, &i) in order.iter().enumerate() {
			graph.remove_wall(wall_between(&graph, x, arms[i])).unwrap();

			assert_consistent(&graph);
			assert_eq!(graph.node_position(arms[i]), None, "order {order:?}");
			let last = step == order.len() - 1;
			assert_eq!(
				graph.node_position(x).is_none(),
				last,
				"order {order:?}, step {step}"
			);
		}
		assert_eq!(graph.nodes().count(), 0);
	}
}

#[test]
fn a_wall_can_be_removed_through_either_direction() {
	let (mut graph, [a, b]) = graph_with_nodes([(0.0, 0.0), (1.0, 0.0)]);
	let wall = add_single_wall(&mut graph, a, b);

	graph.remove_wall(reversed(wall)).unwrap();

	assert!(graph.edges.is_empty());
	assert_consistent(&graph);
}

#[test]
fn removing_an_arm_of_a_crossing_keeps_the_others_in_order() {
	//        N
	//        |
	//   W ---C--- E
	//        |
	//        S
	for removed in 0..4 {
		for order in permutations(vec![0, 1, 2, 3]) {
			for flipped in flip_combinations(4) {
				let (mut graph, [c, w, n, e, s]) = graph_with_nodes([
					(0.0, 0.0),
					(-1.0, 0.0),
					(0.0, 1.0),
					(1.0, 0.0),
					(0.0, -1.0),
				]);
				let arms = [(c, w), (c, n), (c, e), (c, s)];
				add_walls(&mut graph, &arms, &order, &flipped);

				let (from, to) = arms[removed];
				graph.remove_wall(wall_between(&graph, from, to)).unwrap();

				assert_consistent(&graph);
				let clockwise = [w, n, e, s];
				let remaining: Vec<_> = (1..4).map(|i| clockwise[(removed + i) % 4]).collect();
				assert_eq!(
					neighbours_clockwise(&graph, c, remaining[0]),
					remaining,
					"removed {removed}, insertion order {order:?}, flipped {flipped:?}"
				);
				assert_eq!(graph.node_position(clockwise[removed]), None);
			}
		}
	}
}

#[test]
fn removing_a_side_of_a_triangle_merges_inside_and_outside() {
	//  C
	//  | \
	//  A--B
	for side in 0..3 {
		let (mut graph, [a, b, c]) = graph_with_nodes([(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)]);
		let walls = [
			add_single_wall(&mut graph, a, b),
			add_single_wall(&mut graph, b, c),
			add_single_wall(&mut graph, c, a),
		];

		graph.remove_wall(walls[side]).unwrap();

		assert_consistent(&graph);
		// What is left is an open path p-q-r, so a single loop runs out along one side and back along the other.
		let [p, q, r] = [[b, c, a], [c, a, b], [a, b, c]][side];
		assert_eq!(
			walk(&graph, p, q),
			vec![(p, q), (q, r), (r, q), (q, p)],
			"removed side {side}"
		);
	}
}

#[test]
fn removing_the_middle_of_a_path_leaves_two_separate_walls() {
	let (mut graph, [a, b, c, d]) =
		graph_with_nodes([(0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (3.0, 0.0)]);
	add_single_wall(&mut graph, a, b);
	let middle = add_single_wall(&mut graph, b, c);
	add_single_wall(&mut graph, c, d);

	graph.remove_wall(middle).unwrap();

	assert_consistent(&graph);
	assert_eq!(walk(&graph, a, b), vec![(a, b), (b, a)]);
	assert_eq!(walk(&graph, c, d), vec![(c, d), (d, c)]);
}

#[test]
fn a_node_whose_outgoing_edge_is_removed_points_at_a_remaining_one() {
	let (mut graph, [a, b, c]) = graph_with_nodes([(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)]);
	add_single_wall(&mut graph, a, b);
	let later = add_single_wall(&mut graph, a, c);
	assert_eq!(graph.node(a).outgoing_edge, Some(later.forward));

	graph.remove_wall(later).unwrap();

	assert_eq!(graph.node(a).outgoing_edge, Some(find_edge(&graph, a, b)));
	assert_consistent(&graph);
}

#[test]
fn removing_every_wall_of_a_square_in_any_order_empties_it() {
	for order in permutations(vec![0, 1, 2, 3]) {
		let (mut graph, [a, b, c, d]) =
			graph_with_nodes([(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]);
		let walls = [
			add_single_wall(&mut graph, a, b),
			add_single_wall(&mut graph, b, c),
			add_single_wall(&mut graph, c, d),
			add_single_wall(&mut graph, d, a),
		];

		for &i in &order {
			graph.remove_wall(walls[i]).unwrap();
			assert_consistent(&graph);
		}

		assert!(graph.edges.is_empty(), "order {order:?}");
		assert_eq!(graph.nodes().count(), 0, "order {order:?}");
	}
}

#[test]
fn removing_a_wall_twice_is_rejected_and_changes_nothing() {
	let (mut graph, [a, b, c]) = graph_with_nodes([(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)]);
	let wall = add_single_wall(&mut graph, a, b);
	add_single_wall(&mut graph, a, c);
	graph.remove_wall(wall).unwrap();

	assert_eq!(graph.remove_wall(wall), Err(WallError::UnknownWall));
	assert_eq!(wall_node_pairs(&graph), vec![(a, c)]);
	assert_consistent(&graph);
}

#[test]
fn a_wall_that_does_not_match_the_graph_is_rejected() {
	let (mut graph, [a, b, c]) = graph_with_nodes([(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)]);
	let first = add_single_wall(&mut graph, a, b);
	let second = add_single_wall(&mut graph, a, c);

	let forged = [
		Wall {
			forward: first.forward,
			backward: second.backward,
			..first
		},
		Wall {
			origin: first.destination,
			..first
		},
		Wall {
			destination: c,
			..first
		},
		Wall {
			forward: first.backward,
			backward: first.forward,
			..first
		},
	];
	for wall in forged {
		assert_eq!(
			graph.remove_wall(wall),
			Err(WallError::UnknownWall),
			"{wall:?}"
		);
	}
	assert_eq!(graph.walls().count(), 2);
	assert_consistent(&graph);
}

#[test]
fn a_removed_wall_can_be_added_again_with_new_ids() {
	let (mut graph, [a, b, c]) = graph_with_nodes([(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)]);
	add_single_wall(&mut graph, a, c);
	let old = add_single_wall(&mut graph, a, b);
	graph.remove_wall(old).unwrap();

	let b_again = graph.add_node(Vec2::new(1.0, 0.0)).unwrap();
	let new = add_single_wall(&mut graph, b_again, a);

	assert_ne!(b_again, b);

	assert_ne!(new.forward, old.forward);
	assert_ne!(new.forward, old.backward);
	assert_ne!(new.backward, old.forward);
	assert_ne!(new.backward, old.backward);
	// The stale handle must not remove the new wall.
	assert_eq!(graph.remove_wall(old), Err(WallError::UnknownWall));
	assert_eq!(wall_node_pairs(&graph), vec![(a, c), (a, b_again)]);
	assert_consistent(&graph);
}
