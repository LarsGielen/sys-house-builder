use std::collections::HashMap;
use std::f32::consts::TAU;
use std::fmt;

use glam::Vec2;

use crate::geometry::{Curve, CurveShape, intersections};

/// Departures from a node within this many radians are treated as collinear.
const DIRECTION_ANGLE_TOLERANCE: f32 = crate::geometry::ANGLE_TOLERANCE as f32;

/// Points at most this far apart count as the same contact; point-on-curve tests use this tolerance.
const DISTANCE_TOLERANCE: f32 = crate::geometry::DISTANCE_TOLERANCE as f32;

/// Identifies a node in one [`WallGraph`].
///
/// The identifier is opaque and not reused during ordinary edits. [`WallGraph::optimize`]
/// reassigns node identifiers, so callers must use its returned mapping afterwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WallNodeId(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct HalfEdgeId(usize);

/// An error caused by a proposed mutation of a [`WallGraph`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WallError {
	/// A proposed endpoint position contains a non-finite coordinate.
	InvalidPosition,
	/// An endpoint is not a node of this graph.
	UnknownNode,
	/// The wall would start and end at the same node.
	SameNode,
	/// The same geometric path already joins these nodes.
	Duplicate,
	/// The wall runs along part of an existing wall.
	Overlapping,
	/// A wall piece would have endpoints within the graph's distance tolerance.
	ZeroLength,
	/// Junction snapping cannot preserve the participating curves within distance tolerance.
	InconsistentJunction,
	/// The wall is not a wall of this graph, for example because it was already removed.
	UnknownWall,
	/// The sweep is invalid or the arc extends beyond representable coordinates.
	InvalidArc,
	/// Two departures have indistinguishable tangents; their ordering is unsupported.
	TangentialContact,
	/// A query parameter or sampling deviation is invalid or requests too many samples.
	InvalidParameter,
	/// The graph cannot allocate another node or half-edge identifier.
	IdExhausted,
}

impl fmt::Display for WallError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let message = match self {
			WallError::InvalidPosition => "a node position must contain only finite coordinates",
			WallError::UnknownNode => "a wall endpoint is not a node of this graph",
			WallError::SameNode => "a wall cannot start and end at the same node",
			WallError::Duplicate => "the same wall path already joins these nodes",
			WallError::Overlapping => "the wall runs along part of an existing wall",
			WallError::ZeroLength => "a wall cannot join nodes within the distance tolerance",
			WallError::InconsistentJunction => {
				"a junction cannot preserve wall geometry within the distance tolerance"
			}
			WallError::UnknownWall => "the wall is not a wall of this graph",
			WallError::InvalidArc => {
				"an arc needs representable coordinates and a finite sweep with magnitude greater than 0.000001 and less than one revolution"
			}
			WallError::TangentialContact => {
				"walls with indistinguishable departure tangents are unsupported"
			}
			WallError::InvalidParameter => "invalid curve parameter or sampling deviation",
			WallError::IdExhausted => "wall graph identifier space is exhausted",
		};
		f.write_str(message)
	}
}

impl std::error::Error for WallError {}

fn angular_distance(a: f32, b: f32) -> f32 {
	let difference = (a - b).abs();
	difference.min(TAU - difference)
}

#[derive(Debug, Clone, Copy)]
struct WallNode {
	position: Vec2,
	outgoing_edge: Option<HalfEdgeId>,
}

/// Faces lie to the left of each half-edge.
///
/// `next(e)` is the first edge leaving `e`'s destination clockwise from `twin(e)`.
/// This keeps both face traversal and each node's clockwise ring in one linkage.
#[derive(Debug, Clone)]
struct HalfEdge {
	origin: WallNodeId,
	twin: HalfEdgeId,
	next: HalfEdgeId,
	previous: HalfEdgeId,
}

/// A handle to one current wall piece in a [`WallGraph`].
///
/// A handle becomes invalid when its wall is removed, split, or merged. Its endpoints remain
/// available so callers can interpret results without borrowing the graph again. Optimization
/// reassigns wall handles; use the mapping returned by [`WallGraph::optimize`] afterwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Wall {
	forward: HalfEdgeId,
	backward: HalfEdgeId,
	origin: WallNodeId,
	destination: WallNodeId,
}

/// Old-to-new identifiers produced by [`WallGraph::optimize`].
///
/// Callers holding node IDs or wall handles must replace them with the values returned here.
#[derive(Debug)]
pub struct WallGraphIdMap {
	nodes: HashMap<WallNodeId, WallNodeId>,
	walls: HashMap<Wall, Wall>,
}

impl WallGraphIdMap {
	/// Returns the compacted identifier of a node that existed before optimization.
	pub fn node(&self, old: WallNodeId) -> Option<WallNodeId> {
		self.nodes.get(&old).copied()
	}

	/// Returns the compacted handle of a wall that existed before optimization.
	pub fn wall(&self, old: Wall) -> Option<Wall> {
		self.walls.get(&old).copied()
	}
}

impl Wall {
	/// The endpoint from which this handle is directed.
	pub fn origin(self) -> WallNodeId {
		self.origin
	}

