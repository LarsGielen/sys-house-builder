use glam::Vec2;
use wall_graph::{Wall, WallCurve, WallGraph, WallNodeId};

use crate::{interaction_settings::InteractionSettings, pointing::pointer::PointerOnGround};

const GRID_SPACINGS_METRES: [f32; 5] = [0.1, 0.25, 0.5, 1.0, 2.0];
const DEFAULT_GRID_SPACING_INDEX: usize = 3;

/// What a snap target was taken from. Priority is defined by `SnapSource::priority`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SnapSource {
	Junction(WallNodeId),
	Wall(Wall),
	Grid,
	Free,
}

impl SnapSource {
	fn priority(self) -> u8 {
		match self {
			SnapSource::Junction(_) => 0,
			SnapSource::Wall(_) => 1,
			SnapSource::Grid => 2,
			SnapSource::Free => 3,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnapTarget {
	pub position: Vec2,
	pub source: SnapSource,
}

impl SnapTarget {
	pub fn free(position: Vec2) -> Self {
		Self {
			position,
			source: SnapSource::Free,
		}
	}
}

pub struct SnapSettings {
	grid_spacing_index: usize,
	/// Gap kept between an opening and the neighbour it snaps beside, before the graph's
	/// minimum is applied.
	opening_gap: f32,
}

impl Default for SnapSettings {
	fn default() -> Self {
		Self {
			grid_spacing_index: DEFAULT_GRID_SPACING_INDEX,
			opening_gap: 0.0,
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidOpeningGap;

impl std::fmt::Display for InvalidOpeningGap {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		formatter.write_str("Opening gap must be finite, nonnegative, and representable in metres")
	}
}

impl std::error::Error for InvalidOpeningGap {}

impl SnapSettings {
	pub fn grid_spacing(&self) -> f32 {
		GRID_SPACINGS_METRES[self.grid_spacing_index]
	}

	pub fn opening_gap(&self) -> f32 {
		self.opening_gap
	}

	pub fn set_opening_gap(&mut self, gap: f32) -> Result<(), InvalidOpeningGap> {
		if !gap.is_finite() || gap < 0.0 {
			return Err(InvalidOpeningGap);
		}
		self.opening_gap = gap;
		Ok(())
	}

	pub fn step_grid_spacing(&mut self, step: isize) {
		let last_index = GRID_SPACINGS_METRES.len() - 1;
		self.grid_spacing_index = self
			.grid_spacing_index
			.saturating_add_signed(step)
			.min(last_index);
	}
}

/// Picks the best selectable candidate: junctions first, then walls, then grid points,
/// and within a class the candidate nearest the pointer on screen.
/// Wall candidates use the closest point in graph space; under perspective this is
/// an approximation of the closest point on the projected wall.
pub fn choose_snap_target(
	graph: &WallGraph,
	pointer: PointerOnGround,
	settings: &SnapSettings,
	interaction: &InteractionSettings,
	previous_source: Option<SnapSource>,
	graph_to_viewport: impl Fn(Vec2) -> Option<Vec2>,
) -> SnapTarget {
	let junctions = graph.nodes().map(|(node, position)| SnapTarget {
		position,
		source: SnapSource::Junction(node),
	});
	let walls = graph.walls().filter_map(|wall| {
		let (_, position) = graph.wall_closest_point(wall, pointer.graph_position)?;
		Some(SnapTarget {
			position,
			source: SnapSource::Wall(wall),
		})
	});
	let grid = std::iter::once(SnapTarget {
		position: snap_to_grid(pointer.graph_position, settings.grid_spacing()),
		source: SnapSource::Grid,
	});

	junctions
		.chain(walls)
		.chain(grid)
		.filter_map(|target| {
			let distance = graph_to_viewport(target.position)?.distance(pointer.viewport_position);
			let radius = if previous_source == Some(target.source) {
				interaction.snap_release_radius_pixels()
			} else {
				interaction.snap_acquire_radius_pixels()
			};
			(distance <= radius).then_some((target, distance))
		})
		.min_by(|(first, first_distance), (second, second_distance)| {
			first
				.source
				.priority()
				.cmp(&second.source.priority())
				.then(first_distance.total_cmp(second_distance))
				.then_with(|| {
					let first_held = previous_source == Some(first.source);
					let second_held = previous_source == Some(second.source);
					second_held.cmp(&first_held)
				})
				.then_with(|| stable_source_order(graph, first.source, second.source))
		})
		.map(|(target, _)| target)
		.unwrap_or(SnapTarget::free(pointer.graph_position))
}

// Graph iteration order is unspecified, so equal candidates need a stable final order.
fn stable_source_order(
	graph: &WallGraph,
	first: SnapSource,
	second: SnapSource,
) -> std::cmp::Ordering {
	match (first, second) {
		(SnapSource::Junction(first), SnapSource::Junction(second)) => first.cmp(&second),
		(SnapSource::Wall(first), SnapSource::Wall(second)) => {
			// Different arcs can share both endpoints; their sweeps distinguish their paths.
			let sweep = |wall| match graph
				.wall_curve(wall)
				.expect("snap candidates are live walls")
			{
				WallCurve::Straight { .. } => 0.0,
				WallCurve::CircularArc { sweep, .. } => sweep,
			};
			(first.origin(), first.destination())
				.cmp(&(second.origin(), second.destination()))
				.then_with(|| sweep(first).total_cmp(&sweep(second)))
		}
		_ => first.priority().cmp(&second.priority()),
	}
}

pub fn snap_to_grid(position: Vec2, spacing: f32) -> Vec2 {
	(position / spacing).round() * spacing
}

#[cfg(test)]
#[path = "../tests/pointing/snapping.rs"]
mod tests;
