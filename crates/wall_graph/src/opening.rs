use crate::Wall;

/// Identifies an opening in one wall graph. IDs survive ordinary edits and optimization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpeningId(pub(crate) usize);

/// An opening's dimensions and center distance along its wall, in metres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OpeningSpec {
	pub center_distance: f64,
	pub width: f32,
	pub bottom: f32,
	pub height: f32,
}

/// A current opening and its owning wall piece.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Opening {
	pub wall: Wall,
	pub spec: OpeningSpec,
}

pub(crate) const MIN_OPENING_GAP: f64 = 0.02;
pub(crate) const CORNER_CLEARANCE: f64 = 0.05;
