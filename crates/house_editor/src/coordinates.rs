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
mod tests {
	use super::*;

	#[test]
	fn bevy_to_graph_undoes_graph_to_bevy() {
		let graph_vector = Vec3::new(1.0, 2.0, 3.0);
		assert_eq!(bevy_to_graph(graph_to_bevy(graph_vector)), graph_vector);
	}

	#[test]
	fn graph_up_becomes_bevy_up_and_graph_y_becomes_negative_z() {
		assert_eq!(graph_to_bevy(Vec3::Z), Vec3::Y);
		assert_eq!(graph_to_bevy(Vec3::Y), Vec3::NEG_Z);
	}
}
