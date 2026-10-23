use std::{fmt::Display, ops::RangeInclusive};

use bevy::prelude::{Resource, Vec2};
use wall_graph::{OpeningSpec, Wall, WallDimensions, WallError, WallGraph};
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OpeningProposal {
	pub wall: Wall,
	pub spec: OpeningSpec,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Proposal {
	Wall(WallProposal),
	Opening(OpeningProposal),
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
	let mesh = wall_mesh::generate(&candidate_graph, &mesh_settings())?;

	Ok(EditCandidate {
		graph: candidate_graph,
		mesh,
	})
}

/// Adding an opening can pass the graph's checks and still fail meshing near a junction,
/// so the mesh is generated before the candidate is accepted.
pub fn propose_opening(
	committed: &WallGraph,
	opening: &OpeningProposal,
) -> Result<EditCandidate, EditError> {
	let mut candidate_graph = committed.clone();
	candidate_graph.add_opening(opening.wall, opening.spec)?;
	let mesh = wall_mesh::generate(&candidate_graph, &mesh_settings())?;

	Ok(EditCandidate {
		graph: candidate_graph,
		mesh,
	})
}

fn mesh_settings() -> MeshSettings {
	MeshSettings::default()
}

/// Path distances between which an opening on `wall` passes both the graph's and the mesh's
/// junction clearance, or `None` when the wall has no such stretch.
pub fn opening_bounds(graph: &WallGraph, wall: Wall) -> Option<RangeInclusive<f64>> {
	let graph_bounds = graph.opening_bounds(wall)?;
	let mesh_bounds = wall_mesh::opening_bounds(graph, wall, &mesh_settings()).ok()?;
	Some(
		graph_bounds.start().max(*mesh_bounds.start())..=graph_bounds.end().min(*mesh_bounds.end()),
	)
}

pub struct PreviewState {
	pub house_revision: u64,
	pub proposal: Proposal,
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
	/// Tools use these to clear only their own proposal, so that leaving one tool does not
	/// erase the preview of the tool that was just entered.
	pub fn holds_wall_proposal(&self) -> bool {
		matches!(
			self.state,
			Some(PreviewState {
				proposal: Proposal::Wall(_),
				..
			})
		)
	}

	pub fn holds_opening_proposal(&self) -> bool {
		matches!(
			self.state,
			Some(PreviewState {
				proposal: Proposal::Opening(_),
				..
			})
		)
	}

	pub fn valid_candidate(&self) -> Option<&EditCandidate> {
		self.state.as_ref()?.outcome.as_ref().ok()
	}
}

#[cfg(test)]
#[path = "tests/edit.rs"]
mod tests;
