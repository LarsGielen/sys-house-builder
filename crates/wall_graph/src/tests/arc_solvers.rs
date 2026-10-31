use std::f32::consts::{FRAC_PI_2, PI};

use glam::Vec2;

use crate::{
	ArcError, WallGraph, arc_midpoint, arc_radius, arc_sweep_for_radius, arc_tangent_at_both,
	arc_tangent_at_start, arc_through_bend,
};

fn close(actual: f32, expected: f32) {
	assert!((actual - expected).abs() < 1e-5, "{actual} != {expected}");
}

#[test]
fn a_bend_to_the_right_of_the_chord_sweeps_counterclockwise() {
	let start = Vec2::new(1.0, 0.0);
	let end = Vec2::new(-1.0, 0.0);
	close(
		arc_through_bend(start, Vec2::new(0.0, 1.0), end).unwrap(),
		PI,
	);
	close(
		arc_through_bend(start, Vec2::new(0.0, -1.0), end).unwrap(),
		-PI,
	);
}

#[test]
fn a_bend_beyond_the_circle_gives_a_major_arc() {
	// Three quarters of the unit circle, passing through the far side from the chord.
	let sweep = arc_through_bend(
		Vec2::new(1.0, 0.0),
		Vec2::new(-1.0, 0.0),
		Vec2::new(0.0, -1.0),
	)
	.unwrap();
	close(sweep, 1.5 * PI);
}

#[test]
fn the_sweep_from_a_bend_makes_the_arc_pass_through_it() {
	let (start, bend, end) = (Vec2::ZERO, Vec2::new(1.0, 0.5), Vec2::new(3.0, 0.0));
	let sweep = arc_through_bend(start, bend, end).unwrap();
	let mut graph = WallGraph::new();
	let wall = graph.add_arc(start, end, sweep).unwrap()[0];
	let (_, closest) = graph.wall_closest_point(wall, bend).unwrap();
	assert!(closest.distance(bend) < 1e-4);
}

#[test]
fn a_bend_on_the_chord_is_rejected() {
	assert_eq!(
		arc_through_bend(Vec2::ZERO, Vec2::new(1.0, 0.0), Vec2::new(2.0, 0.0)),
		Err(ArcError::BendOnChord)
	);
	assert_eq!(
		arc_through_bend(Vec2::ZERO, Vec2::Y, Vec2::ZERO),
		Err(ArcError::InvalidPosition)
	);
}

#[test]
fn a_tangent_arc_leaves_the_start_along_the_given_direction() {
	let start = Vec2::ZERO;
	let end = Vec2::new(2.0, 2.0);
	let sweep = arc_tangent_at_start(start, Vec2::X, end).unwrap();
	// A quarter turn to the left: half the 45 degree angle between tangent and chord, twice.
	close(sweep, FRAC_PI_2);
	let mut graph = WallGraph::new();
	let wall = graph.add_arc(start, end, sweep).unwrap()[0];
	assert!(graph.wall_tangent(wall, 0.0).unwrap().distance(Vec2::X) < 1e-5);
}

#[test]
fn a_tangent_arc_toward_the_right_sweeps_clockwise() {
	assert!(arc_tangent_at_start(Vec2::ZERO, Vec2::X, Vec2::new(2.0, -2.0)).unwrap() < 0.0);
}

#[test]
fn an_endpoint_ahead_on_the_tangent_is_a_straight_wall_and_one_behind_is_rejected() {
	assert_eq!(
		arc_tangent_at_start(Vec2::ZERO, Vec2::X, Vec2::new(3.0, 0.0)),
		Ok(0.0)
	);
	assert_eq!(
		arc_tangent_at_start(Vec2::ZERO, Vec2::X, Vec2::new(-3.0, 0.0)),
		Err(ArcError::EndpointBehindTangent)
	);
}

#[test]
fn two_consistent_tangents_give_one_arc_that_meets_both_smoothly() {
	// A semicircle from (0, 0) up and over to (2, 0): leaves along +Y, arrives along -Y.
	let sweep = arc_tangent_at_both(Vec2::ZERO, Vec2::Y, Vec2::new(2.0, 0.0), -Vec2::Y).unwrap();
	close(sweep, -PI);
	// A shallow bend: leaves 30 degrees left of the chord and arrives 30 degrees left of it.
	let direction = |degrees: f32| Vec2::from_angle(degrees.to_radians());
	let sweep = arc_tangent_at_both(
		Vec2::ZERO,
		direction(-30.0),
		Vec2::new(2.0, 0.0),
		direction(30.0),
	)
	.unwrap();
	close(sweep, 60.0_f32.to_radians());
}

#[test]
fn two_inconsistent_tangents_are_an_error_not_a_free_arc() {
	assert_eq!(
		arc_tangent_at_both(
			Vec2::ZERO,
			Vec2::Y,
			Vec2::new(2.0, 0.0),
			Vec2::new(1.0, -1.0)
		),
		Err(ArcError::InconsistentTangents)
	);
}

#[test]
fn aligned_tangents_along_the_chord_are_a_straight_wall() {
	assert_eq!(
		arc_tangent_at_both(Vec2::ZERO, Vec2::X, Vec2::new(2.0, 0.0), Vec2::X),
		Ok(0.0)
	);
}

#[test]
fn radius_selects_side_and_size_of_the_arc() {
	let (start, end) = (Vec2::new(1.0, 0.0), Vec2::new(-1.0, 0.0));
	close(
		arc_sweep_for_radius(start, end, 1.0, true, false).unwrap(),
		PI,
	);
	let minor = arc_sweep_for_radius(start, end, 2.0, false, false).unwrap();
	let major = arc_sweep_for_radius(start, end, 2.0, false, true).unwrap();
	assert!(minor < 0.0 && major < 0.0);
	close(minor.abs() + major.abs(), 2.0 * PI);
	close(arc_radius(start, end, minor).unwrap(), 2.0);
	assert_eq!(
		arc_sweep_for_radius(start, end, 0.5, true, false),
		Err(ArcError::RadiusTooSmall)
	);
	assert_eq!(
		arc_sweep_for_radius(start, end, -1.0, true, false),
		Err(ArcError::InvalidRadius)
	);
}

#[test]
fn the_midpoint_lies_on_the_arc_on_its_bulging_side() {
	for sweep in [PI, -PI, 1.0, -1.0, 4.0, -4.0] {
		let (start, end) = (Vec2::new(0.0, 0.0), Vec2::new(3.0, 1.0));
		let mut graph = WallGraph::new();
		let wall = graph.add_arc(start, end, sweep).unwrap()[0];
		let expected = graph.wall_position(wall, 0.5).unwrap();
		assert!(
			arc_midpoint(start, end, sweep).distance(expected) < 1e-4,
			"sweep {sweep}"
		);
	}
	assert_eq!(arc_midpoint(Vec2::ZERO, Vec2::new(2.0, 0.0), 0.0), Vec2::X);
}
