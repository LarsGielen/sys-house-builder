//! Moving junctions and bending walls as one validated operation on the whole layout.
//!
//! A node is shared topology, so a drag moves a node and every wall that ends there follows.
//! Whole-wall and endpoint drags are the same operation applied to two nodes or one.
//!
//! Policy for the walls whose endpoints moved ("rebuilt walls"):
//! - Straight walls stay straight, keeping their other endpoint.
//! - An arc keeps a smooth join when it can: the arc is re-solved to leave a *leading* end
//!   along the join's tangent. A leading end is one whose neighbour has a direction that is
//!   already decided, either an unmoved end (its tangent is kept) or a straight neighbour
//!   (its new direction is known). With two leading ends the arc must satisfy both; if the
//!   two disagree it keeps the end at an unmoved node, else the start.
//! - Otherwise, or when no such arc exists, the arc keeps its sweep, which changes the curve
//!   the least. Joins that no longer hold are reported in [`ReshapeReport`], never stored.
//! - A rebuilt wall may not cross, touch, or overlap another wall, and no node may land on
//!   another. Those layouts need an insertion that splits walls, which a drag does not do.
//! - Openings stay on their wall at the same fraction of its length, so a stretched wall
//!   stretches the gap around them. The move is rejected if any opening then no longer fits.
use std::collections::HashMap;

use glam::Vec2;

use super::*;
use crate::arc::{arc_tangent_at_both, arc_tangent_at_start};

/// Two walls leave a node in opposite directions, within [`SMOOTH_JOIN_TOLERANCE`]. Inferred
/// from geometry on every query, so it cannot go stale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmoothJoin {
	pub node: WallNodeId,
	pub walls: [Wall; 2],
}

/// What a reshape rebuilt and which smooth joins it could not keep.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReshapeReport {
	/// Every wall whose handle changed, from its old handle to its new one.
	pub rebuilt_walls: Vec<(Wall, Wall)>,
	/// Nodes that had a smooth join before the operation and have none now.
	pub lost_smooth_joins: Vec<WallNodeId>,
}

/// Sine of the largest angle, in radians, between opposite departures that still count as one
/// smooth line. Larger than the insertion tolerance, which only separates departures.
const SMOOTH_JOIN_TOLERANCE: f64 = 1e-4;

impl WallGraph {
	/// Every pair of walls that continue each other smoothly through a node.
	pub fn smooth_joins(&self) -> Vec<SmoothJoin> {
		let mut nodes: Vec<_> = self.nodes.keys().copied().collect();
		nodes.sort();
		let mut joins = Vec::new();
		for node in nodes {
			let departures = self.outgoing_half_edges(node);
			for (index, &first) in departures.iter().enumerate() {
				for &second in &departures[index + 1..] {
					let first_tangent = self.half_edge_curve(first).tangent(0.0);
					let second_tangent = self.half_edge_curve(second).tangent(0.0);
					if first_tangent.perp_dot(second_tangent).abs() < SMOOTH_JOIN_TOLERANCE
						&& first_tangent.dot(second_tangent) < 0.0
					{
						joins.push(SmoothJoin {
							node,
							walls: [
								self.wall_of_half_edge(first),
								self.wall_of_half_edge(second),
							],
						});
					}
				}
			}
		}
		joins
	}

	/// Moves nodes to new positions, rebuilding every wall that ends at one.
	///
	/// See the module policy for how arcs, openings, and tangency are treated. Handles of
	/// rebuilt walls change and are listed in the report; node IDs, opening IDs, and the
	/// handles of every other wall are kept. On error the graph is unchanged.
	pub fn move_nodes(&mut self, moves: &[(WallNodeId, Vec2)]) -> Result<ReshapeReport, WallError> {
		let mut targets = HashMap::new();
		for &(node, position) in moves {
			let current = self.node_position(node).ok_or(WallError::UnknownNode)?;
			if !position.is_finite() {
				return Err(WallError::InvalidPosition);
			}
			if current != position {
				targets.insert(node, position);
			}
		}
		if targets.is_empty() {
			return Ok(ReshapeReport::default());
		}
		let (proposed, report) = self.reshaped(&targets, None)?;
		*self = proposed;
		Ok(report)
	}

	/// Changes one wall's sweep, keeping its endpoints. A sweep of zero makes it straight.
	/// The sign is relative to the handle's direction. Walls joined at its ends keep their
	/// shape, so a smooth join there is lost unless the new curve still meets it.
	pub fn bend_wall(&mut self, wall: Wall, signed_sweep: f32) -> Result<ReshapeReport, WallError> {
		self.validate_wall_handle(wall)?;
		if !signed_sweep.is_finite() || signed_sweep.abs() >= TAU {
			return Err(WallError::InvalidArc);
		}
		let canonical = self.wall_of_half_edge(wall.forward);
		let sweep = if canonical.forward == wall.forward {
			signed_sweep
		} else {
			-signed_sweep
		};
		let shape = if sweep.abs() <= DIRECTION_ANGLE_TOLERANCE {
			CurveShape::Straight
		} else {
			CurveShape::CircularArc {
				sweep: f64::from(sweep),
			}
		};
		let (proposed, report) = self.reshaped(&HashMap::new(), Some((canonical, shape)))?;
		*self = proposed;
		Ok(report)
	}

