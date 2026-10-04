use std::fmt::Display;

use bevy::math::Vec2;
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

pub struct WallProposal {
	pub start: Vec2,
	pub end: Vec2,
	pub dimensions: WallDimensions,
}

pub struct Candidate {
	pub graph: WallGraph,
	pub mesh: WallMesh,
}

pub fn propose_wall(committed: &WallGraph, wall: &WallProposal) -> Result<Candidate, EditError> {
	let mut committed_clone = committed.clone();
	committed_clone.add_wall_with_dimensions(wall.start, wall.end, wall.dimensions)?;
	let mesh = wall_mesh::generate(&committed_clone, &MeshSettings::default())?;

	Ok(Candidate {
		graph: committed_clone,
		mesh: mesh,
	})
}
