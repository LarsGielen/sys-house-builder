use std::f32::consts::{FRAC_PI_2, PI};

use glam::Vec2;

use super::super::{
	DISTANCE_TOLERANCE, angular_distance, direction_angle_between, project_point_onto_line,
	segment_intersection_parameters, segment_strictly_contains_point,
	segments_have_collinear_overlap,
};

fn assert_close(actual: f32, expected: f32) {
	assert!(
		(actual - expected).abs() < 1e-5,
		"expected {expected}, got {actual}"
	);
}

fn v(x: f32, y: f32) -> Vec2 {
	Vec2::new(x, y)
}

#[test]
fn direction_angles_grow_counter_clockwise_from_east() {
	let origin = Vec2::ZERO;
	assert_close(direction_angle_between(origin, v(1.0, 0.0)), 0.0);
	assert_close(direction_angle_between(origin, v(0.0, 1.0)), FRAC_PI_2);
	assert_close(direction_angle_between(origin, v(-1.0, 0.0)), PI);
	assert_close(direction_angle_between(origin, v(0.0, -1.0)), -FRAC_PI_2);
}

#[test]
fn direction_angles_are_measured_from_the_start_point() {
	assert_close(direction_angle_between(v(1.0, 1.0), v(2.0, 1.0)), 0.0);
	assert_close(direction_angle_between(v(1.0, 1.0), v(1.0, 2.0)), FRAC_PI_2);
}

#[test]
fn angular_distance_is_the_shortest_gap_between_angles() {
	assert_close(angular_distance(0.5, 0.0), 0.5);
	assert_close(angular_distance(0.0, 0.5), 0.5);
	assert_close(angular_distance(PI, -PI), 0.0);
	assert_close(angular_distance(PI - 0.1, -PI + 0.1), 0.2);
}

#[test]
fn projection_measures_line_parameter_and_perpendicular_distance() {
	let projection = project_point_onto_line(v(1.0, 3.0), v(0.0, 0.0), v(4.0, 0.0));
	assert_close(projection.line_parameter, 0.25);
	assert_close(projection.perpendicular_distance, 3.0);

	let beyond = project_point_onto_line(v(-2.0, -1.0), v(0.0, 0.0), v(4.0, 0.0));
	assert_close(beyond.line_parameter, -0.5);
	assert_close(beyond.perpendicular_distance, 1.0);
}

#[test]
fn a_segment_strictly_contains_points_on_it_away_from_the_ends() {
	let (start, end) = (v(0.0, 0.0), v(2.0, 2.0));

	assert!(segment_strictly_contains_point(v(1.0, 1.0), start, end));
	assert!(segment_strictly_contains_point(v(0.01, 0.01), start, end));
	assert!(segment_strictly_contains_point(
		v(1.0, 1.0 + DISTANCE_TOLERANCE / 2.0),
		start,
		end
	));
}

#[test]
fn a_segment_does_not_strictly_contain_points_off_it_or_at_its_ends() {
	let (start, end) = (v(0.0, 0.0), v(2.0, 0.0));

	assert!(!segment_strictly_contains_point(v(1.0, 0.1), start, end));
	assert!(!segment_strictly_contains_point(v(3.0, 0.0), start, end));
	assert!(!segment_strictly_contains_point(v(-1.0, 0.0), start, end));
	assert!(!segment_strictly_contains_point(start, start, end));
	assert!(!segment_strictly_contains_point(end, start, end));
	assert!(!segment_strictly_contains_point(
		v(DISTANCE_TOLERANCE / 2.0, 0.0),
		start,
		end
	));
}

#[test]
fn collinear_overlap_requires_a_shared_stretch() {
	let base = (v(0.0, 0.0), v(2.0, 0.0));

	for overlapping in [
		(v(1.0, 0.0), v(3.0, 0.0)),
		(v(0.5, 0.0), v(1.5, 0.0)),
		(v(-1.0, 0.0), v(3.0, 0.0)),
		(v(2.0, 0.0), v(0.0, 0.0)),
	] {
		assert!(segments_have_collinear_overlap(base, overlapping));
	}
	assert!(segments_have_collinear_overlap(
		(v(0.5, 0.0), v(0.6, 0.0)),
		base
	));

	for separate in [
		(v(2.0, 0.0), v(3.0, 0.0)),
		(v(3.0, 0.0), v(4.0, 0.0)),
		(v(0.0, 1.0), v(2.0, 1.0)),
		(v(1.0, 0.0), v(1.0, 1.0)),
	] {
		assert!(!segments_have_collinear_overlap(base, separate));
	}
}

#[test]
fn intersection_parameters_report_where_along_each_segment_they_meet() {
	let (along_a, along_b) =
		segment_intersection_parameters((v(0.0, 0.0), v(4.0, 0.0)), (v(1.0, -1.0), v(1.0, 3.0)))
			.unwrap();
	assert_close(along_a, 0.25);
	assert_close(along_b, 0.25);
}

#[test]
fn segments_meeting_at_an_end_still_intersect() {
	let (along_a, along_b) =
		segment_intersection_parameters((v(0.0, 0.0), v(2.0, 0.0)), (v(2.0, 0.0), v(2.0, 2.0)))
			.unwrap();
	assert_close(along_a, 1.0);
	assert_close(along_b, 0.0);
}

#[test]
fn parallel_or_missing_segments_do_not_intersect() {
	let base = (v(0.0, 0.0), v(2.0, 0.0));

	for separate in [
		(v(0.0, 1.0), v(2.0, 1.0)),
		(v(1.0, 0.0), v(3.0, 0.0)),
		(v(3.0, -1.0), v(3.0, 1.0)),
		(v(1.0, 1.0), v(1.0, 2.0)),
	] {
		assert_eq!(segment_intersection_parameters(base, separate), None);
	}
}
