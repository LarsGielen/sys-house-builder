use super::*;

fn area(points: &[DVec2], triangles: &[[usize; 3]]) -> f64 {
	triangles
		.iter()
		.map(|&[a, b, c]| double_area(points[a], points[b], points[c]) * 0.5)
		.sum()
}

#[test]
fn concave_polygons_are_covered_by_positive_triangles() {
	let points = [
		DVec2::new(0.0, 0.0),
		DVec2::new(4.0, 0.0),
		DVec2::new(4.0, 3.0),
		DVec2::new(2.0, 1.0),
		DVec2::new(0.0, 3.0),
	];
	let triangles = triangulate(&points).unwrap();
	assert_eq!(triangles.len(), 3);
	assert!(
		triangles
			.iter()
			.all(|&[a, b, c]| double_area(points[a], points[b], points[c]) > 0.0)
	);
	assert!((area(&points, &triangles) - 8.0).abs() < 1e-12);
}

#[test]
fn collinear_vertices_stay_in_nondegenerate_triangles() {
	let points = [
		DVec2::new(0.0, 0.0),
		DVec2::new(1.0, 0.0),
		DVec2::new(2.0, 0.0),
		DVec2::new(2.0, 1.0),
		DVec2::new(1.0, 1.0),
		DVec2::new(0.0, 1.0),
	];
	let triangles = triangulate(&points).unwrap();
	assert_eq!(triangles.len(), 4);
	for index in 0..points.len() {
		assert!(triangles.iter().any(|triangle| triangle.contains(&index)));
	}
	assert!((area(&points, &triangles) - 2.0).abs() < 1e-12);
}

#[test]
fn nearly_collinear_vertices_are_not_left_in_the_final_triangle() {
	// The left edge bulges outward by 1e-17 m, as junction splits can after rounding.
	let points = [
		DVec2::new(1e-17, -0.15),
		DVec2::new(2.0, -0.15),
		DVec2::new(2.0, 0.15),
		DVec2::new(1e-17, 0.15),
		DVec2::new(-1e-17, 0.1),
		DVec2::new(-1e-17, -0.1),
	];
	let triangles = triangulate(&points).unwrap();
	assert_eq!(triangles.len(), 4);
	assert!((area(&points, &triangles) - 0.6).abs() < 1e-12);
}

#[test]
fn degenerate_polygons_fail() {
	let line = [DVec2::ZERO, DVec2::X, DVec2::X * 2.0];
	assert_eq!(triangulate(&line), None);
}