	/// The endpoint toward which this handle is directed.
	pub fn destination(self) -> WallNodeId {
		self.destination
	}
}

/// The neighbouring half-edges around a node where a new wall endpoint is inserted.
#[derive(Clone, Copy)]
struct NodeRingGap {
	preceding_arrival: HalfEdgeId,
	following_departure: HalfEdgeId,
}

/// A planar graph of wall junctions joined by straight segments and circular arcs.
///
/// Insertion takes positions and creates or reuses its endpoint nodes. It divides the new path
/// and any crossed walls at their junctions. Consecutive compatible pieces merge through
/// degree-two nodes. Geometric comparisons use a fixed tolerance of `0.0001` coordinate units,
/// so resulting pieces are longer than that tolerance.
/// Removing a wall also removes either endpoint if no other wall still reaches it.
#[derive(Debug, Default, Clone)]
pub struct WallGraph {
	next_node_id: usize,
	next_edge_id: usize,
	nodes: HashMap<WallNodeId, WallNode>,
	edges: HashMap<HalfEdgeId, HalfEdge>,
	// The smaller half-edge ID owns the shape in its forward direction.
	shapes: HashMap<HalfEdgeId, CurveShape>,
}

impl WallGraph {
	/// Creates an empty graph.
	pub fn new() -> Self {
		Self::default()
	}

	/// Rebuilds the graph with contiguous node and half-edge IDs, starting at zero.
	///
	/// Geometry and connectivity are preserved. Use the returned mapping to update references
	/// held outside the graph; old identifiers and wall handles must not be used afterwards.
	/// The operation also resets the allocation counters to the compacted graph's next IDs.
	pub fn optimize(&mut self) -> WallGraphIdMap {
		let mut compact = Self::new();
		let mut node_ids = HashMap::with_capacity(self.nodes.len());
		let mut old_nodes: Vec<_> = self.nodes.keys().copied().collect();
		old_nodes.sort();
		for old in old_nodes {
			let new = compact
				.insert_node(self.node(old).position)
				.expect("a compacted graph cannot exhaust node IDs");
			node_ids.insert(old, new);
		}

		let mut wall_ids = HashMap::with_capacity(self.shapes.len());
		let mut old_walls: Vec<_> = self.walls().collect();
		old_walls.sort_by_key(|wall| wall.forward);
		for old in old_walls {
			let new = compact
				.insert_wall_unchecked(
					node_ids[&old.origin],
					node_ids[&old.destination],
					self.shapes[&old.forward],
				)
				.expect("a compacted graph cannot exhaust half-edge IDs");
			wall_ids.insert(old, new);
		}
		debug_assert_eq!(compact.validate_topology(), Ok(()));
		debug_assert_eq!(compact.validate_geometry(), Ok(()));
		*self = compact;
		WallGraphIdMap {
			nodes: node_ids,
			walls: wall_ids,
		}
	}

	fn allocate_node_id(&mut self) -> Result<WallNodeId, WallError> {
		let id = self.next_node_id;
		self.next_node_id = self
			.next_node_id
			.checked_add(1)
			.ok_or(WallError::IdExhausted)?;

		Ok(WallNodeId(id))
	}

