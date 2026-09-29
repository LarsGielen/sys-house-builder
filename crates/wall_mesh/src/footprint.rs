//! Resolves walls and junctions into planar cells that tile the footprint exactly.
//!
//! Each wall becomes body cells between its junction trims and opening edges. Each junction
//! with two or more walls becomes one core cell. Neighbouring cells share vertex IDs along their
//! common edges, so later stages find adjacency by matching reversed edges.

use std::collections::HashSet;
use std::f64::consts::{PI, TAU};

use glam::DVec2;

use crate::intervals::Intervals;
use crate::path::{Path, intersections};
use crate::snapshot::Snapshot;
use crate::{MeshError, MeshSettings, Side, SurfaceSource};

/// Positions closer than this are the same footprint vertex.
const VERTEX_TOLERANCE: f64 = 1e-7;
/// The wall graph's distance tolerance, used when comparing its path distances.
const GRAPH_TOLERANCE: f64 = 1e-4;
/// Junction cores with less area, in square metres, have only collinear cross-sections.
const MIN_CORE_AREA: f64 = 1e-12;

pub(crate) struct Footprint {
	pub(crate) vertices: Vec<DVec2>,
	pub(crate) cells: Vec<Cell>,
}

/// A simple counterclockwise polygon with solid material over some elevation intervals.
pub(crate) struct Cell {
	pub(crate) boundary: Vec<usize>,
	/// `edges[k]` runs from `boundary[k]` to the following vertex.
	pub(crate) edges: Vec<EdgeKind>,
	pub(crate) solid: Intervals,
	pub(crate) kind: CellKind,
}

pub(crate) enum CellKind {
	/// A stretch of one wall. `openings` index the wall's openings that span the whole stretch.
	Body {
		wall: usize,
		openings: Vec<usize>,
	},
	Core {
		node: usize,
	},
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum EdgeKind {
	/// Runs along a wall face. `center` is set on curved faces for smooth normals.
	Side {
		wall: usize,
		side: Side,
		center: Option<DVec2>,
	},
	/// Crosses a wall's thickness. `node` is set where the wall meets a junction core.
	CrossSection { wall: usize, node: Option<usize> },
	/// Cuts off a sharp outside corner, or steps between walls of different thickness.
	Bevel { node: usize },
}

impl Cell {
	pub(crate) fn edge_vertices(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
		let count = self.boundary.len();
		(0..count).map(move |k| (self.boundary[k], self.boundary[(k + 1) % count]))
	}

	pub(crate) fn source(&self, snapshot: &Snapshot) -> SurfaceSource {
		match self.kind {
			CellKind::Body { wall, .. } => SurfaceSource::Wall(snapshot.walls[wall].handle),
			CellKind::Core { node } => SurfaceSource::Junction(snapshot.nodes[node].id),
		}
	}
}

pub(crate) fn build(snapshot: &Snapshot, settings: &MeshSettings) -> Result<Footprint, MeshError> {
	for wall in &snapshot.walls {
		let inner_radius = [wall.half_thickness, -wall.half_thickness]
			.into_iter()
			.filter_map(|offset| wall.path.offset_radius(offset))
			.fold(f64::INFINITY, f64::min);
		if inner_radius <= GRAPH_TOLERANCE {
			return Err(MeshError::ArcTooTight { wall: wall.handle });
		}
	}

	let mut builder = Builder {
		snapshot,
		settings,
		vertices: Vec::new(),
		vertex_junction: Vec::new(),
		junction_vertices: vec![Vec::new(); snapshot.nodes.len()],
		cells: Vec::new(),
	};
	let mut origin_ends = vec![None; snapshot.walls.len()];
	let mut destination_ends = vec![None; snapshot.walls.len()];
	for node in 0..snapshot.nodes.len() {
		if snapshot.nodes[node].walls.len() < 2 {
			continue;
		}
		for (wall, at_origin, end) in builder.resolve_junction(node)? {
			if at_origin {
				origin_ends[wall] = Some(end);
			} else {
				destination_ends[wall] = Some(end);
			}
		}
	}
	for (index, wall) in snapshot.walls.iter().enumerate() {
		let origin = origin_ends[index].unwrap_or_else(|| builder.free_end(index, 0.0));
		let destination =
			destination_ends[index].unwrap_or_else(|| builder.free_end(index, wall.path.length()));
		builder.add_bodies(index, origin, destination)?;
	}
	builder.split_junction_edges_at_shared_vertices();
	builder.check_overlaps()?;
	Ok(Footprint {
		vertices: builder.vertices,
		cells: builder.cells,
	})
}

/// Where a wall body starts or ends, in the wall's own direction.
#[derive(Debug, Clone, Copy)]
struct Station {
	distance: f64,
	right: usize,
	left: usize,
	node: Option<usize>,
}

/// A wall seen from a junction, with its path directed away from the node.
struct Departure {
	wall: usize,
	at_origin: bool,
	path: Path,
	half_thickness: f64,
}

impl Departure {
	/// Maps a side relative to the outward direction onto the wall handle's side.
	fn side_edge(&self, outward: Side) -> EdgeKind {
		EdgeKind::Side {
			wall: self.wall,
			side: if self.at_origin {
				outward
			} else {
				outward.opposite()
			},
			center: self.path.center(),
		}
	}
}

/// How the boundary between two neighbouring departures is closed.
#[derive(Debug, Clone, Copy)]
struct Corner {
	/// Distance along the first departure's left face where the corner starts.
	left_distance: f64,
	/// Distance along the second departure's right face where the corner ends.
	right_distance: f64,
	bevel: bool,
}

struct Builder<'a> {
	snapshot: &'a Snapshot,
	settings: &'a MeshSettings,
	vertices: Vec<DVec2>,
	vertex_junction: Vec<Option<usize>>,
	junction_vertices: Vec<Vec<usize>>,
	cells: Vec<Cell>,
}

