/// Identifies a node of a `WallGraph`. Only meaningful for the graph that created it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WallNodeId(pub(crate) usize);

/// Identifies one direction of a wall in a `WallGraph`. Only meaningful for the graph that created it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HalfEdgeId(pub(crate) usize);