	fn allocate_edge_id(&mut self) -> Result<HalfEdgeId, WallError> {
		let id = self.next_edge_id;
		self.next_edge_id = self
			.next_edge_id
			.checked_add(1)
			.ok_or(WallError::IdExhausted)?;

		Ok(HalfEdgeId(id))
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

	fn insert_node(&mut self, position: Vec2) -> Result<WallNodeId, WallError> {
		debug_assert!(position.is_finite());
		let id = self.allocate_node_id()?;
		self.nodes.insert(
			id,
			WallNode {
				position,
				outgoing_edge: None,
			},
		);
		Ok(id)
	}

	fn half_edge_curve(&self, edge_id: HalfEdgeId) -> Curve {
		let twin = self.edge(edge_id).twin;
		let shape = self.shapes[&edge_id.min(twin)];
		Curve::new(
			self.node(self.edge(edge_id).origin).position,
			self.node(self.half_edge_destination(edge_id)).position,
			if edge_id < twin {
				shape
			} else {
				shape.reversed()
			},
		)
	}

	fn half_edge_destination(&self, edge_id: HalfEdgeId) -> WallNodeId {
		self.edge(self.edge(edge_id).twin).origin
	}

	fn next_outgoing_half_edge_clockwise(&self, edge_id: HalfEdgeId) -> HalfEdgeId {
		self.edge(self.edge(edge_id).twin).next
	}

	fn outgoing_half_edges(&self, node_id: WallNodeId) -> Vec<HalfEdgeId> {
		let Some(first) = self.node(node_id).outgoing_edge else {
			return Vec::new();
		};

		let mut edges = vec![first];
		let mut current = self.next_outgoing_half_edge_clockwise(first);
		while current != first {
			edges.push(current);
			current = self.next_outgoing_half_edge_clockwise(current);
		}
		edges
	}

	fn find_node_ring_gap(&self, node_id: WallNodeId, new_angle: f32) -> Option<NodeRingGap> {
		let mut leaving: Vec<(f32, HalfEdgeId)> = self
			.outgoing_half_edges(node_id)
			.into_iter()
			.map(|edge_id| {
				(
					self.half_edge_curve(edge_id)
						.tangent(0.0)
						.as_vec2()
						.to_angle(),
					edge_id,
				)
			})
			.collect();
		if leaving.is_empty() {
			return None;
		}
		leaving.sort_by(|a, b| a.0.total_cmp(&b.0));

		let slot = leaving.partition_point(|(angle, _)| *angle < new_angle);
		let counter_clockwise_neighbour = leaving[slot % leaving.len()].1;
		let clockwise_neighbour = leaving[(slot + leaving.len() - 1) % leaving.len()].1;

		Some(NodeRingGap {
			preceding_arrival: self.edge(counter_clockwise_neighbour).twin,
			following_departure: clockwise_neighbour,
		})
	}

	/// Adds a straight wall between two positions, creating or reusing its endpoint nodes.
	///
	/// An existing node within the distance tolerance is reused. Existing walls are split where
	/// the new path crosses them or ends on their interior. Compatible straight pieces merge through
	/// degree-two nodes. The returned walls cover the new path in order, but a merged wall can extend
	/// beyond either requested endpoint. Splitting or merging invalidates affected old wall handles.
	/// On error, neither the graph nor its identifier counters change.
	pub fn add_wall(&mut self, origin: Vec2, destination: Vec2) -> Result<Vec<Wall>, WallError> {
		self.add_curve_between_positions(origin, destination, CurveShape::Straight)
	}

	/// Adds a circular arc between two positions, creating or reusing its endpoint nodes.
	///
	/// `signed_sweep` is in radians: positive is counterclockwise, negative clockwise. Its magnitude
	/// must exceed `0.000001` and be less than one revolution. Major arcs are supported;
	/// full circles and tangential junctions are not. Compatible arcs merge through degree-two
	/// nodes. The returned walls cover the new path in order, but a merged arc can extend beyond
	/// either requested endpoint. Merging invalidates affected old handles. On error, the graph
	/// and its identifier counters are unchanged.
	pub fn add_arc(
		&mut self,
		origin: Vec2,
		destination: Vec2,
		signed_sweep: f32,
	) -> Result<Vec<Wall>, WallError> {
		if !signed_sweep.is_finite()
			|| signed_sweep.abs() <= DIRECTION_ANGLE_TOLERANCE
			|| signed_sweep.abs() >= TAU
		{
			return Err(WallError::InvalidArc);
		}
		self.add_curve_between_positions(
			origin,
			destination,
			CurveShape::CircularArc {
				sweep: signed_sweep as f64,
			},
		)
	}

	fn add_curve_between_positions(
		&mut self,
		origin: Vec2,
		destination: Vec2,
		shape: CurveShape,
	) -> Result<Vec<Wall>, WallError> {
		if !origin.is_finite() || !destination.is_finite() {
			return Err(WallError::InvalidPosition);
		}
		if origin.distance(destination) <= DISTANCE_TOLERANCE {
			return Err(WallError::ZeroLength);
		}
		let requested = Curve::new(origin, destination, shape);
		if !requested.valid() {
			return Err(WallError::InvalidArc);
		}

		// Plan against a copy so endpoint creation is part of the same transaction as wall insertion.
		let mut proposed = Self {
			next_node_id: self.next_node_id,
			next_edge_id: self.next_edge_id,
			nodes: self.nodes.clone(),
			edges: self.edges.clone(),
			shapes: self.shapes.clone(),
		};
		let origin_id = proposed.find_or_insert_endpoint(origin)?;
		let destination_id = proposed.find_or_insert_endpoint(destination)?;
		if origin_id == destination_id {
			return Err(WallError::ZeroLength);
		}
		let actual = Curve::new(
			proposed.node(origin_id).position,
			proposed.node(destination_id).position,
			shape,
		);
		if !actual.valid() || actual.deviation_from(requested, 0.0, 1.0) > DISTANCE_TOLERANCE as f64
		{
			return Err(WallError::InconsistentJunction);
		}
		let mut walls = proposed.add_curve(origin_id, destination_id, shape)?;
		proposed.merge_added_walls(&mut walls)?;
		debug_assert_eq!(proposed.validate_topology(), Ok(()));
		debug_assert_eq!(proposed.validate_geometry(), Ok(()));
		*self = proposed;
		Ok(walls)
	}

	fn merge_added_walls(&mut self, added: &mut Vec<Wall>) -> Result<(), WallError> {
		loop {
			let mut merge = None;
			'search: for &wall in added.iter() {
				for node in [wall.origin, wall.destination] {
					if let Some((other, merged)) = self.try_merge_at(wall, node)? {
						merge = Some((wall, other, merged));
						break 'search;
					}
				}
			}
			let Some((wall, other, merged)) = merge else {
				break;
			};
			for result in added.iter_mut() {
				if *result == wall || *result == other {
					*result = merged;
				}
			}
			added.dedup();
		}
		Ok(())
	}

