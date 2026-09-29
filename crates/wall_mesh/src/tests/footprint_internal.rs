use super::*;

#[test]
fn a_second_circle_intersection_does_not_force_a_distant_miter() {
	let angle = 160f64.to_radians();
	let radius = 0.3;
	let first = Departure {
		wall: 0,
		at_origin: true,
		path: Path::Arc {
			center: DVec2::new(0.0, radius),
			radius,
			start_angle: -PI * 0.5,
			sweep: 3.0,
		},
		half_thickness: 0.13,
	};
	let second_center = DVec2::new(angle.sin(), -angle.cos()) * radius;
	let second = Departure {
		wall: 1,
		at_origin: true,
		path: Path::Arc {
			center: second_center,
			radius,
			start_angle: (-second_center).to_angle(),
			sweep: -3.0,
		},
		half_thickness: 0.18,
	};
	let mut candidates = intersections(
		first.path.offset_curve(first.half_thickness),
		second.path.offset_curve(-second.half_thickness),
	);
	candidates.sort_by(|a, b| a.length_squared().total_cmp(&b.length_squared()));
	assert_eq!(candidates.len(), 2);
	assert!(second.path.distance_near(candidates[0], 0.0) < 0.0);
	assert!(first.path.distance_near(candidates[1], 0.0) > 0.0);
	assert!(second.path.distance_near(candidates[1], 0.0) > 0.0);

	let snapshot = Snapshot {
		nodes: Vec::new(),
		walls: Vec::new(),
	};
	let settings = MeshSettings::default();
	let builder = Builder {
		snapshot: &snapshot,
		settings: &settings,
		vertices: Vec::new(),
		vertex_junction: Vec::new(),
		junction_vertices: Vec::new(),
		cells: Vec::new(),
	};
	let join = builder.corner(&first, &second, DVec2::ZERO);
	assert!(join.bevel);
}
