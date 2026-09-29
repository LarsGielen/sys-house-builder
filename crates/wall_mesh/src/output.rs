use std::ops::Range;

use glam::Vec3;
use wall_graph::{OpeningId, Wall, WallNodeId};

/// A side of a wall relative to its handle's direction of travel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
	Left,
	Right,
}

impl Side {
	pub(crate) fn opposite(self) -> Self {
		match self {
			Side::Left => Side::Right,
			Side::Right => Side::Left,
		}
	}
}

/// The role of a surface in the wall solid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SurfaceKind {
	/// A vertical face along one side of a wall.
	Side(Side),
	/// An upward face at the top of a wall or junction.
	Top,
	/// A downward face on the floor.
	Bottom,
	/// The flat end of a wall that meets no other wall.
	EndCap,
	/// A vertical reveal at the side of an opening.
	Jamb,
	/// The downward reveal above an opening.
	Head,
	/// The upward reveal below an opening.
	Sill,
	/// A vertical face exposed where walls of different size meet.
	JunctionStep,
	/// The face cutting off a sharp outside corner.
	Bevel,
}

/// What a surface belongs to. Wall handles are valid only for the meshed graph state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SurfaceSource {
	Wall(Wall),
	Opening(OpeningId),
	Junction(WallNodeId),
}

/// The role and owner of a group of triangles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SurfaceTag {
	pub kind: SurfaceKind,
	pub source: SurfaceSource,
}

/// A contiguous range of [`WallMesh::indices`] sharing one tag.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshSection {
	pub tag: SurfaceTag,
	pub indices: Range<usize>,
}

/// An indexed triangle list in graph coordinates: X and Y from the floor plan, Z up, in metres.
///
/// Triangles wind counterclockwise when seen from outside the solid. Faces meeting at a hard
/// edge have separate vertices so each keeps its own normal.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WallMesh {
	pub(crate) positions: Vec<Vec3>,
	pub(crate) normals: Vec<Vec3>,
	pub(crate) indices: Vec<u32>,
	pub(crate) sections: Vec<MeshSection>,
}

impl WallMesh {
	pub fn positions(&self) -> &[Vec3] {
		&self.positions
	}

	pub fn normals(&self) -> &[Vec3] {
		&self.normals
	}

	/// Three vertex indices per triangle.
	pub fn indices(&self) -> &[u32] {
		&self.indices
	}

	/// Tagged index ranges that together cover every triangle, in order.
	pub fn sections(&self) -> &[MeshSection] {
		&self.sections
	}

	pub fn is_empty(&self) -> bool {
		self.indices.is_empty()
	}
}
