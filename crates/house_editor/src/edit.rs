use std::fmt::Display;

use bevy::prelude::{Resource, Vec2};
use wall_graph::{WallDimensions, WallError, WallGraph};
use wall_mesh::{MeshError, MeshSettings, WallMesh};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditError {
	Graph(WallError),
	Mesh(MeshError),
}

impl Display for EditError {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			EditError::Graph(error) => error.fmt(formatter),
			EditError::Mesh(error) => error.fmt(formatter),
		}
	}
}

impl std::error::Error for EditError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Self::Graph(error) => Some(error),
			Self::Mesh(error) => Some(error),
		}
	}
}

impl From<WallError> for EditError {
	fn from(value: WallError) -> Self {
		EditError::Graph(value)
	}
}

impl From<MeshError> for EditError {
	fn from(value: MeshError) -> Self {
		EditError::Mesh(value)
	}
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallProposal {
	pub start: Vec2,
	pub end: Vec2,
	pub dimensions: WallDimensions,
}

#[derive(Default)]
pub struct EditCandidate {
	graph: WallGraph,
	mesh: WallMesh,
}

impl EditCandidate {
	pub fn graph(&self) -> &WallGraph {
		&self.graph
	}

	pub fn mesh(&self) -> &WallMesh {
		&self.mesh
	}
}

pub fn propose_wall(
	committed: &WallGraph,
	wall: &WallProposal,
) -> Result<EditCandidate, EditError> {
	let mut candidate_graph = committed.clone();
	candidate_graph.add_wall_with_dimensions(wall.start, wall.end, wall.dimensions)?;
	let mesh = wall_mesh::generate(&candidate_graph, &MeshSettings::default())?;

	Ok(EditCandidate {
		graph: candidate_graph,
		mesh,
	})
}

pub struct PreviewState {
	pub house_revision: u64,
	pub proposal: WallProposal,
	pub outcome: Result<EditCandidate, EditError>,
}

/// The edit being proposed and what it would produce, shared between the tool that
/// proposes it and the views that display it. Committing takes exactly this candidate,
/// so the preview and the committed house cannot disagree.
#[derive(Resource, Default)]
pub struct EditPreview {
	pub state: Option<PreviewState>,
}

impl EditPreview {
	pub fn valid_candidate(&self) -> Option<&EditCandidate> {
		self.state.as_ref()?.outcome.as_ref().ok()
	}
}

#[cfg(test)]
#[path = "tests/edit.rs"]
mod tests;
