use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_2, PI};

use glam::{DVec3, Vec2};
use wall_graph::{OpeningSpec, WallDimensions, WallGraph};

use crate::{MeshError, MeshSettings, Side, SurfaceKind, SurfaceSource, WallMesh, generate};

const FINE: MeshSettings = MeshSettings {
	max_deviation: 1e-4,
	miter_limit: 4.0,
	min_pier_width: 0.05,
	max_triangles: 2_000_000,
};

fn dimensions(thickness: f32, height: f32) -> WallDimensions {
	WallDimensions { thickness, height }
}

fn opening(center_distance: f64, width: f32, bottom: f32, height: f32) -> OpeningSpec {
	OpeningSpec {
		center_distance,
		width,
		bottom,
		height,
	}
}

/// Checks that the mesh is a closed, consistently wound, non-degenerate solid and returns its
/// volume in cubic metres.
fn closed_volume(mesh: &WallMesh) -> f64 {
	let positions = mesh.positions();
	let normals = mesh.normals();
	let indices = mesh.indices();
	assert_eq!(positions.len(), normals.len());
	assert_eq!(indices.len() % 3, 0);
	assert!(
		indices
			.iter()
			.all(|&index| (index as usize) < positions.len())
	);

	let mut covered = 0;
	for section in mesh.sections() {
		assert_eq!(section.indices.start, covered);
		assert!(section.indices.end > section.indices.start);
		assert_eq!(section.indices.len() % 3, 0);
		covered = section.indices.end;
	}
	assert_eq!(covered, indices.len());

	for (position, normal) in positions.iter().zip(normals) {
		assert!(position.is_finite());
		assert!((normal.length() - 1.0).abs() < 1e-4, "normal {normal}");
	}

	// Faces have separate vertices, so weld identical positions before checking edges.
	let mut welded: HashMap<[u32; 3], usize> = HashMap::new();
	let ids: Vec<usize> = positions
		.iter()
		.map(|position| {
			// Adding zero turns -0.0 into 0.0 so both weld together.
			let key = (*position + 0.0).to_array().map(f32::to_bits);
			let next = welded.len();
			*welded.entry(key).or_insert(next)
		})
		.collect();

	let mut edges: HashMap<(usize, usize), u32> = HashMap::new();
	let mut volume = 0.0;
	for triangle in indices.chunks_exact(3) {
		let [a, b, c] = [0, 1, 2].map(|k| triangle[k] as usize);
		let corners = [a, b, c].map(|index| positions[index].as_dvec3());
		let cross = (corners[1] - corners[0]).cross(corners[2] - corners[0]);
		assert!(cross.length() > 1e-12, "degenerate triangle {corners:?}");
		let face_normal = cross.normalize();
		for index in [a, b, c] {
			assert!(
				normals[index].as_dvec3().dot(face_normal) > 0.0,
				"vertex normal points away from its triangle at {corners:?}"
			);
		}
		volume += corners[0].dot(corners[1].cross(corners[2])) / 6.0;
		let [a, b, c] = [ids[a], ids[b], ids[c]];
		assert!(a != b && b != c && c != a, "triangle collapses when welded");
		for edge in [(a, b), (b, c), (c, a)] {
			*edges.entry(edge).or_default() += 1;
		}
	}
	for (&(from, to), &count) in &edges {
		assert_eq!(count, 1, "edge used twice in one direction");
		assert_eq!(
			edges.get(&(to, from)),
			Some(&1),
			"edge without an opposite neighbour"
		);
	}
	volume
}

fn assert_close(actual: f64, expected: f64, tolerance: f64) {
	assert!(
		(actual - expected).abs() <= tolerance,
		"expected {expected}, got {actual}"
	);
}

fn has_section(mesh: &WallMesh, kind: SurfaceKind) -> bool {
	mesh.sections()
		.iter()
		.any(|section| section.tag.kind == kind)
}

#[test]
fn an_empty_graph_has_an_empty_mesh() {
	let mesh = generate(&WallGraph::new(), &MeshSettings::default()).unwrap();
	assert!(mesh.is_empty());
	assert!(mesh.positions().is_empty());
}

