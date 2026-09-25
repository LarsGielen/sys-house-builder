use std::collections::HashMap;

use glam::Vec2;

use crate::error::WallError;
use crate::geometry::direction_angle;
use crate::ids::{HalfEdgeId, WallNodeId};

#[derive(Debug, Clone, Copy)]
struct WallNode {
	position: Vec2,
	outgoing_edge: Option<HalfEdgeId>,
}

/// Faces lie to the left of each half-edge (interior is counter-clockwise).
/// `next(e)` is the first edge leaving `e`'s destination clockwise from `twin(e)`; `e -> next(twin(e))` rings a node clockwise.
#[derive(Debug)]
struct HalfEdge {
	origin: WallNodeId,
	twin: HalfEdgeId,
	next: HalfEdgeId,
	previous: HalfEdgeId,
}

/// One wall: `forward` runs from `origin` to `destination`, `backward` the other way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Wall {
	pub forward: HalfEdgeId,
	pub backward: HalfEdgeId,
	pub origin: WallNodeId,
	pub destination: WallNodeId,
}

/// Where a new wall slots into a node's clockwise ring of walls.
#[derive(Clone, Copy)]
struct Splice {
	/// Existing edge arriving at the node; the new wall's leaving edge will follow it.
	arriving: HalfEdgeId,
	/// Existing edge leaving the node; it will follow the new wall's arriving edge.
	leaving: HalfEdgeId,
}

/// One end of a wall being added: the node it attaches to and where it slots into that node's ring.
struct WallEnd {
	node_id: WallNodeId,
	new_leaving: HalfEdgeId,
	new_arriving: HalfEdgeId,
	splice: Option<Splice>,
}

impl WallEnd {
	fn previous_of_leaving(&self) -> HalfEdgeId {
		self.splice
			.map_or(self.new_arriving, |splice| splice.arriving)
	}

	fn next_of_arriving(&self) -> HalfEdgeId {
		self.splice
			.map_or(self.new_leaving, |splice| splice.leaving)
	}
}

/// A graph of wall corners (nodes) joined by walls, from which rooms can be read off closed loops.
#[derive(Debug, Default)]
pub struct WallGraph {
	next_node_id: usize,
	next_edge_id: usize,
	nodes: HashMap<WallNodeId, WallNode>,
	edges: HashMap<HalfEdgeId, HalfEdge>,
}

impl WallGraph {
	pub fn new() -> Self {
		Self::default()
	}

	fn allocate_node_id(&mut self) -> WallNodeId {
		let id = self.next_node_id;
		self.next_node_id += 1;

		WallNodeId(id)
	}

	fn allocate_edge_id(&mut self) -> HalfEdgeId {
		let id = self.next_edge_id;
		self.next_edge_id += 1;

		HalfEdgeId(id)
	}

	fn node(&self, node_id: WallNodeId) -> &WallNode {
		self.nodes.get(&node_id).expect("unknown WallNodeId")
	}

	fn node_mut(&mut self, node_id: WallNodeId) -> &mut WallNode {
		self.nodes.get_mut(&node_id).expect("unknown WallNodeId")
	}

	fn edge(&self, edge_id: HalfEdgeId) -> &HalfEdge {
		self.edges.get(&edge_id).expect("unknown HalfEdgeId")
	}

	fn edge_mut(&mut self, edge_id: HalfEdgeId) -> &mut HalfEdge {
		self.edges.get_mut(&edge_id).expect("unknown HalfEdgeId")
	}

	/// Adds a corner at `position`, not yet joined to any wall.
	pub fn add_node(&mut self, position: Vec2) -> WallNodeId {
		let id = self.allocate_node_id();
		self.nodes.insert(
			id,
			WallNode {
				position,
				outgoing_edge: None,
			},
		);
		id
	}

	fn angle(&self, from: WallNodeId, to: WallNodeId) -> f32 {
		direction_angle(self.node(from).position, self.node(to).position)
	}

	fn edge_destination(&self, edge_id: HalfEdgeId) -> WallNodeId {
		self.edge(self.edge(edge_id).twin).origin
	}

	fn next_leaving_clockwise(&self, edge_id: HalfEdgeId) -> HalfEdgeId {
		self.edge(self.edge(edge_id).twin).next
	}

