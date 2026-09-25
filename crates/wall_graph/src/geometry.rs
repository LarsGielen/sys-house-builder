use std::f32::consts::TAU;

use glam::Vec2;

/// Walls leaving a node closer together than this many radians count as overlapping.
pub(crate) const OVERLAP_ANGLE_TOLERANCE: f32 = 1e-6;

pub(crate) fn direction_angle(from: Vec2, to: Vec2) -> f32 {
	(to - from).to_angle()
}

/// Smallest angle between two directions, accounting for the wrap at ±π.
pub(crate) fn angle_difference(a: f32, b: f32) -> f32 {
	let difference = (a - b).abs();
	difference.min(TAU - difference)
}

#[cfg(test)]
mod tests {
	use std::f32::consts::{FRAC_PI_2, PI};

	use super::*;

	fn assert_close(actual: f32, expected: f32) {
		assert!(
			(actual - expected).abs() < 1e-5,
			"expected {expected}, got {actual}"
		);
	}

	#[test]
	fn direction_angle_grows_counter_clockwise_from_east() {
		let origin = Vec2::ZERO;
		assert_close(direction_angle(origin, Vec2::new(1.0, 0.0)), 0.0);
		assert_close(direction_angle(origin, Vec2::new(0.0, 1.0)), FRAC_PI_2);
		assert_close(direction_angle(origin, Vec2::new(-1.0, 0.0)), PI);
		assert_close(direction_angle(origin, Vec2::new(0.0, -1.0)), -FRAC_PI_2);
	}

	#[test]
	fn direction_angle_is_measured_from_the_start_point() {
		assert_close(
			direction_angle(Vec2::new(1.0, 1.0), Vec2::new(2.0, 1.0)),
			0.0,
		);
		assert_close(
			direction_angle(Vec2::new(1.0, 1.0), Vec2::new(1.0, 2.0)),
			FRAC_PI_2,
		);
	}

	#[test]
	fn angle_difference_is_the_gap_between_close_angles() {
		assert_close(angle_difference(0.5, 0.0), 0.5);
		assert_close(angle_difference(0.0, 0.5), 0.5);
	}

	#[test]
	fn angle_difference_wraps_around_pi() {
		assert_close(angle_difference(PI, -PI), 0.0);
		assert_close(angle_difference(PI - 0.1, -PI + 0.1), 0.2);
	}
}
