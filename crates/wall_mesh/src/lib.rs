#![doc = include_str!("../README.md")]

mod error;
mod extrude;
mod footprint;
mod intervals;
mod output;
mod path;
mod settings;
mod snapshot;
mod triangulate;

pub use error::MeshError;
pub use output::{MeshSection, Side, SurfaceKind, SurfaceSource, SurfaceTag, WallMesh};
pub use settings::MeshSettings;

use std::ops::RangeInclusive;

use wall_graph::{Wall, WallGraph};

/// Builds a closed triangle mesh of every wall in `graph`.
///
/// The mesh is a snapshot: wall handles in its surface tags refer to this graph state only.
/// The graph is never modified, and an error returns no partial geometry.
pub fn generate(graph: &WallGraph, settings: &MeshSettings) -> Result<WallMesh, MeshError> {
	settings.validate()?;
	let snapshot = snapshot::Snapshot::new(graph);
	let footprint = footprint::build(&snapshot, settings)?;
	extrude::extrude(&snapshot, &footprint, settings)
}

/// Returns the path distances between which an opening on `wall` avoids
/// [`MeshError::OpeningTooCloseToJunction`] in [`generate`] with the same settings.
///
/// A wall end that meets a junction is trimmed by its miter, and `min_pier_width` more is
/// kept clear; a free end bounds nothing. An opening whose start and end lie in the range
/// passes this check. The range is empty when the junctions leave no room. Clearance
/// from other openings, and the graph's own junction rules, are not considered.
pub fn opening_bounds(
	graph: &WallGraph,
	wall: Wall,
	settings: &MeshSettings,
) -> Result<RangeInclusive<f64>, MeshError> {
	settings.validate()?;
	let snapshot = snapshot::Snapshot::new(graph);
	let index = snapshot
		.walls
		.iter()
		.position(|candidate| candidate.handle == wall)
		.ok_or(MeshError::UnknownWall { wall })?;
	let (start, end) = footprint::opening_bounds(&snapshot, index, settings)?;
	Ok(start..=end)
}

#[cfg(test)]
mod tests;