#[test]
fn a_free_wall_is_a_closed_box_with_tagged_faces() {
	let mut graph = WallGraph::new();
	let wall = graph.add_wall(Vec2::ZERO, Vec2::new(4.0, 0.0)).unwrap()[0];

	let mesh = generate(&graph, &FINE).unwrap();
	assert_close(closed_volume(&mesh), 4.0 * 0.2 * 2.5, 1e-5);
	assert_eq!(mesh.indices().len(), 12 * 3);
	for kind in [
		SurfaceKind::Side(Side::Left),
		SurfaceKind::Side(Side::Right),
		SurfaceKind::Top,
		SurfaceKind::Bottom,
		SurfaceKind::EndCap,
	] {
		assert!(has_section(&mesh, kind), "{kind:?}");
	}
	assert!(
		mesh.sections()
			.iter()
			.all(|section| section.tag.source == SurfaceSource::Wall(wall))
	);
	// The left face of a wall along +X faces +Y.
	let left = mesh
		.sections()
		.iter()
		.find(|section| section.tag.kind == SurfaceKind::Side(Side::Left))
		.unwrap();
	let normal = mesh.normals()[mesh.indices()[left.indices.start] as usize];
	assert!(normal.y > 0.99);
}

#[test]
fn doors_and_windows_cut_through_with_reveals() {
	let mut graph = WallGraph::new();
	let wall = graph.add_wall(Vec2::ZERO, Vec2::new(6.0, 0.0)).unwrap()[0];
	let door = graph
		.add_opening(wall, opening(1.5, 1.0, 0.0, 2.1))
		.unwrap();
	let window = graph
		.add_opening(wall, opening(4.0, 1.2, 0.9, 1.2))
		.unwrap();

	let mesh = generate(&graph, &FINE).unwrap();
	let volume = 6.0 * 0.2 * 2.5 - 1.0 * 0.2 * 2.1 - 1.2 * 0.2 * 1.2;
	assert_close(closed_volume(&mesh), volume, 1e-5);
	let sources = |kind| -> Vec<SurfaceSource> {
		mesh.sections()
			.iter()
			.filter(|section| section.tag.kind == kind)
			.map(|section| section.tag.source)
			.collect()
	};
	assert!(sources(SurfaceKind::Jamb).contains(&SurfaceSource::Opening(door)));
	assert!(sources(SurfaceKind::Jamb).contains(&SurfaceSource::Opening(window)));
	assert!(sources(SurfaceKind::Head).contains(&SurfaceSource::Opening(door)));
	assert!(sources(SurfaceKind::Head).contains(&SurfaceSource::Opening(window)));
	assert_eq!(
		sources(SurfaceKind::Sill),
		vec![SurfaceSource::Opening(window)]
	);
}

#[test]
fn stacked_and_staggered_openings_stay_closed() {
	let mut graph = WallGraph::new();
	let wall = graph.add_wall(Vec2::ZERO, Vec2::new(5.0, 0.0)).unwrap()[0];
	graph
		.add_opening(wall, opening(1.0, 0.8, 0.3, 0.8))
		.unwrap();
	graph
		.add_opening(wall, opening(1.0, 0.8, 1.2, 0.8))
		.unwrap();
	// Overlaps the next opening along the wall but is separated from it vertically.
	graph
		.add_opening(wall, opening(2.5, 1.0, 0.2, 1.0))
		.unwrap();
	graph
		.add_opening(wall, opening(3.25, 1.5, 1.3, 1.0))
		.unwrap();

	let mesh = generate(&graph, &FINE).unwrap();
	let removed = 2.0 * 0.8 * 0.8 + 1.0 * 1.0 + 1.5 * 1.0;
	assert_close(closed_volume(&mesh), 0.2 * (5.0 * 2.5 - removed), 1e-5);
}

#[test]
fn an_opening_reaching_the_wall_top_leaves_no_sliver() {
	let mut graph = WallGraph::new();
	let wall = graph
		.add_wall_with_dimensions(Vec2::ZERO, Vec2::new(3.0, 0.0), dimensions(0.2, 2.1))
		.unwrap()[0];
	graph
		.add_opening(wall, opening(1.5, 1.0, 0.9, 1.2))
		.unwrap();

	let mesh = generate(&graph, &FINE).unwrap();
	assert_close(closed_volume(&mesh), 0.2 * (3.0 * 2.1 - 1.0 * 1.2), 1e-5);
	assert!(!has_section(&mesh, SurfaceKind::Head));
}

