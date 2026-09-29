use std::fmt;

use wall_graph::{OpeningId, Wall, WallNodeId};

use crate::SurfaceSource;

/// Why a wall graph could not be meshed. No partial mesh is produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshError {
	/// A setting is not finite or is out of range.
	InvalidSettings,
	/// A curved wall's inner face would have no positive radius.
	ArcTooTight { wall: Wall },
	/// A junction's walls do not form a valid outline.
	UnresolvedJunction { node: WallNodeId },
	/// The junctions at both ends of a wall consume its whole length.
	JunctionTrimsOverlap { wall: Wall },
	/// An opening leaves less than the minimum pier beside a resolved junction.
	OpeningTooCloseToJunction { opening: OpeningId },
	/// Two parts of the footprint overlap without meeting at a junction.
	FootprintOverlap {
		first: SurfaceSource,
		second: SurfaceSource,
	},
	/// A footprint region could not be divided into triangles.
	TriangulationFailed { source: SurfaceSource },
	/// The mesh would exceed a segment, vertex, or triangle limit.
	OutputLimitExceeded,
}

impl fmt::Display for MeshError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let message = match self {
			MeshError::InvalidSettings => "mesh settings must be finite and within range",
			MeshError::ArcTooTight { .. } => "a curved wall is too thick for its radius",
			MeshError::UnresolvedJunction { .. } => "a wall junction has no valid outline",
			MeshError::JunctionTrimsOverlap { .. } => {
				"a wall is too short for the junctions at its ends"
			}
			MeshError::OpeningTooCloseToJunction { .. } => {
				"an opening is too close to a resolved wall junction"
			}
			MeshError::FootprintOverlap { .. } => "walls overlap away from a junction",
			MeshError::TriangulationFailed { .. } => "a wall outline could not be triangulated",
			MeshError::OutputLimitExceeded => "the wall mesh would exceed its size limits",
		};
		f.write_str(message)
	}
}

impl std::error::Error for MeshError {}
