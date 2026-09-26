use std::collections::HashMap;
use std::f32::consts::TAU;
use std::fmt;

use glam::Vec2;

/// Departures from a node within this many radians are treated as collinear.
const DIRECTION_ANGLE_TOLERANCE: f32 = 1e-6;

/// Points at most this far apart count as the same point, and a point this close to a line lies on it.
const DISTANCE_TOLERANCE: f32 = 1e-4;

/// Identifies a node in one [`WallGraph`].
///
/// The identifier is opaque and never reused, but becomes invalid if its node is removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WallNodeId(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct HalfEdgeId(usize);

/// An error caused by a proposed mutation of a [`WallGraph`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WallError {
	/// A node position contains a non-finite coordinate.
	InvalidPosition,
	/// An endpoint is not a node of this graph.
	UnknownNode,
	/// The wall would start and end at the same node.
	SameNode,
	/// The two nodes are already joined by a wall.
	Duplicate,
	/// The wall runs along part of an existing wall.
	Overlapping,
	/// The endpoint positions are within the graph's distance tolerance.
	ZeroLength,
	/// The wall is not a wall of this graph, for example because it was already removed.
	UnknownWall,
}

impl fmt::Display for WallError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let message = match self {
			WallError::InvalidPosition => "a node position must contain only finite coordinates",
			WallError::UnknownNode => "a wall endpoint is not a node of this graph",
			WallError::SameNode => "a wall cannot start and end at the same node",
			WallError::Duplicate => "there is already a wall between these nodes",
			WallError::Overlapping => "the wall runs along part of an existing wall",
			WallError::ZeroLength => "a wall cannot join nodes within the distance tolerance",
			WallError::UnknownWall => "the wall is not a wall of this graph",
		};
		f.write_str(message)
	}
}

impl std::error::Error for WallError {}

fn direction_angle_between(from: Vec2, to: Vec2) -> f32 {
	(to - from).to_angle()
}

fn angular_distance(a: f32, b: f32) -> f32 {
	let difference = (a - b).abs();
	difference.min(TAU - difference)
}

/// A point's closest location on the line through `start` and `end`, which must be distinct.
#[derive(Debug, Clone, Copy)]
struct LineProjection {
	line_parameter: f32,
	perpendicular_distance: f32,
}

fn project_point_onto_line(point: Vec2, start: Vec2, end: Vec2) -> LineProjection {
	let line = end - start;
	let line_parameter = (point - start).dot(line) / line.length_squared();
	LineProjection {
		line_parameter,
		perpendicular_distance: point.distance(start + line * line_parameter),
	}
}

/// Whether `point` lies on the interior of the segment, excluding points within tolerance of either end.
fn segment_strictly_contains_point(point: Vec2, start: Vec2, end: Vec2) -> bool {
	let projection = project_point_onto_line(point, start, end);
	projection.perpendicular_distance <= DISTANCE_TOLERANCE
		&& (0.0..=1.0).contains(&projection.line_parameter)
		&& point.distance(start) > DISTANCE_TOLERANCE
		&& point.distance(end) > DISTANCE_TOLERANCE
}

/// Whether segments `a` and `b` are collinear and share a stretch longer than the distance tolerance.
fn segments_have_collinear_overlap(a: (Vec2, Vec2), b: (Vec2, Vec2)) -> bool {
	// Measure against the longer segment: a very short one gives a poorly defined line.
	let (long, short) = if a.0.distance(a.1) >= b.0.distance(b.1) {
		(a, b)
	} else {
		(b, a)
	};
	let first = project_point_onto_line(short.0, long.0, long.1);
	let second = project_point_onto_line(short.1, long.0, long.1);
	if first.perpendicular_distance > DISTANCE_TOLERANCE
		|| second.perpendicular_distance > DISTANCE_TOLERANCE
	{
		return false;
	}
	let low = first.line_parameter.min(second.line_parameter).max(0.0);
	let high = first.line_parameter.max(second.line_parameter).min(1.0);
	(high - low) * long.0.distance(long.1) > DISTANCE_TOLERANCE
}

