use std::f64::consts::{FRAC_PI_2, PI, TAU};

use glam::{DVec2, Vec2};

use super::{Curve, CurveShape, DISTANCE_TOLERANCE, intersections};

fn line(start: (f32, f32), end: (f32, f32)) -> Curve {
	Curve::new(Vec2::from(start), Vec2::from(end), CurveShape::Straight)
}

fn arc(start: (f32, f32), end: (f32, f32), sweep: f64) -> Curve {
	Curve::new(
		Vec2::from(start),
		Vec2::from(end),
		CurveShape::CircularArc { sweep },
	)
}

fn contact_parameters(first: Curve, second: Curve) -> Vec<(f64, f64)> {
	intersections(first, second)
		.contacts
		.into_iter()
		.map(|contact| (contact.first_parameter, contact.second_parameter))
		.collect()
}

fn close(actual: DVec2, expected: DVec2) {
	assert!(actual.distance(expected) < 1e-8, "{actual} != {expected}");
}

#[test]
fn segment_evaluation_projection_and_tangent() {
	let curve = line((1.0, 2.0), (5.0, 2.0));
	close(curve.position(0.25), DVec2::new(2.0, 2.0));
	close(curve.tangent(0.7), DVec2::X);
	assert_eq!(curve.parameter_of(DVec2::new(2.0, 2.0)), Some(0.25));
	assert_eq!(curve.parameter_of(DVec2::new(0.0, 2.0)), None);
	assert_eq!(curve.parameter_of(DVec2::new(2.0, 3.0)), None);
	assert!(curve.interior_parameter(curve.start).is_none());
	assert!(curve.interior_parameter(curve.end).is_none());
	assert!(
		curve
			.interior_parameter(DVec2::new(2.0, 2.0 + DISTANCE_TOLERANCE / 2.0))
			.is_some()
	);
	assert_eq!(curve.length(), 4.0);
}

#[test]
fn clockwise_and_counterclockwise_semicircles() {
	for sign in [-1.0, 1.0] {
		let curve = arc((1.0, 0.0), (-1.0, 0.0), sign * PI);
		close(curve.position(0.5), DVec2::new(0.0, sign));
		close(curve.tangent(0.0), DVec2::new(0.0, sign));
		close(curve.tangent(1.0), DVec2::new(0.0, -sign));
		assert!((curve.length() - PI).abs() < 1e-10);
		assert!((curve.parameter_of(DVec2::new(0.0, sign)).unwrap() - 0.5).abs() < 1e-10);
		assert_eq!(curve.parameter_of(DVec2::new(0.0, -sign)), None);
		assert_eq!(curve.parameter_of(DVec2::new(0.0, sign * 1.01)), None);
	}
}

#[test]
fn major_arcs_select_the_long_path() {
	let curve = arc((1.0, 0.0), (0.0, -1.0), 3.0 * FRAC_PI_2);
	close(curve.position(1.0 / 3.0), DVec2::Y);
	close(curve.position(2.0 / 3.0), -DVec2::X);
	assert!((curve.length() - 3.0 * FRAC_PI_2).abs() < 1e-10);
	assert!(curve.parameter_of(DVec2::Y).is_some());
}

#[test]
fn reversal_and_subcurves_preserve_the_path() {
	for sweep in [FRAC_PI_2, -PI, 1.5 * PI, 1e-6] {
		let curve = arc((2.0, 1.0), (-1.0, 3.0), sweep);
		let reverse = Curve {
			start: curve.end,
			end: curve.start,
			shape: curve.shape.reversed(),
		};
		let piece = Curve {
			start: curve.position(0.2),
			end: curve.position(0.8),
			shape: curve.subcurve(0.2, 0.8).shape,
		};
		for parameter in [0.0, 0.1, 0.5, 0.9, 1.0] {
			close(reverse.position(parameter), curve.position(1.0 - parameter));
			close(reverse.tangent(parameter), -curve.tangent(1.0 - parameter));
			close(
				piece.position(parameter),
				curve.position(0.2 + parameter * 0.6),
			);
		}
		assert!((piece.length() - curve.length() * 0.6).abs() < 1e-8);
	}
}

#[test]
fn invalid_and_degenerate_curves_are_rejected() {
	for sweep in [0.0, TAU, -TAU, f64::NAN, f64::INFINITY] {
		assert!(!arc((0.0, 0.0), (1.0, 0.0), sweep).valid());
	}
	assert!(!line((0.0, 0.0), (0.0, 0.0)).valid());
	assert!(!line((0.0, 0.0), (f32::INFINITY, 0.0)).valid());
	assert!(arc((0.0, 0.0), (1.0, 0.0), 1e-6).valid());
}

#[test]
fn segment_intersections_include_endpoint_contacts() {
	let first = line((0.0, 0.0), (4.0, 0.0));
	let second = line((1.0, -1.0), (1.0, 3.0));
	assert_eq!(contact_parameters(first, second), vec![(0.25, 0.25)]);
	assert_eq!(
		contact_parameters(first, line((4.0, 0.0), (4.0, 2.0))),
		vec![(1.0, 0.0)]
	);
	assert!(
		intersections(first, line((5.0, -1.0), (5.0, 1.0)))
			.contacts
			.is_empty()
	);
	assert!(
		intersections(first, line((0.0, 1.0), (4.0, 1.0)))
			.contacts
			.is_empty()
	);
}