	fn leaving_edges(&self, node_id: WallNodeId) -> Vec<HalfEdgeId> {
		let Some(first) = self.node(node_id).outgoing_edge else {
			return Vec::new();
		};

		let mut edges = vec![first];
		let mut current = self.next_leaving_clockwise(first);
		while current != first {
			edges.push(current);
			current = self.next_leaving_clockwise(current);
		}
		edges
	}

	fn find_splice(&self, node_id: WallNodeId, new_angle: f32) -> Option<Splice> {
		let mut leaving: Vec<(f32, HalfEdgeId)> = self
			.leaving_edges(node_id)
			.into_iter()
			.map(|edge_id| (self.angle(node_id, self.edge_destination(edge_id)), edge_id))
			.collect();
		if leaving.is_empty() {
			return None;
		}
		leaving.sort_by(|a, b| a.0.total_cmp(&b.0));

		let slot = leaving.partition_point(|(angle, _)| *angle < new_angle);
		let counter_clockwise_neighbour = leaving[slot % leaving.len()].1;
		let clockwise_neighbour = leaving[(slot + leaving.len() - 1) % leaving.len()].1;

		Some(Splice {
			arriving: self.edge(counter_clockwise_neighbour).twin,
			leaving: clockwise_neighbour,
		})
	}

	fn wall_end(
		&self,
		node_id: WallNodeId,
		toward_id: WallNodeId,
		new_leaving: HalfEdgeId,
		new_arriving: HalfEdgeId,
	) -> WallEnd {
		WallEnd {
			node_id,
			new_leaving,
			new_arriving,
			splice: self.find_splice(node_id, self.angle(node_id, toward_id)),
		}
	}

	fn attach(&mut self, end: WallEnd) {
		if let Some(splice) = end.splice {
			self.edge_mut(splice.arriving).next = end.new_leaving;
			self.edge_mut(splice.leaving).previous = end.new_arriving;
		}
		self.node_mut(end.node_id).outgoing_edge = Some(end.new_leaving);
	}

	/// Adds a wall between two existing nodes, or explains why it was rejected; a rejected wall changes nothing.
	pub fn add_wall(
		&mut self,
		origin_id: WallNodeId,
		destination_id: WallNodeId,
	) -> Result<Wall, WallError> {
		self.validate_new_wall(origin_id, destination_id)?;

		let forward_id = self.allocate_edge_id();
		let backward_id = self.allocate_edge_id();

		let origin_end = self.wall_end(origin_id, destination_id, forward_id, backward_id);
		let destination_end = self.wall_end(destination_id, origin_id, backward_id, forward_id);

		self.edges.insert(
			forward_id,
			HalfEdge {
				origin: origin_id,
				twin: backward_id,
				next: destination_end.next_of_arriving(),
				previous: origin_end.previous_of_leaving(),
			},
		);
		self.edges.insert(
			backward_id,
			HalfEdge {
				origin: destination_id,
				twin: forward_id,
				next: origin_end.next_of_arriving(),
				previous: destination_end.previous_of_leaving(),
			},
		);

		self.attach(origin_end);
		self.attach(destination_end);

		debug_assert_eq!(self.validate(), Ok(()));

		Ok(Wall {
			forward: forward_id,
			backward: backward_id,
			origin: origin_id,
			destination: destination_id,
		})
	}

	/// Every node with its position, in no particular order.
	pub fn nodes(&self) -> impl Iterator<Item = (WallNodeId, Vec2)> + '_ {
		self.nodes.iter().map(|(&id, node)| (id, node.position))
	}

	/// Every wall exactly once, in no particular order.
	pub fn walls(&self) -> impl Iterator<Item = Wall> + '_ {
		self.edges
			.iter()
			.filter(|(id, edge)| **id < edge.twin)
			.map(|(&id, edge)| Wall {
				forward: id,
				backward: edge.twin,
				origin: edge.origin,
				destination: self.edge_destination(id),
			})
	}

	/// The position of a node, or `None` if it isn't a node of this graph.
	pub fn node_position(&self, node_id: WallNodeId) -> Option<Vec2> {
		self.nodes.get(&node_id).map(|node| node.position)
	}
}

mod validation;

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