/// The normalized parameters where segments `a` and `b` meet, including endpoint contact.
///
/// Returns `None` when they are parallel, including when they are collinear, or when they do not meet.
fn segment_intersection_parameters(a: (Vec2, Vec2), b: (Vec2, Vec2)) -> Option<(f32, f32)> {
	let a_direction = a.1 - a.0;
	let b_direction = b.1 - b.0;
	let denominator = a_direction.perp_dot(b_direction);
	if denominator.abs() <= f32::EPSILON * a_direction.length() * b_direction.length() {
		return None;
	}
	let between_starts = b.0 - a.0;
	let along_a = between_starts.perp_dot(b_direction) / denominator;
	let along_b = between_starts.perp_dot(a_direction) / denominator;
	let within = |along: f32| (0.0..=1.0).contains(&along);
	(within(along_a) && within(along_b)).then_some((along_a, along_b))
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
#[derive(Debug)]
struct HalfEdge {
	origin: WallNodeId,
	twin: HalfEdgeId,
	next: HalfEdgeId,
	previous: HalfEdgeId,
}

/// A handle to one wall in a [`WallGraph`].
///
/// A handle becomes invalid when its wall is removed or split. Its endpoints remain available so
/// callers can interpret results without borrowing the graph again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Wall {
	forward: HalfEdgeId,
	backward: HalfEdgeId,
	origin: WallNodeId,
	destination: WallNodeId,
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

/// A planar graph of wall corners joined by straight walls.
///
/// Adding a wall automatically divides it and any crossed walls at their junctions. Geometric
/// comparisons use a fixed tolerance of `0.0001` coordinate units, so resulting wall pieces are
/// always longer than that tolerance. Removing a wall also removes either endpoint if no other wall
/// still reaches it.
#[derive(Debug, Default)]
pub struct WallGraph {
	next_node_id: usize,
	next_edge_id: usize,
	nodes: HashMap<WallNodeId, WallNode>,
	edges: HashMap<HalfEdgeId, HalfEdge>,
}

impl WallGraph {
	/// Creates an empty graph.
	pub fn new() -> Self {
		Self::default()
	}

	fn allocate_node_id(&mut self) -> WallNodeId {
		let id = self.next_node_id;
		self.next_node_id = self
			.next_node_id
			.checked_add(1)
			.expect("WallNodeId space exhausted");

		WallNodeId(id)
	}

	fn allocate_edge_id(&mut self) -> HalfEdgeId {
		let id = self.next_edge_id;
		self.next_edge_id = self
			.next_edge_id
			.checked_add(1)
			.expect("HalfEdgeId space exhausted");

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
	///
	/// Distinct nodes may occupy the same position, but a wall cannot directly join them.
	/// Returns [`WallError::InvalidPosition`] without changing the graph if either coordinate is not
	/// finite.
	pub fn add_node(&mut self, position: Vec2) -> Result<WallNodeId, WallError> {
		if !position.is_finite() {
			return Err(WallError::InvalidPosition);
		}
		Ok(self.insert_node(position))
	}

	fn insert_node(&mut self, position: Vec2) -> WallNodeId {
		debug_assert!(position.is_finite());
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

	fn direction_angle_between_nodes(&self, from: WallNodeId, to: WallNodeId) -> f32 {
		direction_angle_between(self.node(from).position, self.node(to).position)
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
					self.direction_angle_between_nodes(
						node_id,
						self.half_edge_destination(edge_id),
					),
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

	/// Adds a straight wall between two existing nodes.
	///
	/// The operation is atomic: on error, the graph is unchanged. Existing walls are split where
	/// the new wall crosses them or where an endpoint lies on their interior. The new wall is also
	/// split at every existing node it passes through. The returned walls cover the resulting path
	/// in order from `origin_id` to `destination_id`.
	pub fn add_wall(
		&mut self,
		origin_id: WallNodeId,
		destination_id: WallNodeId,
	) -> Result<Vec<Wall>, WallError> {
		self.validate_wall_endpoints(origin_id, destination_id)?;
		let plan = self.plan_wall_insertion(origin_id, destination_id)?;

		for (wall, node_id) in plan.wall_splits_at_existing_nodes {
			self.split_wall_at_node(wall, node_id);
		}

		let mut path = vec![origin_id];
		for breakpoint in plan.path_breakpoints {
			path.push(match breakpoint {
				PathBreakpoint::ExistingNode(node_id) => node_id,
				PathBreakpoint::Crossing { walls, position } => {
					let node_id = self.insert_node(position);
					for wall in walls {
						self.split_wall_at_node(wall, node_id);
					}
					node_id
				}
			});
		}
		path.push(destination_id);

		let walls = path
			.windows(2)
			.map(|pair| self.insert_wall_unchecked(pair[0], pair[1]))
			.collect();

		debug_assert_eq!(self.validate_topology(), Ok(()));
		debug_assert_eq!(self.validate_geometry(), Ok(()));

		Ok(walls)
	}

	/// Removes a wall and any endpoint left with no remaining walls.
	///
	/// Removing a wall invalidates its handle. Nodes that were already isolated are unaffected.
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

	/// Replaces `wall` with two walls meeting at an existing node on its interior.
	///
	/// Callers must ensure the wall and node are valid; the original wall handle is invalidated.
	fn split_wall_at_node(&mut self, wall: Wall, node_id: WallNodeId) {
		self.remove_wall_unchecked(wall);
		self.insert_wall_unchecked(wall.origin, node_id);
		self.insert_wall_unchecked(node_id, wall.destination);
	}

	fn remove_wall_unchecked(&mut self, wall: Wall) {
		self.detach_wall_end_from_node_ring(wall.forward, wall.backward);
		self.detach_wall_end_from_node_ring(wall.backward, wall.forward);
		self.edges.remove(&wall.forward);
		self.edges.remove(&wall.backward);
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
	fn insert_wall_unchecked(&mut self, origin_id: WallNodeId, destination_id: WallNodeId) -> Wall {
		let forward_id = self.allocate_edge_id();
		let backward_id = self.allocate_edge_id();

		let origin_gap = self.find_node_ring_gap(
			origin_id,
			self.direction_angle_between_nodes(origin_id, destination_id),
		);
		let destination_gap = self.find_node_ring_gap(
			destination_id,
			self.direction_angle_between_nodes(destination_id, origin_id),
		);

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

		Wall {
			forward: forward_id,
			backward: backward_id,
			origin: origin_id,
			destination: destination_id,
		}
	}

	/// Iterates over every node and its position, in no particular order.
	pub fn nodes(&self) -> impl Iterator<Item = (WallNodeId, Vec2)> + '_ {
		self.nodes.iter().map(|(&id, node)| (id, node.position))
	}

	/// Iterates over every wall exactly once, in no particular order.
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

	/// The position of a node, or `None` if it isn't a node of this graph.
	pub fn node_position(&self, node_id: WallNodeId) -> Option<Vec2> {
		self.nodes.get(&node_id).map(|node| node.position)
	}
}

/// A point along a new wall where it must be divided.
enum PathBreakpoint {
	ExistingNode(WallNodeId),
	Crossing { walls: Vec<Wall>, position: Vec2 },
}

struct WallInsertionPlan {
	wall_splits_at_existing_nodes: Vec<(Wall, WallNodeId)>,
	path_breakpoints: Vec<PathBreakpoint>,
}

struct BreakpointGroup {
	representative_position: Vec2,
	existing_nodes: Vec<WallNodeId>,
	crossing_walls: Vec<Wall>,
}

impl WallGraph {
	fn plan_wall_insertion(
		&self,
		origin_id: WallNodeId,
		destination_id: WallNodeId,
	) -> Result<WallInsertionPlan, WallError> {
		let start = self.node(origin_id).position;
		let end = self.node(destination_id).position;
		let is_end = |node_id: WallNodeId| node_id == origin_id || node_id == destination_id;

		let mut raw_breakpoints: Vec<(f32, PathBreakpoint)> = self
			.nodes()
			.filter(|&(node_id, position)| {
				!is_end(node_id) && segment_strictly_contains_point(position, start, end)
			})
			.map(|(node_id, position)| {
				let line_parameter = project_point_onto_line(position, start, end).line_parameter;
				(line_parameter, PathBreakpoint::ExistingNode(node_id))
			})
			.collect();
		let mut wall_splits_at_existing_nodes = Vec::new();

		for wall in self.walls() {
			let wall_start = self.node(wall.origin).position;
			let wall_end = self.node(wall.destination).position;
			if segments_have_collinear_overlap((start, end), (wall_start, wall_end)) {
				return Err(WallError::Overlapping);
			}
			// Other than overlapping, a wall sharing a node with the new one can only meet it there.
			if is_end(wall.origin) || is_end(wall.destination) {
				continue;
			}

			let mut end_on_wall = false;
			for (node_id, position) in [(origin_id, start), (destination_id, end)] {
				if segment_strictly_contains_point(position, wall_start, wall_end) {
					wall_splits_at_existing_nodes.push((wall, node_id));
					end_on_wall = true;
				}
			}
			if end_on_wall {
				continue;
			}

			if let Some((line_parameter, _)) =
				segment_intersection_parameters((start, end), (wall_start, wall_end))
			{
				let position = start.lerp(end, line_parameter);
				// Contact within tolerance of an endpoint is represented by that existing node.
				let clear_of_endpoints = [start, end, wall_start, wall_end]
					.iter()
					.all(|endpoint| endpoint.distance(position) > DISTANCE_TOLERANCE);
				if clear_of_endpoints {
					raw_breakpoints.push((
						line_parameter,
						PathBreakpoint::Crossing {
							walls: vec![wall],
							position,
						},
					));
				}
			}
		}

		let path_breakpoints =
			self.normalize_path_breakpoints(raw_breakpoints, &mut wall_splits_at_existing_nodes)?;
		let mut path_positions = vec![start];
		path_positions.extend(path_breakpoints.iter().map(|breakpoint| match breakpoint {
			PathBreakpoint::ExistingNode(node_id) => self.node(*node_id).position,
			PathBreakpoint::Crossing { position, .. } => *position,
		}));
		path_positions.push(end);
		if path_positions
			.windows(2)
			.any(|pair| pair[0].distance(pair[1]) <= DISTANCE_TOLERANCE)
		{
			return Err(WallError::ZeroLength);
		}
		Ok(WallInsertionPlan {
			wall_splits_at_existing_nodes,
			path_breakpoints,
		})
	}

	fn normalize_path_breakpoints(
		&self,
		mut raw_breakpoints: Vec<(f32, PathBreakpoint)>,
		wall_splits_at_existing_nodes: &mut Vec<(Wall, WallNodeId)>,
	) -> Result<Vec<PathBreakpoint>, WallError> {
		let position_of = |breakpoint: &PathBreakpoint| match breakpoint {
			PathBreakpoint::ExistingNode(node_id) => self.node(*node_id).position,
			PathBreakpoint::Crossing { position, .. } => *position,
		};
		raw_breakpoints.sort_by(|a, b| {
			let a_position = position_of(&a.1);
			let b_position = position_of(&b.1);
			a.0.total_cmp(&b.0)
				.then_with(|| a_position.x.total_cmp(&b_position.x))
				.then_with(|| a_position.y.total_cmp(&b_position.y))
		});
		let mut groups: Vec<BreakpointGroup> = Vec::new();

		for (_, breakpoint) in raw_breakpoints {
			let position = match &breakpoint {
				PathBreakpoint::ExistingNode(node_id) => self.node(*node_id).position,
				PathBreakpoint::Crossing { position, .. } => *position,
			};
			let group = groups.last_mut().filter(|group| {
				group.representative_position.distance(position) <= DISTANCE_TOLERANCE
			});
			let group = match group {
				Some(group) => group,
				None => {
					groups.push(BreakpointGroup {
						representative_position: position,
						existing_nodes: Vec::new(),
						crossing_walls: Vec::new(),
					});
					groups.last_mut().unwrap()
				}
			};
			match breakpoint {
				PathBreakpoint::ExistingNode(node_id) => group.existing_nodes.push(node_id),
				PathBreakpoint::Crossing { walls, .. } => {
					group.crossing_walls.extend(walls);
				}
			}
		}

		groups
			.into_iter()
			.map(|mut group| {
				group.existing_nodes.sort_by(|a, b| {
					let a_distance = self
						.node(*a)
						.position
						.distance(group.representative_position);
					let b_distance = self
						.node(*b)
						.position
						.distance(group.representative_position);
					a_distance.total_cmp(&b_distance).then_with(|| a.cmp(b))
				});
				group.existing_nodes.dedup();
				if let Some(&node_id) = group.existing_nodes.first() {
					let position = self.node(node_id).position;
					for wall in group.crossing_walls {
						let wall_start = self.node(wall.origin).position;
						let wall_end = self.node(wall.destination).position;
						if !segment_strictly_contains_point(position, wall_start, wall_end) {
							return Err(WallError::ZeroLength);
						}
						if !wall_splits_at_existing_nodes.contains(&(wall, node_id)) {
							wall_splits_at_existing_nodes.push((wall, node_id));
						}
					}
					Ok(PathBreakpoint::ExistingNode(node_id))
				} else {
					let position = group.representative_position;
					let mut walls = Vec::new();
					for wall in group.crossing_walls {
						let wall_start = self.node(wall.origin).position;
						let wall_end = self.node(wall.destination).position;
						if !segment_strictly_contains_point(position, wall_start, wall_end) {
							return Err(WallError::ZeroLength);
						}
						if !walls.contains(&wall) {
							walls.push(wall);
						}
					}
					Ok(PathBreakpoint::Crossing { walls, position })
				}
			})
			.collect()
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
		self.reject_duplicate_wall(origin_id, destination_id)?;
		self.reject_collinear_departure(origin_id, destination_id)?;
		self.reject_collinear_departure(destination_id, origin_id)
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
		for wall in self.walls() {
			let length = self
				.node(wall.origin)
				.position
				.distance(self.node(wall.destination).position);
			if length <= DISTANCE_TOLERANCE {
				return Err(format!("{wall:?} is shorter than the distance tolerance"));
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

	fn reject_duplicate_wall(
		&self,
		origin_id: WallNodeId,
		destination_id: WallNodeId,
	) -> Result<(), WallError> {
		let exists = self
			.outgoing_half_edges(origin_id)
			.into_iter()
			.any(|edge_id| self.half_edge_destination(edge_id) == destination_id);
		if exists {
			Err(WallError::Duplicate)
		} else {
			Ok(())
		}
	}

	fn reject_collinear_departure(
		&self,
		node_id: WallNodeId,
		toward_id: WallNodeId,
	) -> Result<(), WallError> {
		let angle = self.direction_angle_between_nodes(node_id, toward_id);
		let overlaps = self
			.outgoing_half_edges(node_id)
			.into_iter()
			.any(|edge_id| {
				let existing_angle = self
					.direction_angle_between_nodes(node_id, self.half_edge_destination(edge_id));
				angular_distance(angle, existing_angle) < DIRECTION_ANGLE_TOLERANCE
			});
		if overlaps {
			Err(WallError::Overlapping)
		} else {
			Ok(())
		}
	}
}

#[cfg(test)]
#[path = "graph_tests.rs"]
mod tests;
