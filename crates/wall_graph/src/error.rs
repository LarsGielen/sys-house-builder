use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WallError {
	/// An endpoint is not a node of this graph.
	UnknownNode,
	/// The wall would start and end at the same node.
	SameNode,
	/// The two nodes are already joined by a wall.
	Duplicate,
	/// The wall runs along a wall that already leaves one of its nodes.
	Overlapping,
}

impl fmt::Display for WallError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let message = match self {
			WallError::UnknownNode => "a wall endpoint is not a node of this graph",
			WallError::SameNode => "a wall cannot start and end at the same node",
			WallError::Duplicate => "there is already a wall between these nodes",
			WallError::Overlapping => "the wall overlaps an existing wall leaving the same node",
		};
		f.write_str(message)
	}
}

impl std::error::Error for WallError {}
