use glam::Vec2;

use super::super::{HalfEdgeId, Wall, WallError, WallGraph, WallNodeId};
use crate::geometry::CurveShape;

/// Adds a wall that is expected not to meet any other wall, so it comes back whole.
pub(super) fn add_single_wall(graph: &mut WallGraph, from: WallNodeId, to: WallNodeId) -> Wall {
	let walls = graph.add_wall_between_nodes(from, to).unwrap();
	assert_eq!(walls.len(), 1, "wall was unexpectedly split: {walls:?}");
	walls[0]
}

/// Every wall as its pair of nodes, normalised like `sorted_pairs`, so graphs can be compared regardless of wall direction.
pub(super) fn wall_node_pairs(graph: &WallGraph) -> Vec<(WallNodeId, WallNodeId)> {
	let walls: Vec<_> = graph
		.walls()
		.map(|wall| (wall.origin, wall.destination))
		.collect();
	sorted_pairs(&walls)
}

/// Node pairs with the smaller ID first, in sorted order.
pub(super) fn sorted_pairs(pairs: &[(WallNodeId, WallNodeId)]) -> Vec<(WallNodeId, WallNodeId)> {
	let mut sorted: Vec<_> = pairs.iter().map(|&(a, b)| (a.min(b), a.max(b))).collect();
	sorted.sort();
	sorted
}

/// The wall joining two nodes, in whichever direction it was added.
pub(super) fn wall_between(graph: &WallGraph, a: WallNodeId, b: WallNodeId) -> Wall {
	graph
		.walls()
		.find(|wall| {
			(wall.origin, wall.destination) == (a, b) || (wall.origin, wall.destination) == (b, a)
		})
		.expect("no wall between the given nodes")
}

/// The single node at `position`, within a small tolerance.
pub(super) fn node_at(graph: &WallGraph, position: Vec2) -> WallNodeId {
	let matches: Vec<_> = graph
		.nodes()
		.filter(|(_, node_position)| node_position.distance(position) < 1e-4)
		.map(|(id, _)| id)
		.collect();
	assert_eq!(
		matches.len(),
		1,
		"expected one node at {position}, found {matches:?}"
	);
	matches[0]
}

/// The walls leaving a node in clockwise order, as their far nodes, starting from the one toward `first`.
pub(super) fn neighbours_clockwise(
	graph: &WallGraph,
	node_id: WallNodeId,
	first: WallNodeId,
) -> Vec<WallNodeId> {
	let start = find_edge(graph, node_id, first);
	let mut neighbours = vec![first];
	let mut current = graph.next_outgoing_half_edge_clockwise(start);
	while current != start {
		neighbours.push(graph.half_edge_destination(current));
		current = graph.next_outgoing_half_edge_clockwise(current);
	}
	neighbours
}

/// Builds a graph with one node per position and returns the node IDs in the same order.
pub(super) fn graph_with_nodes<const N: usize>(
	positions: [(f32, f32); N],
) -> (WallGraph, [WallNodeId; N]) {
	let mut graph = WallGraph::new();
	let ids = positions.map(|(x, y)| graph.add_node(Vec2::new(x, y)).unwrap());
	(graph, ids)
}

pub(super) fn find_edge(graph: &WallGraph, from: WallNodeId, to: WallNodeId) -> HalfEdgeId {
	*graph
		.edges
		.iter()
		.find(|(id, edge)| edge.origin == from && graph.half_edge_destination(**id) == to)
		.map(|(id, _)| id)
		.expect("no half-edge found between the given nodes")
}

pub(super) fn assert_consistent(graph: &WallGraph) {
	if let Err(violation) = graph.validate_topology() {
		panic!("graph invariant violated: {violation}");
	}
	if let Err(violation) = graph.validate_geometry() {
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
		cycle.push((
			graph.edge(current).origin,
			graph.half_edge_destination(current),
		));
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
			graph.add_wall_between_nodes(to, from)
		} else {
			graph.add_wall_between_nodes(from, to)
		};
		result.unwrap();
	}
}

// These helpers exercise the internal planner with pre-existing nodes, including deliberately
// isolated nodes that callers cannot create through the public API.
impl WallGraph {
	pub(super) fn add_node(&mut self, position: Vec2) -> Result<WallNodeId, WallError> {
		if !position.is_finite() {
			return Err(WallError::InvalidPosition);
		}
		Ok(self.insert_node(position))
	}

	pub(super) fn add_wall_between_nodes(
		&mut self,
		origin: WallNodeId,
		destination: WallNodeId,
	) -> Result<Vec<Wall>, WallError> {
		self.add_curve(origin, destination, CurveShape::Straight)
	}

	pub(super) fn add_arc_between_nodes(
		&mut self,
		origin: WallNodeId,
		destination: WallNodeId,
		sweep: f32,
	) -> Result<Vec<Wall>, WallError> {
		if !sweep.is_finite()
			|| sweep.abs() <= super::super::DIRECTION_ANGLE_TOLERANCE
			|| sweep.abs() >= std::f32::consts::TAU
		{
			return Err(WallError::InvalidArc);
		}
		self.add_curve(
			origin,
			destination,
			CurveShape::CircularArc {
				sweep: sweep as f64,
			},
		)
	}
}
