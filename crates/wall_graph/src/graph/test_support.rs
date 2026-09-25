use glam::Vec2;

use super::WallGraph;
use crate::ids::{HalfEdgeId, WallNodeId};

/// Builds a graph with one node per position and returns the node IDs in the same order.
pub(super) fn graph_with_nodes<const N: usize>(
	positions: [(f32, f32); N],
) -> (WallGraph, [WallNodeId; N]) {
	let mut graph = WallGraph::new();
	let ids = positions.map(|(x, y)| graph.add_node(Vec2::new(x, y)));
	(graph, ids)
}

pub(super) fn find_edge(graph: &WallGraph, from: WallNodeId, to: WallNodeId) -> HalfEdgeId {
	*graph
		.edges
		.iter()
		.find(|(id, edge)| edge.origin == from && graph.edge_destination(**id) == to)
		.map(|(id, _)| id)
		.expect("no half-edge found between the given nodes")
}

pub(super) fn assert_consistent(graph: &WallGraph) {
	if let Err(violation) = graph.validate() {
		panic!("graph invariant violated: {violation}");
	}
}

pub(super) fn permutations(items: Vec<usize>) -> Vec<Vec<usize>> {
	if items.len() <= 1 {
		return vec![items];
	}
	let mut result = Vec::new();
	for i in 0..items.len() {
		let mut rest = items.clone();
		let picked = rest.remove(i);
		for mut tail in permutations(rest) {
			tail.insert(0, picked);
			result.push(tail);
		}
	}
	result
}

/// Every way of choosing `true` or `false` for `count` items.
pub(super) fn flip_combinations(count: usize) -> Vec<Vec<bool>> {
	(0..1usize << count)
		.map(|bits| (0..count).map(|i| (bits >> i) & 1 == 1).collect())
		.collect()
}

/// Follows `next` from the half-edge `from -> to` until it returns, listing each half-edge as (origin, destination).
pub(super) fn walk(
	graph: &WallGraph,
	from: WallNodeId,
	to: WallNodeId,
) -> Vec<(WallNodeId, WallNodeId)> {
	let start = find_edge(graph, from, to);
	let mut cycle = Vec::new();
	let mut current = start;
	loop {
		cycle.push((graph.edge(current).origin, graph.edge_destination(current)));
		current = graph.edge(current).next;
		assert!(
			cycle.len() <= graph.edges.len(),
			"next never returned to the start edge"
		);
		if current == start {
			break;
		}
	}
	cycle
}

/// Adds the walls in the given order; wall `i` is added as (to, from) instead of (from, to) when `flipped[i]` is set.
pub(super) fn add_walls(
	graph: &mut WallGraph,
	walls: &[(WallNodeId, WallNodeId)],
	order: &[usize],
	flipped: &[bool],
) {
	for &i in order {
		let (from, to) = walls[i];
		let result = if flipped[i] {
			graph.add_wall(to, from)
		} else {
			graph.add_wall(from, to)
		};
		result.unwrap();
	}
}