impl Builder<'_> {
	fn add_vertex(&mut self, position: DVec2, junction: Option<usize>) -> usize {
		self.vertices.push(position);
		self.vertex_junction.push(junction);
		self.vertices.len() - 1
	}

	/// Reuses a coincident vertex of the same junction, so miter points and neighbouring
	/// cross-section ends become one shared vertex.
	fn junction_vertex(&mut self, node: usize, position: DVec2) -> usize {
		if let Some(&existing) = self.junction_vertices[node]
			.iter()
			.find(|&&vertex| self.vertices[vertex].distance(position) <= VERTEX_TOLERANCE)
		{
			return existing;
		}
		let vertex = self.add_vertex(position, Some(node));
		self.junction_vertices[node].push(vertex);
		vertex
	}

	fn free_end(&mut self, wall: usize, distance: f64) -> Station {
		let snapshot = self.snapshot;
		let half = snapshot.walls[wall].half_thickness;
		let path = snapshot.walls[wall].path;
		Station {
			distance,
			right: self.add_vertex(path.point(distance, -half), None),
			left: self.add_vertex(path.point(distance, half), None),
			node: None,
		}
	}

	/// Builds the core cell of one junction and returns each wall's trimmed end.
	fn resolve_junction(&mut self, node: usize) -> Result<Vec<(usize, bool, Station)>, MeshError> {
		let snapshot = self.snapshot;
		let position = snapshot.nodes[node].position;
		let departures: Vec<Departure> = snapshot.nodes[node]
			.walls
			.iter()
			.map(|&wall| {
				let wall_snapshot = &snapshot.walls[wall];
				let at_origin = wall_snapshot.origin == node;
				Departure {
					wall,
					at_origin,
					path: if at_origin {
						wall_snapshot.path
					} else {
						wall_snapshot.path.reversed()
					},
					half_thickness: wall_snapshot.half_thickness,
				}
			})
			.collect();
		let count = departures.len();
		let corners: Vec<Corner> = (0..count)
			.map(|k| self.corner(&departures[k], &departures[(k + 1) % count], position))
			.collect();
		let trims: Vec<f64> = (0..count)
			.map(|k| {
				0.0f64
					.max(corners[k].left_distance)
					.max(corners[(k + count - 1) % count].right_distance)
			})
			.collect();

		// Walk the core boundary counterclockwise. Each point carries the edge leaving it.
		let deviation = self.settings.max_deviation;
		let mut points: Vec<(DVec2, EdgeKind)> = Vec::new();
		let mut cross_sections = Vec::with_capacity(count);
		for k in 0..count {
			let departure = &departures[k];
			let next_index = (k + 1) % count;
			let next = &departures[next_index];
			let corner = corners[k];
			let half = departure.half_thickness;
			cross_sections.push(points.len());
			points.push((
				departure.path.point(trims[k], -half),
				EdgeKind::CrossSection {
					wall: departure.wall,
					node: Some(node),
				},
			));
			let left_edge = departure.side_edge(Side::Left);
			for distance in
				departure
					.path
					.samples(trims[k], corner.left_distance, half, deviation)?
			{
				points.push((departure.path.point(distance, half), left_edge));
			}
			if corner.bevel {
				points.last_mut().expect("left face was sampled").1 = EdgeKind::Bevel { node };
			}
			let next_half = next.half_thickness;
			let right_edge = next.side_edge(Side::Right);
			for distance in next.path.samples(
				corner.right_distance,
				trims[next_index],
				-next_half,
				deviation,
			)? {
				points.push((next.path.point(distance, -next_half), right_edge));
			}
		}

		let ids: Vec<usize> = points
			.iter()
			.map(|&(point, _)| self.junction_vertex(node, point))
			.collect();
		let ends = departures
			.iter()
			.zip(&cross_sections)
			.zip(&trims)
			.map(|((departure, &index), &trim)| {
				let (outward_right, outward_left) = (ids[index], ids[index + 1]);
				let wall_length = snapshot.walls[departure.wall].path.length();
				let station = if departure.at_origin {
					Station {
						distance: trim,
						right: outward_right,
						left: outward_left,
						node: Some(node),
					}
				} else {
					Station {
						distance: wall_length - trim,
						right: outward_left,
						left: outward_right,
						node: Some(node),
					}
				};
				(departure.wall, departure.at_origin, station)
			})
			.collect();

		// Coincident points collapse; the surviving point takes the later edge kind because the
		// zero-length edge between them disappears.
		let mut boundary: Vec<usize> = Vec::with_capacity(points.len());
		let mut edges: Vec<EdgeKind> = Vec::with_capacity(points.len());
		for (&id, &(_, kind)) in ids.iter().zip(&points) {
			if boundary.last() == Some(&id) {
				*edges.last_mut().expect("edges follow boundary") = kind;
			} else {
				boundary.push(id);
				edges.push(kind);
			}
		}
		while boundary.len() > 1 && boundary.last() == boundary.first() {
			boundary.pop();
			edges.pop();
		}
		let area = polygon_area(&self.vertices, &boundary);
		if area < -MIN_CORE_AREA {
			return Err(MeshError::UnresolvedJunction {
				node: snapshot.nodes[node].id,
			});
		}
		if area > MIN_CORE_AREA {
			let height = departures
				.iter()
				.map(|departure| snapshot.walls[departure.wall].height)
				.fold(0.0, f64::max);
			self.cells.push(Cell {
				boundary,
				edges,
				solid: Intervals::span(0.0, height),
				kind: CellKind::Core { node },
			});
		}
		Ok(ends)
	}

	/// Joins `first`'s left face to `second`'s right face across the sector between them.
	fn corner(&self, first: &Departure, second: &Departure, node: DVec2) -> Corner {
		let angle = (second.path.tangent(0.0).to_angle() - first.path.tangent(0.0).to_angle())
			.rem_euclid(TAU);
		let reflex = angle > PI;
		let widest = first.half_thickness.max(second.half_thickness);
		let miter = intersections(
			first.path.offset_curve(first.half_thickness),
			second.path.offset_curve(-second.half_thickness),
		)
		.into_iter()
		.min_by(|a, b| {
			a.distance_squared(node)
				.total_cmp(&b.distance_squared(node))
		})
		.and_then(|point| {
			let left_distance = first.path.distance_near(point, 0.0);
			let right_distance = second.path.distance_near(point, 0.0);
			// An outside miter lies behind both walls and an inside one ahead of both. Otherwise
			// the faces only meet on an extension of one wall, as near-straight joins of walls
			// with different thickness do, and the join becomes a step instead.
			let valid = if reflex {
				left_distance <= VERTEX_TOLERANCE
					&& right_distance <= VERTEX_TOLERANCE
					&& point.distance(node) <= self.settings.miter_limit * widest
			} else {
				left_distance >= -VERTEX_TOLERANCE && right_distance >= -VERTEX_TOLERANCE
			};
			valid.then_some(Corner {
				left_distance,
				right_distance,
				bevel: false,
			})
		});
		miter.unwrap_or_else(|| {
			// Inside a convex sector, trimming both walls this far keeps their cross-sections
			// from crossing. Outside sectors need no trim.
			let trim = if reflex {
				0.0
			} else {
				widest / (angle * 0.5).tan()
			};
			// A straight continuation gives a trim of about 1e-17; zero keeps its cross-sections
			// exactly collinear.
			let trim = if trim <= VERTEX_TOLERANCE { 0.0 } else { trim };
			Corner {
				left_distance: trim,
				right_distance: trim,
				bevel: true,
			}
		})
	}

	fn add_bodies(
		&mut self,
		wall: usize,
		origin: Station,
		destination: Station,
	) -> Result<(), MeshError> {
		let all = self.snapshot;
		let snapshot = &all.walls[wall];
		if destination.distance - origin.distance <= GRAPH_TOLERANCE {
			return Err(MeshError::JunctionTrimsOverlap {
				wall: snapshot.handle,
			});
		}
		let minimum_pier = self.settings.min_pier_width - GRAPH_TOLERANCE;
		for opening in &snapshot.openings {
			if (origin.node.is_some() && opening.start - origin.distance < minimum_pier)
				|| (destination.node.is_some() && destination.distance - opening.end < minimum_pier)
			{
				return Err(MeshError::OpeningTooCloseToJunction {
					opening: opening.id,
				});
			}
		}

		let mut distances: Vec<f64> = snapshot
			.openings
			.iter()
			.flat_map(|opening| [opening.start, opening.end])
			.filter(|&distance| {
				distance > origin.distance + GRAPH_TOLERANCE
					&& distance < destination.distance - GRAPH_TOLERANCE
			})
			.collect();
		distances.sort_by(f64::total_cmp);
		distances.dedup_by(|later, earlier| *later - *earlier <= GRAPH_TOLERANCE);
		let mut stations = Vec::with_capacity(distances.len() + 2);
		stations.push(origin);
		for distance in distances {
			stations.push(self.free_end(wall, distance));
		}
		stations.push(destination);
		for pair in stations.windows(2) {
			self.add_body(wall, pair[0], pair[1])?;
		}
		Ok(())
	}

	fn add_body(&mut self, wall: usize, from: Station, to: Station) -> Result<(), MeshError> {
		let all = self.snapshot;
		let snapshot = &all.walls[wall];
		let path = snapshot.path;
		let half = snapshot.half_thickness;
		// Both faces use the same distances so every chord pair spans the same angle.
		let samples = path.samples(
			from.distance,
			to.distance,
			path.outer_offset(half),
			self.settings.max_deviation,
		)?;
		let interior = &samples[1..samples.len() - 1];
		let right: Vec<usize> = interior
			.iter()
			.map(|&distance| self.add_vertex(path.point(distance, -half), None))
			.collect();
		let left: Vec<usize> = interior
			.iter()
			.map(|&distance| self.add_vertex(path.point(distance, half), None))
			.collect();

		let side = |side| EdgeKind::Side {
			wall,
			side,
			center: path.center(),
		};
		let mut boundary = Vec::with_capacity(2 * interior.len() + 4);
		let mut edges = Vec::with_capacity(boundary.capacity());
		boundary.push(from.right);
		edges.push(side(Side::Right));
		for &vertex in &right {
			boundary.push(vertex);
			edges.push(side(Side::Right));
		}
		boundary.push(to.right);
		edges.push(EdgeKind::CrossSection {
			wall,
			node: to.node,
		});
		boundary.push(to.left);
		edges.push(side(Side::Left));
		for &vertex in left.iter().rev() {
			boundary.push(vertex);
			edges.push(side(Side::Left));
		}
		boundary.push(from.left);
		edges.push(EdgeKind::CrossSection {
			wall,
			node: from.node,
		});

		let mut solid = Intervals::span(0.0, snapshot.height);
		let mut openings = Vec::new();
		for (index, opening) in snapshot.openings.iter().enumerate() {
			if opening.start <= from.distance + GRAPH_TOLERANCE
				&& opening.end >= to.distance - GRAPH_TOLERANCE
			{
				solid = solid.subtract(opening.bottom, opening.top);
				openings.push(index);
			}
		}
		self.cells.push(Cell {
			boundary,
			edges,
			solid,
			kind: CellKind::Body { wall, openings },
		});
		Ok(())
	}

	/// Splits straight junction edges at other vertices of the same junction lying on them.
	///
	/// Collinear walls of different thickness meet without a core, so the wider wall's
	/// cross-section must be divided where the narrower one's ends touch it.
	fn split_junction_edges_at_shared_vertices(&mut self) {
		for cell in &mut self.cells {
			let count = cell.boundary.len();
			let mut boundary = Vec::with_capacity(count);
			let mut edges = Vec::with_capacity(count);
			for k in 0..count {
				let (from, to) = (cell.boundary[k], cell.boundary[(k + 1) % count]);
				boundary.push(from);
				edges.push(cell.edges[k]);
				let (Some(junction), Some(other)) =
					(self.vertex_junction[from], self.vertex_junction[to])
				else {
					continue;
				};
				if junction != other {
					continue;
				}
				let (start, end) = (self.vertices[from], self.vertices[to]);
				let span = end - start;
				let mut inside: Vec<(f64, usize)> = self.junction_vertices[junction]
					.iter()
					.filter(|&&vertex| vertex != from && vertex != to)
					.filter_map(|&vertex| {
						let point = self.vertices[vertex];
						let along = (point - start).dot(span) / span.length_squared();
						(along > 0.0
							&& along < 1.0 && point.distance(start + span * along) <= VERTEX_TOLERANCE)
							.then_some((along, vertex))
					})
					.collect();
				inside.sort_by(|a, b| a.0.total_cmp(&b.0));
				for (_, vertex) in inside {
					boundary.push(vertex);
					edges.push(cell.edges[k]);
				}
			}
			cell.boundary = boundary;
			cell.edges = edges;
		}
	}

	/// Rejects cells whose edges touch anywhere other than at shared vertices.
	fn check_overlaps(&self) -> Result<(), MeshError> {
		let mut seen = HashSet::new();
		let mut segments: Vec<(usize, usize, usize)> = Vec::new();
		for (index, cell) in self.cells.iter().enumerate() {
			for (from, to) in cell.edge_vertices() {
				let key = (from.min(to), from.max(to));
				if seen.insert(key) {
					segments.push((key.0, key.1, index));
				}
			}
		}
		let min_x = |&(a, b, _): &(usize, usize, usize)| self.vertices[a].x.min(self.vertices[b].x);
		segments.sort_by(|a, b| min_x(a).total_cmp(&min_x(b)));
		for (position, first) in segments.iter().enumerate() {
			let (a, b) = (self.vertices[first.0], self.vertices[first.1]);
			let max_x = a.x.max(b.x) + VERTEX_TOLERANCE;
			for second in &segments[position + 1..] {
				if min_x(second) > max_x {
					break;
				}
				if first.0 == second.0
					|| first.0 == second.1
					|| first.1 == second.0
					|| first.1 == second.1
				{
					continue;
				}
				let (c, d) = (self.vertices[second.0], self.vertices[second.1]);
				if segment_distance(a, b, c, d) <= VERTEX_TOLERANCE {
					return Err(MeshError::FootprintOverlap {
						first: self.cells[first.2].source(self.snapshot),
						second: self.cells[second.2].source(self.snapshot),
					});
				}
			}
		}
		Ok(())
	}
}

pub(crate) fn polygon_area(vertices: &[DVec2], boundary: &[usize]) -> f64 {
	let count = boundary.len();
	(0..count)
		.map(|k| vertices[boundary[k]].perp_dot(vertices[boundary[(k + 1) % count]]))
		.sum::<f64>()
		* 0.5
}

fn segment_distance(a: DVec2, b: DVec2, c: DVec2, d: DVec2) -> f64 {
	let side = |p: DVec2, q: DVec2, r: DVec2| (q - p).perp_dot(r - p);
	let crosses = side(a, b, c) * side(a, b, d) < 0.0 && side(c, d, a) * side(c, d, b) < 0.0;
	if crosses {
		return 0.0;
	}
	point_segment_distance(a, c, d)
		.min(point_segment_distance(b, c, d))
		.min(point_segment_distance(c, a, b))
		.min(point_segment_distance(d, a, b))
}

fn point_segment_distance(point: DVec2, start: DVec2, end: DVec2) -> f64 {
	let span = end - start;
	let along = ((point - start).dot(span) / span.length_squared()).clamp(0.0, 1.0);
	point.distance(start + span * along)
}

#[cfg(test)]
#[path = "tests/footprint_internal.rs"]
mod tests;
