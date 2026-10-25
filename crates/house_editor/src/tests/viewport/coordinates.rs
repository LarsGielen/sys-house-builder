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
