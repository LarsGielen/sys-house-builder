use super::*;

fn close(a: DVec2, b: DVec2) -> bool {
	a.distance(b) < 1e-12
}

#[test]
fn arc_offsets_move_toward_the_center_on_the_left_of_counterclockwise_travel() {
	let arc = Path::Arc {
		center: DVec2::ZERO,
		radius: 2.0,
		start_angle: 0.0,
		sweep: PI,
	};
	assert!(close(arc.point(0.0, 0.5), DVec2::new(1.5, 0.0)));
	assert!(close(arc.point(0.0, -0.5), DVec2::new(2.5, 0.0)));
	assert!(close(arc.tangent(0.0), DVec2::Y));
	assert!(close(arc.point(arc.length(), 0.0), DVec2::new(-2.0, 0.0)));

	let clockwise = arc.reversed();
	assert!(close(clockwise.point(0.0, 0.5), DVec2::new(-2.5, 0.0)));
	assert!(close(clockwise.tangent(0.0), DVec2::Y));
}

#[test]
fn distances_of_offset_points_include_positions_behind_the_start() {
	let line = Path::new(WallCurve::Straight {
		start: DVec2::new(1.0, 1.0),
		end: DVec2::new(4.0, 1.0),
	});
	assert_eq!(
		line.reversed().distance_near(DVec2::new(5.0, 3.0), 0.0),
		-1.0
	);

	let arc = Path::Arc {
		center: DVec2::ZERO,
		radius: 2.0,
		start_angle: 0.0,
		sweep: -1.5 * PI,
	};
	let behind = arc.point(-0.3, 0.2);
	assert!((arc.distance_near(behind, 0.0) + 0.3).abs() < 1e-12);
	let far = arc.point(4.0, -0.2);
	assert!((arc.distance_near(far, arc.length()) - 4.0).abs() < 1e-12);
}

#[test]
fn arc_samples_keep_chords_within_the_deviation_on_the_requested_offset() {
	let arc = Path::Arc {
		center: DVec2::ZERO,
		radius: 1.0,
		start_angle: 0.0,
		sweep: PI,
	};
	let offset = arc.outer_offset(0.5);
	let samples = arc.samples(0.2, 2.9, offset, 0.001).unwrap();
	assert_eq!(samples.first(), Some(&0.2));
	assert_eq!(samples.last(), Some(&2.9));
	for pair in samples.windows(2) {
		let a = arc.point(pair[0], offset);
		let b = arc.point(pair[1], offset);
		let middle = arc.point((pair[0] + pair[1]) * 0.5, offset);
		assert!(middle.distance((a + b) * 0.5) <= 0.001 + 1e-12);
	}
	assert_eq!(
		arc.samples(0.0, 3.0, offset, 1e-15),
		Err(MeshError::OutputLimitExceeded)
	);
}

#[test]
fn offset_curves_intersect_as_lines_and_circles() {
	let horizontal = OffsetCurve::Line {
		point: DVec2::new(0.0, 1.0),
		direction: DVec2::X,
	};
	let vertical = OffsetCurve::Line {
		point: DVec2::new(2.0, 0.0),
		direction: DVec2::Y,
	};
	assert_eq!(
		intersections(horizontal, vertical),
		vec![DVec2::new(2.0, 1.0)]
	);
	assert!(
		intersections(
			horizontal,
			OffsetCurve::Line {
				point: DVec2::ZERO,
				direction: -DVec2::X
			}
		)
		.is_empty()
	);

	let circle = OffsetCurve::Circle {
		center: DVec2::ZERO,
		radius: 2.0,
	};
	let points = intersections(horizontal, circle);
	assert_eq!(points.len(), 2);
	assert!(
		points
			.iter()
			.all(|p| (p.length() - 2.0).abs() < 1e-12 && p.y == 1.0)
	);

	let other = OffsetCurve::Circle {
		center: DVec2::new(2.0, 0.0),
		radius: 2.0,
	};
	let points = intersections(circle, other);
	assert_eq!(points.len(), 2);
	assert!(points.iter().all(|p| (p.x - 1.0).abs() < 1e-12));
}
