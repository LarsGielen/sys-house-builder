use super::WallGraph;
use crate::error::WallError;
use crate::geometry::{OVERLAP_ANGLE_TOLERANCE, angle_difference};
use crate::ids::WallNodeId;

impl WallGraph {
	pub(super) fn validate_new_wall(
		&self,
		origin_id: WallNodeId,
		destination_id: WallNodeId,
	) -> Result<(), WallError> {
		self.check_nodes_exist(origin_id, destination_id)?;
		if origin_id == destination_id {
			return Err(WallError::SameNode);
		}
		self.check_no_duplicate(origin_id, destination_id)?;
		self.check_no_overlap_at(origin_id, destination_id)?;
		self.check_no_overlap_at(destination_id, origin_id)
	}

	/// Checks the structural invariants of the whole graph, describing the first one that is broken.
	pub(super) fn validate(&self) -> Result<(), String> {
		for (&id, edge) in &self.edges {
			if self.edge(edge.twin).twin != id {
				return Err(format!("twin of twin of {id:?} is not itself"));
			}
			if self.edge(edge.next).previous != id {
				return Err(format!("previous(next({id:?})) is not itself"));
			}
			if self.edge(edge.previous).next != id {
				return Err(format!("next(previous({id:?})) is not itself"));
			}
			if self.edge(edge.next).origin != self.edge_destination(id) {
				return Err(format!("next({id:?}) does not start where {id:?} ends"));
			}
			if self.edge_destination(edge.previous) != edge.origin {
				return Err(format!("previous({id:?}) does not end where {id:?} starts"));
			}
		}
		for (&node_id, node) in &self.nodes {
			let starts_elsewhere = node
				.outgoing_edge
				.is_some_and(|edge_id| self.edge(edge_id).origin != node_id);
			if starts_elsewhere {
				return Err(format!(
					"outgoing edge of {node_id:?} does not start at that node"
				));
			}
		}
		Ok(())
	}

	fn check_nodes_exist(
		&self,
		origin_id: WallNodeId,
		destination_id: WallNodeId,
	) -> Result<(), WallError> {
		if self.nodes.contains_key(&origin_id) && self.nodes.contains_key(&destination_id) {
			Ok(())
		} else {
			Err(WallError::UnknownNode)
		}
	}

	fn check_no_duplicate(
		&self,
		origin_id: WallNodeId,
		destination_id: WallNodeId,
	) -> Result<(), WallError> {
		let exists = self
			.leaving_edges(origin_id)
			.into_iter()
			.any(|edge_id| self.edge_destination(edge_id) == destination_id);
		if exists {
			Err(WallError::Duplicate)
		} else {
			Ok(())
		}
	}

	/// Rejects a wall leaving `node_id` toward `toward_id` if it runs along a wall already leaving `node_id`.
	fn check_no_overlap_at(
		&self,
		node_id: WallNodeId,
		toward_id: WallNodeId,
	) -> Result<(), WallError> {
		let angle = self.angle(node_id, toward_id);
		let overlaps = self.leaving_edges(node_id).into_iter().any(|edge_id| {
			let existing_angle = self.angle(node_id, self.edge_destination(edge_id));
			angle_difference(angle, existing_angle) < OVERLAP_ANGLE_TOLERANCE
		});
		if overlaps {
			Err(WallError::Overlapping)
		} else {
			Ok(())
		}
	}
}
