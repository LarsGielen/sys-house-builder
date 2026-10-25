use super::*;

#[test]
fn downward_ray_hits_the_ground_in_graph_coordinates() {
	let ray = Ray3d::new(Vec3::new(1.0, 5.0, 2.0), Dir3::NEG_Y);
	// Bevy z = 2 is graph y = -2.
	assert_eq!(ground_hit(ray), Some(Vec2::new(1.0, -2.0)));
}

#[test]
fn ray_pointing_away_from_the_ground_has_no_hit() {
	let ray = Ray3d::new(Vec3::new(0.0, 5.0, 0.0), Dir3::Y);
	assert_eq!(ground_hit(ray), None);
}

#[test]
fn ray_parallel_to_the_ground_has_no_hit() {
	let ray = Ray3d::new(Vec3::new(0.0, 5.0, 0.0), Dir3::X);
	assert_eq!(ground_hit(ray), None);
}