	fn try_merge_at(
		&mut self,
		wall: Wall,
		node: WallNodeId,
	) -> Result<Option<(Wall, Wall)>, WallError> {
		let outgoing = self.outgoing_half_edges(node);
		if outgoing.len() != 2 {
			return Ok(None);
		}
		let own_edge = if wall.origin == node {
			wall.forward
		} else {
			wall.backward
		};
		let Some(&other_edge) = outgoing.iter().find(|&&edge| edge != own_edge) else {
			return Ok(None);
		};
		let other_twin = self.edge(other_edge).twin;
		let other_forward = other_edge.min(other_twin);
		let other = Wall {
			forward: other_forward,
			backward: other_edge.max(other_twin),
			origin: self.edge(other_forward).origin,
			destination: self.half_edge_destination(other_forward),
		};
		let (first, second, origin, destination) = if wall.origin == node {
			(
				self.half_edge_curve(other_twin),
				self.half_edge_curve(wall.forward),
				self.half_edge_destination(other_edge),
				wall.destination,
			)
		} else {
			(
				self.half_edge_curve(wall.forward),
				self.half_edge_curve(other_edge),
				wall.origin,
				self.half_edge_destination(other_edge),
			)
		};
		let Some(shape) = first.merged_shape(second) else {
			return Ok(None);
		};
		self.next_edge_id
			.checked_add(2)
			.ok_or(WallError::IdExhausted)?;
		self.remove_wall_unchecked(wall);
		self.remove_wall_unchecked(other);
		self.nodes.remove(&node);
		Ok(Some((
			other,
			self.insert_wall_unchecked(origin, destination, shape)?,
		)))
	}

	fn find_or_insert_endpoint(&mut self, position: Vec2) -> Result<WallNodeId, WallError> {
		if let Some(id) = self
			.nodes()
			.filter(|&(_, existing)| existing.distance(position) <= DISTANCE_TOLERANCE)
			.min_by(|(first_id, first), (second_id, second)| {
				first
					.distance(position)
					.total_cmp(&second.distance(position))
					.then_with(|| first_id.cmp(second_id))
			})
			.map(|(id, _)| id)
		{
			Ok(id)
		} else {
			self.insert_node(position)
		}
	}

	fn add_curve(
		&mut self,
		origin_id: WallNodeId,
		destination_id: WallNodeId,
		shape: CurveShape,
	) -> Result<Vec<Wall>, WallError> {
		self.validate_wall_endpoints(origin_id, destination_id)?;
		let curve = Curve::new(
			self.node(origin_id).position,
			self.node(destination_id).position,
			shape,
		);
		if !curve.valid() {
			return Err(WallError::InvalidArc);
		}
		let plan = self.plan_wall_insertion(origin_id, destination_id, curve)?;
		let new_nodes = plan
			.junctions
			.iter()
			.filter(|junction| junction.existing_node.is_none())
			.count();
		let wall_pieces =
			plan.replacements
				.iter()
				.try_fold(plan.additions.len(), |count, (_, pieces)| {
					count
						.checked_add(pieces.len())
						.ok_or(WallError::IdExhausted)
				})?;
		let new_edges = wall_pieces.checked_mul(2).ok_or(WallError::IdExhausted)?;
		self.next_node_id
			.checked_add(new_nodes)
			.ok_or(WallError::IdExhausted)?;
		self.next_edge_id
			.checked_add(new_edges)
			.ok_or(WallError::IdExhausted)?;
		let junction_ids: Vec<_> = plan
			.junctions
			.iter()
			.map(|junction| {
				if let Some(id) = junction.existing_node {
					Ok(id)
				} else {
					self.insert_node(junction.position)
				}
			})
			.collect::<Result<_, WallError>>()?;
		let resolve = |node: PlannedNode| match node {
			PlannedNode::Existing(id) => id,
			PlannedNode::Junction(index) => junction_ids[index],
		};
		for (wall, _) in &plan.replacements {
			self.remove_wall_unchecked(*wall);
		}
		for (_, pieces) in plan.replacements {
			for piece in pieces {
				self.insert_wall_unchecked(
					resolve(piece.origin),
					resolve(piece.destination),
					piece.shape,
				)?;
			}
		}
		let walls = plan
			.additions
			.into_iter()
			.map(|piece| {
				self.insert_wall_unchecked(
					resolve(piece.origin),
					resolve(piece.destination),
					piece.shape,
				)
			})
			.collect::<Result<Vec<_>, WallError>>()?;
		debug_assert_eq!(self.validate_topology(), Ok(()));
		debug_assert_eq!(self.validate_geometry(), Ok(()));
		Ok(walls)
	}

	/// Removes a wall and any endpoint left with no remaining walls.
	///
	/// Removing a wall invalidates its handle. Shared endpoint nodes remain connected.
	pub fn remove_wall(&mut self, wall: Wall) -> Result<(), WallError> {
		self.validate_wall_handle(wall)?;
		self.remove_wall_unchecked(wall);
		for node_id in [wall.origin, wall.destination] {
			if self.node(node_id).outgoing_edge.is_none() {
				self.nodes.remove(&node_id);
			}
		}

		debug_assert_eq!(self.validate_topology(), Ok(()));
		debug_assert_eq!(self.validate_geometry(), Ok(()));

		Ok(())
	}