#[test]
fn an_equal_height_corner_has_no_internal_faces() {
	let mut graph = WallGraph::new();
	graph.add_wall(Vec2::ZERO, Vec2::new(3.0, 0.0)).unwrap();
	graph.add_wall(Vec2::ZERO, Vec2::new(0.0, 2.0)).unwrap();

	let mesh = generate(&graph, &FINE).unwrap();
	assert_close(closed_volume(&mesh), 0.2 * 5.0 * 2.5, 1e-5);
	assert!(!has_section(&mesh, SurfaceKind::JunctionStep));
}

#[test]
fn a_five_degree_corner_has_a_closed_bevelled_mesh() {
	let mut graph = WallGraph::new();
	graph.add_wall(Vec2::ZERO, Vec2::new(30.0, 0.0)).unwrap();
	graph
		.add_wall(Vec2::ZERO, Vec2::from_angle(5f32.to_radians()) * 30.0)
		.unwrap();

	let mesh = generate(&graph, &FINE).unwrap();
	assert!(has_section(&mesh, SurfaceKind::Bevel));
	assert!(closed_volume(&mesh) > 0.0);
}

#[test]
fn the_taller_wall_wraps_a_mixed_height_corner() {
	let mut graph = WallGraph::new();
	graph
		.add_wall_with_dimensions(Vec2::ZERO, Vec2::new(3.0, 0.0), dimensions(0.2, 3.0))
		.unwrap();
	graph
		.add_wall_with_dimensions(Vec2::ZERO, Vec2::new(0.0, 2.0), dimensions(0.2, 2.5))
		.unwrap();

	let mesh = generate(&graph, &FINE).unwrap();
	let volume = 0.2 * 2.9 * 3.0 + 0.2 * 1.9 * 2.5 + 0.04 * 3.0;
	assert_close(closed_volume(&mesh), volume, 1e-5);
	assert!(has_section(&mesh, SurfaceKind::JunctionStep));
}

#[test]
fn a_mixed_t_and_a_crossing_stay_closed() {
	let mut graph = WallGraph::new();
	graph
		.add_wall(Vec2::new(-2.0, 0.0), Vec2::new(2.0, 0.0))
		.unwrap();
	graph
		.add_wall_with_dimensions(Vec2::ZERO, Vec2::new(0.0, 3.0), dimensions(0.1, 3.0))
		.unwrap();
	let mesh = generate(&graph, &FINE).unwrap();
	let volume = 0.2 * 3.9 * 2.5 + 0.1 * 2.9 * 3.0 + 0.1 * 0.2 * 3.0;
	assert_close(closed_volume(&mesh), volume, 1e-5);

	let mut graph = WallGraph::new();
	graph
		.add_wall(Vec2::new(-2.0, 0.0), Vec2::new(2.0, 0.0))
		.unwrap();
	graph
		.add_wall(Vec2::new(0.0, -2.0), Vec2::new(0.0, 2.0))
		.unwrap();
	let mesh = generate(&graph, &FINE).unwrap();
	assert_close(closed_volume(&mesh), (0.8 + 0.8 - 0.04) * 2.5, 1e-5);
}

#[test]
fn collinear_walls_of_different_size_meet_with_step_faces() {
	let mut graph = WallGraph::new();
	graph.add_wall(Vec2::new(-2.0, 0.0), Vec2::ZERO).unwrap();
	graph
		.add_wall_with_dimensions(Vec2::ZERO, Vec2::new(2.0, 0.0), dimensions(0.3, 3.0))
		.unwrap();

	let mesh = generate(&graph, &FINE).unwrap();
	assert_close(closed_volume(&mesh), 0.4 * 2.5 + 0.6 * 3.0, 1e-5);
	assert!(has_section(&mesh, SurfaceKind::JunctionStep));

	// Off the axes, the split cross-section is collinear only to within rounding.
	let direction = Vec2::from_angle(0.65);
	let mut graph = WallGraph::new();
	graph.add_wall(direction * -2.0, Vec2::ZERO).unwrap();
	graph
		.add_wall_with_dimensions(Vec2::ZERO, direction * 2.0, dimensions(0.3, 3.0))
		.unwrap();
	let mesh = generate(&graph, &FINE).unwrap();
	assert_close(closed_volume(&mesh), 0.4 * 2.5 + 0.6 * 3.0, 1e-4);
}