	fn reshaped(
		&self,
		moved: &HashMap<WallNodeId, Vec2>,
		bend: Option<(Wall, CurveShape)>,
	) -> Result<(Self, ReshapeReport), WallError> {
		let position_after = |node: WallNodeId| {
			moved
				.get(&node)
				.copied()
				.unwrap_or_else(|| self.node(node).position)
		};
		for (&node, &position) in moved {
			let crowded = self.nodes.keys().any(|&other| {
				other != node && position_after(other).distance(position) <= DISTANCE_TOLERANCE
			});
			if crowded {
				return Err(WallError::CrossesWall);
			}
		}

		let mut affected: Vec<Wall> = self
			.walls()
			.filter(|wall| {
				moved.contains_key(&wall.origin)
					|| moved.contains_key(&wall.destination)
					|| bend.is_some_and(|(bent, _)| bent == *wall)
			})
			.collect();
		affected.sort_by_key(|wall| wall.forward);

		let joins_before = self.smooth_joins();
		let mut shapes = Vec::with_capacity(affected.len());
		for &wall in &affected {
			let shape = match bend {
				Some((bent, shape)) if bent == wall => shape,
				_ => self.shape_after_move(wall, moved, &affected, &joins_before, &position_after),
			};
			let curve = Curve::new(
				position_after(wall.origin),
				position_after(wall.destination),
				shape,
			);
			if curve.start.distance(curve.end) <= DISTANCE_TOLERANCE as f64 {
				return Err(WallError::ZeroLength);
			}
			if !curve.valid() {
				return Err(WallError::InvalidArc);
			}
			shapes.push(shape);
		}

		let mut proposed = self.clone();
		let openings_before: Vec<Vec<(OpeningId, f64)>> = affected
			.iter()
			.map(|&wall| {
				let length = self.half_edge_curve(wall.forward).length();
				self.openings
					.iter()
					.filter(|(_, opening)| opening.wall == wall)
					.map(|(&id, opening)| (id, opening.spec.center_distance / length))
					.collect()
			})
			.collect();
		for &wall in &affected {
			proposed.remove_wall_unchecked(wall);
		}
		for (&node, &position) in moved {
			proposed.node_mut(node).position = position;
		}
		let mut rebuilt = Vec::with_capacity(affected.len());
		for ((&wall, &shape), openings) in affected.iter().zip(&shapes).zip(&openings_before) {
			let dimensions = self.wall_data[&wall.forward].dimensions;
			let new_wall =
				proposed.insert_wall_unchecked(wall.origin, wall.destination, shape, dimensions)?;
			let old_length = self.half_edge_curve(wall.forward).length();
			let new_length = proposed.half_edge_curve(new_wall.forward).length();
			for &(id, fraction) in openings {
				let opening = proposed
					.openings
					.get_mut(&id)
					.expect("an opening of a rebuilt wall exists");
				opening.wall = new_wall;
				// A rigid move must not nudge the opening through rounding.
				if (new_length - old_length).abs() > DISTANCE_TOLERANCE as f64 {
					opening.spec.center_distance = fraction * new_length;
				}
			}
			rebuilt.push((wall, new_wall));
		}

		let new_walls: Vec<Wall> = rebuilt.iter().map(|&(_, new)| new).collect();
		proposed.validate_departures_at(&new_walls)?;
		proposed.validate_no_crossings(&new_walls)?;
		proposed.validate_openings()?;
		debug_assert_eq!(proposed.validate_topology(), Ok(()));
		debug_assert_eq!(proposed.validate_geometry(), Ok(()));

		let lost_smooth_joins = lost_joins(&joins_before, &proposed.smooth_joins(), &rebuilt);
		Ok((
			proposed,
			ReshapeReport {
				rebuilt_walls: rebuilt,
				lost_smooth_joins,
			},
		))
	}

	fn shape_after_move(
		&self,
		wall: Wall,
		moved: &HashMap<WallNodeId, Vec2>,
		affected: &[Wall],
		joins: &[SmoothJoin],
		position_after: &dyn Fn(WallNodeId) -> Vec2,
	) -> CurveShape {
		let CurveShape::CircularArc { sweep } = self.wall_data[&wall.forward].shape else {
			return CurveShape::Straight;
		};
		let origin = position_after(wall.origin);
		let destination = position_after(wall.destination);
		let anchor =
			|node| self.anchored_direction(wall, node, moved, affected, joins, position_after);
		let from_origin = |direction| arc_tangent_at_start(origin, direction, destination);
		let from_destination =
			|direction| arc_tangent_at_start(destination, direction, origin).map(|sweep| -sweep);

		let solved = match (anchor(wall.origin), anchor(wall.destination)) {
			(Some(at_origin), Some(at_destination)) => {
				arc_tangent_at_both(origin, at_origin, destination, -at_destination).or_else(|_| {
					if moved.contains_key(&wall.origin) && !moved.contains_key(&wall.destination) {
						from_destination(at_destination)
					} else {
						from_origin(at_origin)
					}
				})
			}
			(Some(at_origin), None) => from_origin(at_origin),
			(None, Some(at_destination)) => from_destination(at_destination),
			(None, None) => return CurveShape::CircularArc { sweep },
		};
		match solved {
			Ok(0.0) => CurveShape::Straight,
			Ok(solved) => CurveShape::CircularArc {
				sweep: f64::from(solved),
			},
			// No arc keeps the join, so keep the arc's own sweep.
			Err(_) => CurveShape::CircularArc { sweep },
		}
	}