	fn remove_wall_unchecked(&mut self, wall: Wall) {
		self.detach_wall_end_from_node_ring(wall.forward, wall.backward);
		self.detach_wall_end_from_node_ring(wall.backward, wall.forward);
		self.edges.remove(&wall.forward);
		self.edges.remove(&wall.backward);
		self.shapes.remove(&wall.forward.min(wall.backward));
	}

	fn detach_wall_end_from_node_ring(&mut self, leaving: HalfEdgeId, arriving: HalfEdgeId) {
		let node_id = self.edge(leaving).origin;
		let before = self.edge(leaving).previous;
		let after = self.edge(arriving).next;

		if before == arriving {
			// The wall was the node's only one.
			self.node_mut(node_id).outgoing_edge = None;
		} else {
			self.edge_mut(before).next = after;
			self.edge_mut(after).previous = before;
			self.node_mut(node_id).outgoing_edge = Some(after);
		}
	}

	/// Inserts a wall after callers have established that the operation preserves graph invariants.
	fn insert_wall_unchecked(
		&mut self,
		origin_id: WallNodeId,
		destination_id: WallNodeId,
		shape: CurveShape,
	) -> Result<Wall, WallError> {
		self.next_edge_id
			.checked_add(2)
			.ok_or(WallError::IdExhausted)?;
		let forward_id = self.allocate_edge_id()?;
		let backward_id = self.allocate_edge_id()?;

		let curve = Curve::new(
			self.node(origin_id).position,
			self.node(destination_id).position,
			shape,
		);
		let origin_gap =
			self.find_node_ring_gap(origin_id, curve.tangent(0.0).as_vec2().to_angle());
		let destination_gap =
			self.find_node_ring_gap(destination_id, (-curve.tangent(1.0)).as_vec2().to_angle());

		self.shapes.insert(forward_id, shape);
		self.edges.insert(
			forward_id,
			HalfEdge {
				origin: origin_id,
				twin: backward_id,
				next: destination_gap.map_or(backward_id, |gap| gap.following_departure),
				previous: origin_gap.map_or(backward_id, |gap| gap.preceding_arrival),
			},
		);
		self.edges.insert(
			backward_id,
			HalfEdge {
				origin: destination_id,
				twin: forward_id,
				next: origin_gap.map_or(forward_id, |gap| gap.following_departure),
				previous: destination_gap.map_or(forward_id, |gap| gap.preceding_arrival),
			},
		);

		if let Some(gap) = origin_gap {
			self.edge_mut(gap.preceding_arrival).next = forward_id;
			self.edge_mut(gap.following_departure).previous = backward_id;
		}
		self.node_mut(origin_id).outgoing_edge = Some(forward_id);

		if let Some(gap) = destination_gap {
			self.edge_mut(gap.preceding_arrival).next = backward_id;
			self.edge_mut(gap.following_departure).previous = forward_id;
		}
		self.node_mut(destination_id).outgoing_edge = Some(backward_id);

		Ok(Wall {
			forward: forward_id,
			backward: backward_id,
			origin: origin_id,
			destination: destination_id,
		})
	}

	/// Iterates over every current wall junction and its position, in no particular order.
	pub fn nodes(&self) -> impl Iterator<Item = (WallNodeId, Vec2)> + '_ {
		self.nodes.iter().map(|(&id, node)| (id, node.position))
	}

	/// Iterates over every current wall piece exactly once, in no particular order.
	pub fn walls(&self) -> impl Iterator<Item = Wall> + '_ {
		self.edges
			.iter()
			.filter(|(id, edge)| **id < edge.twin)
			.map(|(&id, edge)| Wall {
				forward: id,
				backward: edge.twin,
				origin: edge.origin,
				destination: self.half_edge_destination(id),
			})
	}

	/// Evaluates a live wall at a normalized parameter in `[0, 1]`, in the handle's direction.
	/// Returns `None` for a stale handle or invalid parameter. Parameters do not generally measure distance.
	pub fn wall_position(&self, wall: Wall, parameter: f32) -> Option<Vec2> {
		self.validate_wall_handle(wall).ok()?;
		(0.0..=1.0).contains(&parameter).then(|| {
			self.half_edge_curve(wall.forward)
				.position(parameter as f64)
				.as_vec2()
		})
	}

	/// Returns the unit tangent in the handle's direction, or `None` for invalid input.
	pub fn wall_tangent(&self, wall: Wall, parameter: f32) -> Option<Vec2> {
		self.validate_wall_handle(wall).ok()?;
		(0.0..=1.0).contains(&parameter).then(|| {
			self.half_edge_curve(wall.forward)
				.tangent(parameter as f64)
				.as_vec2()
		})
	}

	/// Returns the path length of a live wall, or `None` for a stale handle.
	pub fn wall_length(&self, wall: Wall) -> Option<f64> {
		self.validate_wall_handle(wall).ok()?;
		Some(self.half_edge_curve(wall.forward).length())
	}

	/// Returns the closest point on a live wall to a finite position, with its path parameter.
	pub fn wall_closest_point(&self, wall: Wall, position: Vec2) -> Option<(f32, Vec2)> {
		self.validate_wall_handle(wall).ok()?;
		if !position.is_finite() {
			return None;
		}
		let curve = self.half_edge_curve(wall.forward);
		let parameter = curve.closest_parameter(position.as_dvec2());
		Some((parameter as f32, curve.position(parameter).as_vec2()))
	}

	/// Samples a wall in handle order, including both endpoints, for rendering or export.
	///
	/// `max_deviation` is a positive finite world-space chord error, before rounding to `Vec2`.
	/// Requests needing more than 65,536 segments return [`WallError::InvalidParameter`].
	/// Sampling never introduces graph nodes.
	pub fn sample_wall(&self, wall: Wall, max_deviation: f32) -> Result<Vec<Vec2>, WallError> {
		self.validate_wall_handle(wall)?;
		if !max_deviation.is_finite() || max_deviation <= 0.0 {
			return Err(WallError::InvalidParameter);
		}
		let curve = self.half_edge_curve(wall.forward);
		let count = curve.sample_count(max_deviation as f64);
		if count > 65_536 {
			return Err(WallError::InvalidParameter);
		}
		Ok((0..=count)
			.map(|index| curve.position(index as f64 / count as f64).as_vec2())
			.collect())
	}

	/// The position of a node, or `None` if it isn't a node of this graph.
	pub fn node_position(&self, node_id: WallNodeId) -> Option<Vec2> {
		self.nodes.get(&node_id).map(|node| node.position)
	}
}

