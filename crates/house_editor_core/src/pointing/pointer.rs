use glam::Vec2;

/// A pointer position and the floor-plan point beneath it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerOnGround {
	/// Logical pixels from the window's top-left corner.
	pub viewport_position: Vec2,
	/// Graph metres.
	pub graph_position: Vec2,
}
