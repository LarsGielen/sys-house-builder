use std::f32::consts::PI;

use super::super::angular_distance;

#[test]
fn angular_distance_is_the_shortest_gap_between_angles() {
	for (first, second, expected) in [
		(0.5, 0.0, 0.5),
		(0.0, 0.5, 0.5),
		(PI, -PI, 0.0),
		(PI - 0.1, -PI + 0.1, 0.2),
	] {
		assert!((angular_distance(first, second) - expected).abs() < 1e-5);
	}
}