/// Nodes are allocated only after the complete insertion plan has passed validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlannedNode {
	Existing(WallNodeId),
	Junction(usize),
}

struct Junction {
	parameter: f64,
	position: Vec2,
	existing_node: Option<WallNodeId>,
}

struct RawContact {
	parameter: f64,
	position: Vec2,
	existing_node: Option<WallNodeId>,
	crossed_wall: Option<Wall>,
}

struct PlannedPiece {
	origin: PlannedNode,
	destination: PlannedNode,
	shape: CurveShape,
}

struct WallInsertionPlan {
	junctions: Vec<Junction>,
	replacements: Vec<(Wall, Vec<PlannedPiece>)>,
	additions: Vec<PlannedPiece>,
}

impl WallGraph {
	fn planned_position(&self, node: PlannedNode, junctions: &[Junction]) -> Vec2 {
		match node {
			PlannedNode::Existing(id) => self.node(id).position,
			PlannedNode::Junction(index) => junctions[index].position,
		}
	}

	fn planned_curve(&self, piece: &PlannedPiece, junctions: &[Junction]) -> Curve {
		Curve::new(
			self.planned_position(piece.origin, junctions),
			self.planned_position(piece.destination, junctions),
			piece.shape,
		)
	}

	fn plan_wall_insertion(
		&self,
		origin_id: WallNodeId,
		destination_id: WallNodeId,
		curve: Curve,
	) -> Result<WallInsertionPlan, WallError> {
		let mut walls: Vec<_> = self.walls().collect();
		walls.sort_by_key(|wall| wall.forward);
		let mut raw_contacts = Vec::new();
		for (node_id, position) in self.nodes() {
			if node_id == origin_id || node_id == destination_id {
				continue;
			}
			if let Some(parameter) = curve.interior_parameter(position.as_dvec2()) {
				raw_contacts.push(RawContact {
					parameter,
					position,
					existing_node: Some(node_id),
					crossed_wall: None,
				});
			}
		}
		for &wall in &walls {
			let existing = self.half_edge_curve(wall.forward);
			let same_endpoints = (wall.origin == origin_id && wall.destination == destination_id)
				|| (wall.origin == destination_id && wall.destination == origin_id);
			if same_endpoints && existing.same_path(curve) {
				return Err(WallError::Duplicate);
			}
			let contacts = intersections(curve, existing);
			if contacts.overlapping {
				return Err(WallError::Overlapping);
			}
			for contact in contacts.contacts {
				let parameter = contact.first_parameter;
				let other_parameter = contact.second_parameter;
				let position = curve.position(parameter).as_vec2();
				if contact.tangential
					&& (curve.interior_parameter(position.as_dvec2()).is_some()
						|| existing.interior_parameter(position.as_dvec2()).is_some())
				{
					return Err(WallError::TangentialContact);
				}
				// Endpoint contacts are represented by the existing-node scan and the endpoint splits below.
				if curve.interior_parameter(position.as_dvec2()).is_some()
					&& existing
						.interior_parameter(existing.position(other_parameter))
						.is_some()
				{
					raw_contacts.push(RawContact {
						parameter,
						position,
						existing_node: None,
						crossed_wall: Some(wall),
					});
				}
			}
		}
		raw_contacts.sort_by(|a, b| {
			a.parameter
				.total_cmp(&b.parameter)
				.then_with(|| a.position.x.total_cmp(&b.position.x))
				.then_with(|| a.position.y.total_cmp(&b.position.y))
				.then_with(|| a.existing_node.cmp(&b.existing_node))
		});
		let mut groups: Vec<Vec<RawContact>> = Vec::new();
		for contact in raw_contacts {
			if let Some(group) = groups
				.last_mut()
				.filter(|group| group[0].position.distance(contact.position) <= DISTANCE_TOLERANCE)
			{
				group.push(contact);
			} else {
				groups.push(vec![contact]);
			}
		}
		let mut junctions = Vec::new();
		for group in groups {
			let representative = group[0].position;
			let existing_node = group
				.iter()
				.filter_map(|contact| contact.existing_node)
				.min_by(|a, b| {
					self.node(*a)
						.position
						.distance(representative)
						.total_cmp(&self.node(*b).position.distance(representative))
						.then_with(|| a.cmp(b))
				});
			let position = existing_node.map_or(representative, |id| self.node(id).position);
			let parameter = curve
				.interior_parameter(position.as_dvec2())
				.ok_or(WallError::InconsistentJunction)?;
			for contact in &group {
				if let Some(wall) = contact.crossed_wall
					&& self
						.half_edge_curve(wall.forward)
						.interior_parameter(position.as_dvec2())
						.is_none()
				{
					return Err(WallError::InconsistentJunction);
				}
			}
			junctions.push(Junction {
				parameter,
				position,
				existing_node,
			});
		}
		junctions.sort_by(|a, b| a.parameter.total_cmp(&b.parameter));
		let junction_node = |index: usize| {
			junctions[index]
				.existing_node
				.map_or(PlannedNode::Junction(index), PlannedNode::Existing)
		};
		let mut path = vec![(0.0, PlannedNode::Existing(origin_id))];
		path.extend(
			junctions
				.iter()
				.enumerate()
				.map(|(index, junction)| (junction.parameter, junction_node(index))),
		);
		path.push((1.0, PlannedNode::Existing(destination_id)));
		let additions = self.plan_pieces(curve, &path, &junctions)?;
		let mut replacements = Vec::new();
		for wall in walls {
			let existing = self.half_edge_curve(wall.forward);
			let mut cuts = vec![(0.0, PlannedNode::Existing(wall.origin))];
			for &(_, node) in &path {
				if let Some(parameter) =
					existing.interior_parameter(self.planned_position(node, &junctions).as_dvec2())
				{
					cuts.push((parameter, node));
				}
			}
			cuts.push((1.0, PlannedNode::Existing(wall.destination)));
			cuts.sort_by(|a, b| a.0.total_cmp(&b.0));
			cuts.dedup_by(|a, b| a.1 == b.1);
			if cuts.len() > 2 {
				replacements.push((wall, self.plan_pieces(existing, &cuts, &junctions)?));
			}
		}
		let plan = WallInsertionPlan {
			junctions,
			replacements,
			additions,
		};
		self.validate_planned_departures(&plan)?;
		Ok(plan)
	}

