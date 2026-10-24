use bevy::prelude::Resource;
use wall_graph::WallGraph;
use wall_mesh::WallMesh;

use crate::edit::EditCandidate;

/// The committed graph and its generated mesh are replaced as one validated candidate.
#[derive(Resource, Default)]
pub struct House {
	committed: EditCandidate,
	revision: u64,
}

impl House {
	pub fn committed(&self) -> &EditCandidate {
		&self.committed
	}

	pub fn graph(&self) -> &WallGraph {
		self.committed.graph()
	}

	pub fn mesh(&self) -> &WallMesh {
		self.committed.mesh()
	}

	pub fn revision(&self) -> u64 {
		self.revision
	}

	pub fn commit(&mut self, candidate: EditCandidate) {
		self.committed = candidate;
		self.revision += 1;
	}
}
