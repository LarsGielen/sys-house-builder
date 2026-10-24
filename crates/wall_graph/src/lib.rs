#![doc = include_str!("../README.md")]

mod arc;
mod geometry;
mod graph;
mod opening;

pub use arc::{
	ArcError, arc_midpoint, arc_points, arc_radius, arc_sweep_for_radius, arc_tangent_at_both,
	arc_tangent_at_start, arc_through_bend,
};
pub use graph::{
	ReshapeReport, SmoothJoin, Wall, WallCurve, WallDimensions, WallError, WallGraph,
	WallGraphIdMap, WallNodeId,
};
pub use opening::{MIN_OPENING_GAP, Opening, OpeningId, OpeningSpec};