	fn plan_pieces(
		&self,
		original: Curve,
		cuts: &[(f64, PlannedNode)],
		junctions: &[Junction],
	) -> Result<Vec<PlannedPiece>, WallError> {
		cuts.windows(2)
			.map(|pair| {
				let piece = PlannedPiece {
					origin: pair[0].1,
					destination: pair[1].1,
					shape: original.subcurve(pair[0].0, pair[1].0).shape,
				};
				let curve = self.planned_curve(&piece, junctions);
				if !curve.valid() || curve.length() <= DISTANCE_TOLERANCE as f64 {
					return Err(WallError::ZeroLength);
				}
				// Bound deviation over the entire subcurve, not just a few sampled points.
				if curve.deviation_from(original, pair[0].0, pair[1].0) > DISTANCE_TOLERANCE as f64
				{
					return Err(WallError::InconsistentJunction);
				}
				Ok(piece)
			})
			.collect()
	}

	fn validate_planned_departures(&self, plan: &WallInsertionPlan) -> Result<(), WallError> {
		let mut departures: Vec<(PlannedNode, f32, CurveShape)> = Vec::new();
		let replaced: Vec<_> = plan
			.replacements
			.iter()
			.map(|(wall, _)| wall.forward)
			.collect();
		for piece in plan
			.additions
			.iter()
			.chain(plan.replacements.iter().flat_map(|(_, pieces)| pieces))
		{
			let curve = self.planned_curve(piece, &plan.junctions);
			for (node, direction) in [
				(piece.origin, curve.tangent(0.0)),
				(piece.destination, -curve.tangent(1.0)),
			] {
				let angle = direction.as_vec2().to_angle();
				let reject = |other_angle, other_shape| {
					if angular_distance(angle, other_angle) < DIRECTION_ANGLE_TOLERANCE {
						Err(
							if piece.shape == CurveShape::Straight
								&& other_shape == CurveShape::Straight
							{
								WallError::Overlapping
							} else {
								WallError::TangentialContact
							},
						)
					} else {
						Ok(())
					}
				};
				for &(other_node, other_angle, other_shape) in &departures {
					if node == other_node {
						reject(other_angle, other_shape)?;
					}
				}
				if let PlannedNode::Existing(id) = node {
					for edge in self.outgoing_half_edges(id) {
						if replaced.contains(&edge.min(self.edge(edge).twin)) {
							continue;
						}
						let existing = self.half_edge_curve(edge);
						reject(existing.tangent(0.0).as_vec2().to_angle(), existing.shape)?;
					}
				}
				departures.push((node, angle, piece.shape));
			}
		}
		Ok(())
	}