#[test]
fn nearly_straight_walls_of_different_size_stay_closed() {
	let mut graph = WallGraph::new();
	graph.add_wall(Vec2::new(-2.0, 0.0), Vec2::ZERO).unwrap();
	graph
		.add_wall_with_dimensions(Vec2::ZERO, Vec2::new(2.0, 0.02), dimensions(0.3, 3.0))
		.unwrap();

	let mesh = generate(&graph, &FINE).unwrap();
	assert_close(closed_volume(&mesh), 0.4 * 2.5 + 0.6 * 3.0, 2e-3);
}

#[test]
fn curved_walls_with_openings_stay_closed() {
	let mut graph = WallGraph::new();
	let arc = graph
		.add_arc(Vec2::new(2.0, 0.0), Vec2::new(-2.0, 0.0), PI)
		.unwrap()[0];
	let center = graph.wall_length(arc).unwrap() * 0.5;
	graph
		.add_opening(arc, opening(center, 0.5, 0.8, 1.0))
		.unwrap();

	let mesh = generate(&graph, &FINE).unwrap();
	// An annular opening removes its centerline width times the thickness.
	let volume = PI as f64 * 2.0 * 0.2 * 2.5 - 0.5 * 0.2 * 1.0;
	assert_close(closed_volume(&mesh), volume, 1e-3);
}

#[test]
fn major_arcs_of_both_turns_have_closed_meshes() {
	for sweep in [1.5 * PI, -1.5 * PI] {
		let mut graph = WallGraph::new();
		let arc = graph
			.add_arc(Vec2::new(2.0, 0.0), Vec2::from_angle(sweep) * 2.0, sweep)
			.unwrap()[0];
		graph
			.add_opening(
				arc,
				opening(graph.wall_length(arc).unwrap() * 0.5, 0.5, 0.8, 1.0),
			)
			.unwrap();

		let mesh = generate(&graph, &FINE).unwrap();
		let expected_volume = sweep.abs() as f64 * 2.0 * 0.2 * 2.5 - 0.5 * 0.2 * 1.0;
		assert_close(closed_volume(&mesh), expected_volume, 1e-3);
	}
}

#[test]
fn mixed_width_arc_to_arc_junction_has_a_closed_mesh() {
	let mut graph = WallGraph::new();
	graph
		.add_arc(Vec2::ZERO, Vec2::new(2.0, 2.0), FRAC_PI_2)
		.unwrap();
	graph
		.add_arc_with_dimensions(
			Vec2::new(2.0, 2.0),
			Vec2::new(4.0, 0.0),
			-FRAC_PI_2,
			dimensions(0.3, 3.0),
		)
		.unwrap();

	let mesh = generate(&graph, &FINE).unwrap();
	assert!(closed_volume(&mesh) > 0.0);
	assert!(has_section(&mesh, SurfaceKind::JunctionStep));
}

#[test]
fn a_small_house_stays_closed() {
	let mut graph = WallGraph::new();
	let bottom = graph
		.add_wall(Vec2::new(-3.0, -2.0), Vec2::new(3.0, -2.0))
		.unwrap()[0];
	graph
		.add_wall(Vec2::new(3.0, -2.0), Vec2::new(3.0, 2.0))
		.unwrap();
	graph
		.add_wall(Vec2::new(3.0, 2.0), Vec2::new(-3.0, 2.0))
		.unwrap();
	graph
		.add_wall(Vec2::new(-3.0, 2.0), Vec2::new(-3.0, -2.0))
		.unwrap();
	graph
		.add_arc(Vec2::new(3.0, 2.0), Vec2::new(5.0, 0.0), -FRAC_PI_2)
		.unwrap();
	graph
		.add_wall_with_dimensions(
			Vec2::new(4.0, -0.6),
			Vec2::new(4.0, 2.6),
			dimensions(0.15, 3.0),
		)
		.unwrap();
	graph
		.add_wall_with_dimensions(
			Vec2::new(0.0, -2.0),
			Vec2::new(0.0, 2.0),
			dimensions(0.1, 2.0),
		)
		.unwrap();
	let piece = graph
		.walls()
		.find(|&wall| graph.wall_position(wall, 0.0) == Some(Vec2::new(-3.0, -2.0)))
		.unwrap_or(bottom);
	graph
		.add_opening(piece, opening(1.5, 0.9, 0.0, 2.1))
		.unwrap();

	let mesh = generate(&graph, &MeshSettings::default()).unwrap();
	assert!(closed_volume(&mesh) > 0.0);
}

