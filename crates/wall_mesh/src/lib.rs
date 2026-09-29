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

use wall_graph::WallGraph;

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

#[cfg(test)]
mod tests;