	fn validate_wall_endpoints(
		&self,
		origin_id: WallNodeId,
		destination_id: WallNodeId,
	) -> Result<(), WallError> {
		self.validate_endpoints_exist(origin_id, destination_id)?;
		if origin_id == destination_id {
			return Err(WallError::SameNode);
		}
		if self
			.node(origin_id)
			.position
			.distance(self.node(destination_id).position)
			<= DISTANCE_TOLERANCE
		{
			return Err(WallError::ZeroLength);
		}
		Ok(())
	}

	fn validate_wall_handle(&self, wall: Wall) -> Result<(), WallError> {
		let forward_matches = self
			.edges
			.get(&wall.forward)
			.is_some_and(|edge| edge.twin == wall.backward && edge.origin == wall.origin);
		let backward_matches = self
			.edges
			.get(&wall.backward)
			.is_some_and(|edge| edge.twin == wall.forward && edge.origin == wall.destination);
		if forward_matches && backward_matches {
			Ok(())
		} else {
			Err(WallError::UnknownWall)
		}
	}

	fn validate_topology(&self) -> Result<(), String> {
		for (&id, edge) in &self.edges {
			if !self.edges.contains_key(&edge.twin)
				|| !self.edges.contains_key(&edge.next)
				|| !self.edges.contains_key(&edge.previous)
			{
				return Err(format!("{id:?} links to a removed half-edge"));
			}
			let Some(origin) = self.nodes.get(&edge.origin) else {
				return Err(format!("{id:?} starts at a removed node"));
			};
			if origin.outgoing_edge.is_none() {
				return Err(format!(
					"{id:?} leaves {:?}, which has no outgoing edge",
					edge.origin
				));
			}
			if self.edge(edge.twin).twin != id {
				return Err(format!("twin of twin of {id:?} is not itself"));
			}
			if self.edge(edge.next).previous != id {
				return Err(format!("previous(next({id:?})) is not itself"));
			}
			if self.edge(edge.previous).next != id {
				return Err(format!("next(previous({id:?})) is not itself"));
			}
			if self.edge(edge.next).origin != self.half_edge_destination(id) {
				return Err(format!("next({id:?}) does not start where {id:?} ends"));
			}
			if self.half_edge_destination(edge.previous) != edge.origin {
				return Err(format!("previous({id:?}) does not end where {id:?} starts"));
			}
		}
		for (&node_id, node) in &self.nodes {
			let starts_elsewhere = node.outgoing_edge.is_some_and(|edge_id| {
				self.edges
					.get(&edge_id)
					.is_none_or(|edge| edge.origin != node_id)
			});
			if starts_elsewhere {
				return Err(format!(
					"outgoing edge of {node_id:?} is missing or does not start at that node"
				));
			}
		}
		Ok(())
	}

	fn validate_geometry(&self) -> Result<(), String> {
		for (&node_id, node) in &self.nodes {
			if !node.position.is_finite() {
				return Err(format!("{node_id:?} has a non-finite position"));
			}
		}
		if self.shapes.len() * 2 != self.edges.len() {
			return Err("wall geometry count does not match half-edge count".into());
		}
		for wall in self.walls() {
			if !self.shapes.contains_key(&wall.forward) {
				return Err(format!("{wall:?} has no geometry"));
			}
			let curve = self.half_edge_curve(wall.forward);
			if !curve.valid() || curve.length() <= DISTANCE_TOLERANCE as f64 {
				return Err(format!("{wall:?} has invalid geometry"));
			}
		}
		for &node_id in self.nodes.keys() {
			let mut departures: Vec<_> = self
				.edges
				.iter()
				.filter(|(_, edge)| edge.origin == node_id)
				.map(|(&id, _)| {
					(
						self.half_edge_curve(id).tangent(0.0).as_vec2().to_angle(),
						id,
					)
				})
				.collect();
			departures.sort_by(|a, b| a.0.total_cmp(&b.0));
			for (index, &(angle, edge)) in departures.iter().enumerate() {
				let previous = departures[(index + departures.len() - 1) % departures.len()];
				if self.next_outgoing_half_edge_clockwise(edge) != previous.1 {
					return Err(format!("{node_id:?} departures are not ordered by tangent"));
				}
				if departures.len() > 1
					&& angular_distance(angle, previous.0) < DIRECTION_ANGLE_TOLERANCE
				{
					return Err(format!("{node_id:?} has ambiguous departure tangents"));
				}
			}
		}
		Ok(())
	}

	fn validate_endpoints_exist(
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
}

#[cfg(test)]
#[path = "graph_tests.rs"]
mod tests;