	/// The direction `wall` must leave `node` in to stay smooth with its partner there, if the
	/// partner's direction is decided. See the module policy.
	fn anchored_direction(
		&self,
		wall: Wall,
		node: WallNodeId,
		moved: &HashMap<WallNodeId, Vec2>,
		affected: &[Wall],
		joins: &[SmoothJoin],
		position_after: &dyn Fn(WallNodeId) -> Vec2,
	) -> Option<Vec2> {
		let join = joins
			.iter()
			.find(|join| join.node == node && join.walls.contains(&wall))?;
		let partner = if join.walls[0] == wall {
			join.walls[1]
		} else {
			join.walls[0]
		};
		let partner_is_straight = self.wall_data[&partner.forward].shape == CurveShape::Straight;
		if partner_is_straight && affected.contains(&partner) {
			let far = if partner.origin == node {
				partner.destination
			} else {
				partner.origin
			};
			let outgoing = position_after(far) - position_after(node);
			(outgoing.length() > DISTANCE_TOLERANCE).then(|| -outgoing.normalize())
		} else if !moved.contains_key(&node) {
			let edge = if wall.origin == node {
				wall.forward
			} else {
				wall.backward
			};
			Some(self.half_edge_curve(edge).tangent(0.0).as_vec2())
		} else {
			None
		}
	}

	/// Reordering a node's departures is safe; two that cannot be told apart are not.
	fn validate_departures_at(&self, walls: &[Wall]) -> Result<(), WallError> {
		let mut nodes: Vec<_> = walls
			.iter()
			.flat_map(|wall| [wall.origin, wall.destination])
			.collect();
		nodes.sort();
		nodes.dedup();
		for node in nodes {
			let departures: Vec<(f32, CurveShape)> = self
				.outgoing_half_edges(node)
				.into_iter()
				.map(|edge| {
					let curve = self.half_edge_curve(edge);
					(curve.tangent(0.0).as_vec2().to_angle(), curve.shape)
				})
				.collect();
			for (index, &(angle, shape)) in departures.iter().enumerate() {
				for &(other_angle, other_shape) in &departures[index + 1..] {
					if angular_distance(angle, other_angle) < DIRECTION_ANGLE_TOLERANCE {
						return Err(
							if shape == CurveShape::Straight && other_shape == CurveShape::Straight
							{
								WallError::Overlapping
							} else {
								WallError::TangentialContact
							},
						);
					}
				}
			}
		}
		Ok(())
	}

	/// The rebuilt walls may meet other walls only at the nodes they share.
	fn validate_no_crossings(&self, rebuilt: &[Wall]) -> Result<(), WallError> {
		let all: Vec<Wall> = self.walls().collect();
		for &wall in rebuilt {
			let curve = self.half_edge_curve(wall.forward);
			for &other in &all {
				// A pair of rebuilt walls is checked once.
				if other == wall || (rebuilt.contains(&other) && other.forward < wall.forward) {
					continue;
				}
				let result = intersections(curve, self.half_edge_curve(other.forward));
				if result.overlapping {
					return Err(WallError::Overlapping);
				}
				for contact in result.contacts {
					let point = curve.position(contact.first_parameter);
					let at_shared_node = [wall.origin, wall.destination]
						.into_iter()
						.filter(|node| [other.origin, other.destination].contains(node))
						.any(|node| {
							self.node(node).position.as_dvec2().distance(point)
								<= DISTANCE_TOLERANCE as f64
						});
					if !at_shared_node {
						return Err(WallError::CrossesWall);
					}
				}
			}
		}
		Ok(())
	}
}

/// Nodes where a smooth join was present before and its pair of walls no longer continue
/// each other. Rebuilt walls are compared through their new handles.
fn lost_joins(
	before: &[SmoothJoin],
	after: &[SmoothJoin],
	rebuilt: &[(Wall, Wall)],
) -> Vec<WallNodeId> {
	let current = |wall: Wall| {
		rebuilt
			.iter()
			.find(|&&(old, _)| old == wall)
			.map_or(wall, |&(_, new)| new)
	};
	let mut lost = Vec::new();
	for join in before {
		let pair = join.walls.map(current);
		let kept = after.iter().any(|candidate| {
			candidate.node == join.node
				&& candidate.walls.contains(&pair[0])
				&& candidate.walls.contains(&pair[1])
		});
		if !kept && !lost.contains(&join.node) {
			lost.push(join.node);
		}
	}
	lost
}