#[test]
fn segment_overlap_requires_a_shared_stretch() {
	let base = line((0.0, 0.0), (2.0, 0.0));
	for other in [
		line((1.0, 0.0), (3.0, 0.0)),
		line((0.5, 0.0), (1.5, 0.0)),
		line((2.0, 0.0), (0.0, 0.0)),
	] {
		assert!(intersections(base, other).overlapping);
		assert!(intersections(other, base).overlapping);
	}
	assert!(!intersections(base, line((2.0, 0.0), (3.0, 0.0))).overlapping);
}

#[test]
fn line_arc_intersections_can_have_two_contacts_and_are_symmetric() {
	let circle = arc((1.0, 0.0), (-1.0, 0.0), PI);
	let segment = line((-2.0, 0.5), (2.0, 0.5));
	let contacts = contact_parameters(segment, circle);
	assert_eq!(contacts.len(), 2);
	for (first, second) in &contacts {
		close(segment.position(*first), circle.position(*second));
	}
	let reverse = contact_parameters(circle, segment);
	assert_eq!(reverse.len(), 2);
	for (first, second) in reverse {
		assert!(
			contacts
				.iter()
				.any(|&(a, b)| (a - second).abs() < 1e-10 && (b - first).abs() < 1e-10)
		);
	}
}

#[test]
fn circle_intersections_filter_contacts_outside_the_arc() {
	let first = arc((-1.0, 0.0), (0.0, -1.0), -1.5 * PI);
	let second = arc((2.0, 0.0), (1.0, -1.0), 1.5 * PI);
	let contacts = contact_parameters(first, second);
	assert_eq!(contacts.len(), 2);
	for (a, b) in contacts {
		close(first.position(a), second.position(b));
	}
	let upper = arc((1.0, 0.0), (-1.0, 0.0), PI);
	let other_upper = arc((2.0, 0.0), (0.0, 0.0), PI);
	assert_eq!(intersections(upper, other_upper).contacts.len(), 1);
}

#[test]
fn coincident_arcs_detect_overlap_across_the_angle_boundary() {
	let first = arc((0.0, -1.0), (0.0, 1.0), PI);
	let second = arc((1.0, 0.0), (-1.0, 0.0), PI);
	assert!(intersections(first, second).overlapping);
	let left = arc((0.0, 1.0), (0.0, -1.0), PI);
	assert!(!intersections(first, left).overlapping);
	assert_eq!(intersections(first, left).contacts.len(), 2);
}

#[test]
fn tangency_produces_one_contact_and_disjoint_circles_none() {
	let curve = arc((1.0, 0.0), (-1.0, 0.0), PI);
	assert_eq!(
		intersections(curve, line((-2.0, 1.0), (2.0, 1.0)))
			.contacts
			.len(),
		1
	);
	assert!(
		intersections(curve, line((-2.0, 1.001), (2.0, 1.001)))
			.contacts
			.is_empty()
	);
	let touching = arc((1.0, 2.0), (-1.0, 2.0), -PI);
	assert_eq!(intersections(curve, touching).contacts.len(), 1);
	let distant = arc((1.0, 3.0), (-1.0, 3.0), -PI);
	assert!(intersections(curve, distant).contacts.is_empty());
}

#[test]
fn sampling_respects_the_chord_error() {
	for sweep in [PI, -FRAC_PI_2, 1.5 * PI, 1e-6] {
		let curve = arc((1.0, 0.0), (-1.0, 0.0), sweep);
		for deviation in [0.001, 0.1, 10.0] {
			let count = curve.sample_count(deviation);
			for index in 0..count {
				let start = curve.position(index as f64 / count as f64);
				let end = curve.position((index + 1) as f64 / count as f64);
				let midpoint = curve.position((index as f64 + 0.5) / count as f64);
				assert!(midpoint.distance((start + end) * 0.5) <= deviation + 1e-10);
			}
		}
	}
}

#[test]
fn snapping_error_bound_covers_the_whole_curve() {
	for sweep in [FRAC_PI_2, -PI, 1.8 * PI, 1e-6] {
		let original = arc((1.0, 0.0), (-1.0, 0.0), sweep);
		let piece = Curve {
			start: original.position(0.1) + DVec2::new(0.00001, -0.00002),
			end: original.position(0.9) + DVec2::new(-0.00002, 0.00003),
			shape: original.subcurve(0.1, 0.9).shape,
		};
		let bound = piece.deviation_from(original, 0.1, 0.9);
		let measured = (0..=1000)
			.map(|index| {
				let parameter = index as f64 / 1000.0;
				piece
					.position(parameter)
					.distance(original.position(0.1 + parameter * 0.8))
			})
			.fold(0.0, f64::max);
		assert!(bound + 1e-12 >= measured, "{bound} < {measured}");
		assert!((bound - measured).abs() < 1e-8);
	}
}
