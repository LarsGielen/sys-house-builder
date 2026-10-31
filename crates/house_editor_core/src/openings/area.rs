use glam::{Vec2, Vec3};
use wall_graph::{OpeningSpec, Wall, WallGraph};

/// An opening as a flat rectangle through the middle of its host wall.
///
/// It exists for the editor only: the mesh has no triangles across a hole, so picking and
/// highlighting use this instead of adding invisible geometry to the house. A curved host
/// is approximated by the chord between the opening's two edges.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OpeningArea {
	/// Graph metres, on the wall's centerline.
	pub start: Vec2,
	pub end: Vec2,
	/// Unit direction of the wall at the opening's centre.
	pub tangent: Vec2,
	/// Elevations above the floor.
	pub bottom: f32,
	pub top: f32,
}

impl OpeningArea {
	/// Counterclockwise as seen from the wall's left side, in graph metres.
	pub fn corners(&self) -> [Vec3; 4] {
		[
			self.start.extend(self.bottom),
			self.end.extend(self.bottom),
			self.end.extend(self.top),
			self.start.extend(self.top),
		]
	}

	pub fn center(&self) -> Vec3 {
		((self.start + self.end) / 2.0).extend((self.bottom + self.top) / 2.0)
	}
}

/// The area of an opening with `spec` on `wall`, or `None` for a stale wall.
///
/// Edges beyond the wall's ends are clamped onto it, so a proposal that is invalid for
/// sticking out of the wall can still be shown.
pub fn opening_area(graph: &WallGraph, wall: Wall, spec: &OpeningSpec) -> Option<OpeningArea> {
	let length = graph.wall_length(wall)?;
	let half_width = f64::from(spec.width) / 2.0;
	let parameter_at = |distance: f64| (distance / length).clamp(0.0, 1.0) as f32;
	Some(OpeningArea {
		start: graph.wall_position(wall, parameter_at(spec.center_distance - half_width))?,
		end: graph.wall_position(wall, parameter_at(spec.center_distance + half_width))?,
		tangent: graph.wall_tangent(wall, parameter_at(spec.center_distance))?,
		bottom: spec.bottom,
		top: spec.bottom + spec.height,
	})
}

/// The path distance from the start of `wall` to the point on it closest to `position`.
///
/// The graph places an opening at `center_distance / length` along the path, which is also
/// how an arc's parameter becomes a distance.
pub fn closest_distance_along_wall(graph: &WallGraph, wall: Wall, position: Vec2) -> Option<f64> {
	let (parameter, _) = graph.wall_closest_point(wall, position)?;
	Some(f64::from(parameter) * graph.wall_length(wall)?)
}

#[cfg(test)]
#[path = "../tests/openings/area.rs"]
mod tests;