#[test]
fn generation_is_deterministic() {
	let mut graph = WallGraph::new();
	graph
		.add_wall(Vec2::new(-2.0, 0.0), Vec2::new(2.0, 0.0))
		.unwrap();
	graph
		.add_wall(Vec2::new(0.0, -2.0), Vec2::new(0.0, 2.0))
		.unwrap();
	graph
		.add_arc(Vec2::new(2.0, 0.0), Vec2::new(0.0, 2.0), FRAC_PI_2)
		.unwrap();

	let first = generate(&graph, &MeshSettings::default()).unwrap();
	let second = generate(&graph.clone(), &MeshSettings::default()).unwrap();
	assert_eq!(first, second);
}

#[test]
fn crossing_geometry_is_independent_of_insertion_order() {
	let walls = [
		(Vec2::new(-2.0, 0.0), Vec2::new(2.0, 0.0)),
		(Vec2::new(0.0, -2.0), Vec2::new(0.0, 2.0)),
		(Vec2::new(2.0, 0.0), Vec2::new(3.0, 1.0)),
	];
	let mut reference = None;
	for order in [[0, 1, 2], [2, 0, 1], [1, 2, 0]] {
		let mut graph = WallGraph::new();
		for index in order {
			let (start, end) = walls[index];
			graph.add_wall(start, end).unwrap();
		}
		let mesh = generate(&graph, &FINE).unwrap();
		assert!(closed_volume(&mesh) > 0.0);
		let mut triangles: Vec<[[u32; 3]; 3]> = mesh
			.indices()
			.chunks_exact(3)
			.map(|triangle| {
				let mut corners = [triangle[0], triangle[1], triangle[2]].map(|index| {
					mesh.positions()[index as usize]
						.to_array()
						.map(|coordinate| (coordinate + 0.0).to_bits())
				});
				corners.sort_unstable();
				corners
			})
			.collect();
		triangles.sort_unstable();
		if let Some(expected) = &reference {
			assert_eq!(&triangles, expected);
		} else {
			reference = Some(triangles);
		}
	}
}

#[test]
fn invalid_settings_and_limits_are_reported() {
	let mut graph = WallGraph::new();
	graph.add_wall(Vec2::ZERO, Vec2::new(4.0, 0.0)).unwrap();

	for settings in [
		MeshSettings {
			max_deviation: 0.0,
			..MeshSettings::default()
		},
		MeshSettings {
			miter_limit: f64::NAN,
			..MeshSettings::default()
		},
		MeshSettings {
			min_pier_width: -1.0,
			..MeshSettings::default()
		},
		MeshSettings {
			max_triangles: 0,
			..MeshSettings::default()
		},
	] {
		assert_eq!(generate(&graph, &settings), Err(MeshError::InvalidSettings));
	}
	assert_eq!(
		generate(
			&graph,
			&MeshSettings {
				max_triangles: 11,
				..MeshSettings::default()
			}
		),
		Err(MeshError::OutputLimitExceeded)
	);
}

#[test]
fn smooth_normals_follow_curved_faces() {
	let mut graph = WallGraph::new();
	graph
		.add_arc(Vec2::new(2.0, 0.0), Vec2::new(-2.0, 0.0), PI)
		.unwrap();

	let mesh = generate(&graph, &MeshSettings::default()).unwrap();
	for section in mesh.sections() {
		let SurfaceKind::Side(side) = section.tag.kind else {
			continue;
		};
		for &index in &mesh.indices()[section.indices.clone()] {
			let position = mesh.positions()[index as usize].as_dvec3();
			let radial = DVec3::new(position.x, position.y, 0.0).normalize();
			let normal = mesh.normals()[index as usize].as_dvec3();
			// The left face of a counterclockwise arc faces its center.
			let expected = if side == Side::Left { -radial } else { radial };
			assert!(normal.dot(expected) > 0.9999);
		}
	}
}
