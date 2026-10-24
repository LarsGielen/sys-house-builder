use std::{collections::HashMap, ops::RangeInclusive, sync::Mutex};

use bevy::prelude::Resource;
use wall_graph::{Wall, WallGraph};
use wall_mesh::WallMesh;

use crate::edit::{self, EditCandidate};

/// The committed graph and its generated mesh are replaced as one validated candidate.
#[derive(Resource, Default)]
pub struct House {
	committed: EditCandidate,
	revision: u64,
	/// Computing a wall's bounds meshes the junctions of the whole house, and opening placement
	/// asks for them every frame, so they are kept until the next commit.
	opening_bounds: Mutex<HashMap<Wall, Option<RangeInclusive<f64>>>>,
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

	/// See [`edit::opening_bounds`]. Cached, because the committed house cannot change between
	/// commits.
	pub fn opening_bounds(&self, wall: Wall) -> Option<RangeInclusive<f64>> {
		self.opening_bounds
			.lock()
			.expect("the cache holds no invariants a panic could break")
			.entry(wall)
			.or_insert_with(|| edit::opening_bounds(self.graph(), wall))
			.clone()
	}

	pub fn commit(&mut self, candidate: EditCandidate) {
		self.committed = candidate;
		self.revision += 1;
		self.opening_bounds
			.get_mut()
			.expect("the cache holds no invariants a panic could break")
			.clear();
	}
}
