use bevy::prelude::*;

/// Maps a graph-space vector (x, y, z-up) to Bevy's y-up space as (x, z, -y).
///
/// This is a proper rotation, so it keeps triangle winding and works for both
/// positions and normals.
pub fn graph_to_bevy(graph_vector: Vec3) -> Vec3 {
	Vec3::new(graph_vector.x, graph_vector.z, -graph_vector.y)
}

/// The inverse of [`graph_to_bevy`].
pub fn bevy_to_graph(bevy_vector: Vec3) -> Vec3 {
	Vec3::new(bevy_vector.x, -bevy_vector.z, bevy_vector.y)
}

#[cfg(test)]
#[path = "../tests/viewport/coordinates.rs"]
mod tests;
