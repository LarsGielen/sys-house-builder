use glam::Vec3;
use house_document::{OpeningCategory, OpeningProposal};
use wall_graph::{OpeningSpec, WallGraph};
use wall_mesh::WallMesh;

use crate::{openings::area::closest_distance_along_wall, pointing::picking::pick_wall_hit};

const DEFAULT_WIDTH_METRES: f32 = 0.9;
const DEFAULT_HEIGHT_METRES: f32 = 2.1;
const DEFAULT_BOTTOM_METRES: f32 = 0.0;

/// Size, elevation and category applied to every opening the placement tool proposes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OpeningDefaults {
	category: OpeningCategory,
	width: f32,
	height: f32,
	bottom: f32,
}

impl Default for OpeningDefaults {
	fn default() -> Self {
		Self {
			category: OpeningCategory::default(),
			width: DEFAULT_WIDTH_METRES,
			height: DEFAULT_HEIGHT_METRES,
			bottom: DEFAULT_BOTTOM_METRES,
		}
	}
}

impl OpeningDefaults {
	pub fn category(&self) -> OpeningCategory {
		self.category
	}

	pub fn set_category(&mut self, category: OpeningCategory) {
		self.category = category;
	}

	fn spec_at(&self, center_distance: f64) -> OpeningSpec {
		OpeningSpec {
			center_distance,
			width: self.width,
			bottom: self.bottom,
			height: self.height,
		}
	}
}

/// Hosts an opening on the wall a graph-space ray meets, centred where the ray meets it.
pub fn propose_opening_at_ray(
	graph: &WallGraph,
	mesh: &WallMesh,
	defaults: &OpeningDefaults,
	origin: Vec3,
	direction: Vec3,
) -> Option<OpeningProposal> {
	let hit = pick_wall_hit(graph, mesh, origin, direction)?;
	let center_distance = closest_distance_along_wall(graph, hit.wall, hit.position.truncate())?;
	Some(OpeningProposal {
		wall: hit.wall,
		spec: defaults.spec_at(center_distance),
		category: defaults.category(),
	})
}

#[cfg(test)]
#[path = "../tests/openings/placement.rs"]
mod tests;
