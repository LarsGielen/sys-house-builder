use glam::{Vec2, Vec3};
use house_document::Proposal;
use wall_graph::{OpeningId, Wall, WallGraph, WallNodeId};
use wall_mesh::{SurfaceSource, WallMesh};

use crate::openings::area::opening_area;

// A jamb lies just inside the opening's area, so the area wins ties this close.
const OCCLUSION_TOLERANCE_METRES: f32 = 1e-3;

/// What a pointer ray picked. A selection can refer to exactly one wall or opening.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SelectionTarget {
	Wall(Wall),
	Opening(OpeningId),
}

impl SelectionTarget {
	/// The edit that deletes this target.
	pub fn removal(self) -> Proposal {
		match self {
			Self::Wall(wall) => Proposal::WallRemoval(wall),
			Self::Opening(opening) => Proposal::OpeningRemoval(opening),
		}
	}

	pub fn wall(self) -> Option<Wall> {
		match self {
			Self::Wall(wall) => Some(wall),
			Self::Opening(_) => None,
		}
	}

	pub fn opening(self) -> Option<OpeningId> {
		match self {
			Self::Opening(opening) => Some(opening),
			Self::Wall(_) => None,
		}
	}
}

/// Picks what a graph-space ray points at. An opening's area takes precedence over its host
/// wall, unless something nearer hides it, and its jambs, head, and sill select the opening.
pub fn pick_target(
	graph: &WallGraph,
	mesh: &WallMesh,
	origin: Vec3,
	direction: Vec3,
) -> Option<SelectionTarget> {
	let surface = nearest_surface(mesh, origin, direction);
	if let Some((area_distance, opening)) = nearest_opening_area(graph, origin, direction)
		&& surface.is_none_or(|(surface_distance, _)| {
			area_distance <= surface_distance + OCCLUSION_TOLERANCE_METRES
		}) {
		return Some(SelectionTarget::Opening(opening));
	}

	let (distance, source) = surface?;
	match source {
		SurfaceSource::Opening(opening) => Some(SelectionTarget::Opening(opening)),
		_ => wall_of_surface(graph, source, (origin + direction * distance).truncate())
			.map(SelectionTarget::Wall),
	}
}

/// The nearest opening whose area the ray crosses, with the distance to it.
fn nearest_opening_area(
	graph: &WallGraph,
	origin: Vec3,
	direction: Vec3,
) -> Option<(f32, OpeningId)> {
	graph
		.openings()
		.filter_map(|(id, opening)| {
			let [a, b, c, d] = opening_area(graph, opening.wall, &opening.spec)?.corners();
			let distance = ray_triangle_distance(origin, direction, [a, b, c])
				.or_else(|| ray_triangle_distance(origin, direction, [a, c, d]))?;
			Some((distance, id))
		})
		.min_by(|(a, _), (b, _)| a.total_cmp(b))
}

/// The wall owning the nearest mesh surface along a ray, and where the ray meets it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WallHit {
	pub wall: Wall,
	/// Graph metres.
	pub position: Vec3,
}

pub fn pick_wall_hit(
	graph: &WallGraph,
	mesh: &WallMesh,
	origin: Vec3,
	direction: Vec3,
) -> Option<WallHit> {
	let (distance, source) = nearest_surface(mesh, origin, direction)?;
	let position = origin + direction * distance;
	let wall = wall_of_surface(graph, source, position.truncate())?;
	Some(WallHit { wall, position })
}

/// The nearest mesh triangle along a graph-space ray, with its distance and what it belongs to.
fn nearest_surface(mesh: &WallMesh, origin: Vec3, direction: Vec3) -> Option<(f32, SurfaceSource)> {
	let positions = mesh.positions();
	let indices = mesh.indices();
	mesh.sections()
		.iter()
		.flat_map(|section| {
			indices[section.indices.clone()]
				.as_chunks::<3>()
				.0
				.iter()
				.map(move |triangle| (section.tag.source, triangle))
		})
		.filter_map(|(source, triangle)| {
			let corners = triangle.map(|index| positions[index as usize]);
			Some((ray_triangle_distance(origin, direction, corners)?, source))
		})
		.min_by(|(a, _), (b, _)| a.total_cmp(b))
}

/// The wall a surface belongs to. A hit on a junction picks the wall nearest `point`.
fn wall_of_surface(graph: &WallGraph, source: SurfaceSource, point: Vec2) -> Option<Wall> {
	match source {
		SurfaceSource::Wall(wall) => Some(wall),
		SurfaceSource::Opening(opening) => Some(graph.opening(opening)?.wall),
		SurfaceSource::Junction(node) => nearest_wall_at_junction(graph, node, point),
	}
}

/// A junction surface belongs to several walls, so it selects the one closest to the hit.
fn nearest_wall_at_junction(graph: &WallGraph, node: WallNodeId, point: Vec2) -> Option<Wall> {
	graph
		.node_walls(node)?
		.into_iter()
		.filter_map(|wall| {
			let (_, closest) = graph.wall_closest_point(wall, point)?;
			Some((wall, closest.distance(point)))
		})
		.min_by(|(_, a), (_, b)| a.total_cmp(b))
		.map(|(wall, _)| wall)
}

/// Möller–Trumbore, hitting both faces so a ray from inside a wall still finds it.
fn ray_triangle_distance(origin: Vec3, direction: Vec3, [a, b, c]: [Vec3; 3]) -> Option<f32> {
	let edge_ab = b - a;
	let edge_ac = c - a;
	let direction_cross_edge = direction.cross(edge_ac);
	let determinant = edge_ab.dot(direction_cross_edge);
	// A ray parallel to the triangle plane never crosses it.
	if determinant.abs() < f32::EPSILON {
		return None;
	}

	let to_origin = origin - a;
	let origin_cross_edge = to_origin.cross(edge_ab);
	let u = to_origin.dot(direction_cross_edge) / determinant;
	let v = direction.dot(origin_cross_edge) / determinant;
	let distance = edge_ac.dot(origin_cross_edge) / determinant;
	(u >= 0.0 && v >= 0.0 && u + v <= 1.0 && distance >= 0.0).then_some(distance)
}

#[cfg(test)]
#[path = "../tests/pointing/picking.rs"]
mod tests;
