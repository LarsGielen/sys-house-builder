use std::f32::consts::{FRAC_PI_2, PI};

use glam::Vec2;
use wall_graph::{OpeningSpec, WallDimensions, WallGraph};

use crate::footprint::{self, CellKind, EdgeKind, Footprint, polygon_area};
use crate::snapshot::Snapshot;
use crate::{MeshError, MeshSettings, SurfaceSource};

use super::svg;

const FINE: MeshSettings = MeshSettings {
	max_deviation: 1e-5,
	miter_limit: 4.0,
	min_pier_width: 0.05,
	max_triangles: 2_000_000,
};

fn thickness(thickness: f32) -> WallDimensions {
	WallDimensions {
		thickness,
		height: 2.5,
	}
}

fn try_resolve(graph: &WallGraph, settings: &MeshSettings) -> Result<Footprint, MeshError> {
	footprint::build(&Snapshot::new(graph), settings)
}

/// Resolves a fixture, checks cell invariants, and writes its SVG when requested.
fn resolve(name: &str, graph: &WallGraph, settings: &MeshSettings) -> Footprint {
	let footprint = try_resolve(graph, settings).expect("footprint resolves");
	for cell in &footprint.cells {
		assert_eq!(cell.boundary.len(), cell.edges.len());
		assert!(cell.boundary.len() >= 3);
		assert!(
			polygon_area(&footprint.vertices, &cell.boundary) > 0.0,
			"{name}: cell is not counterclockwise"
		);
		for (from, to) in cell.edge_vertices() {
			assert_ne!(from, to, "{name}: repeated vertex");
		}
	}
	svg::write_if_requested(name, &footprint);
	footprint
}

fn area(footprint: &Footprint) -> f64 {
	footprint
		.cells
		.iter()
		.map(|cell| polygon_area(&footprint.vertices, &cell.boundary))
		.sum()
}

fn cores(footprint: &Footprint) -> usize {
	footprint
		.cells
		.iter()
		.filter(|cell| matches!(cell.kind, CellKind::Core { .. }))
		.count()
}

fn has_bevel(footprint: &Footprint) -> bool {
	footprint.cells.iter().any(|cell| {
		cell.edges
			.iter()
			.any(|edge| matches!(edge, EdgeKind::Bevel { .. }))
	})
}

/// Dimensions are `f32`, so exact areas are off by about 1e-8 in `f64`.
fn assert_close(actual: f64, expected: f64, tolerance: f64) {
	let tolerance = tolerance.max(1e-6);
	assert!(
		(actual - expected).abs() <= tolerance,
		"expected {expected}, got {actual}"
	);
}

#[test]
fn a_free_wall_is_one_rectangle() {
	let mut graph = WallGraph::new();
	graph.add_wall(Vec2::ZERO, Vec2::new(4.0, 0.0)).unwrap();

	let footprint = resolve("straight", &graph, &FINE);
	assert_eq!(footprint.cells.len(), 1);
	assert_eq!(footprint.cells[0].boundary.len(), 4);
	assert_close(area(&footprint), 0.8, 1e-12);
}

#[test]
fn a_right_angle_corner_fills_its_outside_square() {
	let mut graph = WallGraph::new();
	graph.add_wall(Vec2::ZERO, Vec2::new(3.0, 0.0)).unwrap();
	graph.add_wall(Vec2::ZERO, Vec2::new(0.0, 2.0)).unwrap();

	let footprint = resolve("corner_90", &graph, &FINE);
	assert_eq!(cores(&footprint), 1);
	assert!(!has_bevel(&footprint));
	assert_close(area(&footprint), 0.2 * 5.0, 1e-12);
}

#[test]
fn mitered_corners_keep_the_centerline_area() {
	for degrees in [30.0f32, 150.0] {
		let mut graph = WallGraph::new();
		let direction = Vec2::from_angle(degrees.to_radians());
		graph.add_wall(Vec2::ZERO, Vec2::new(3.0, 0.0)).unwrap();
		graph.add_wall(Vec2::ZERO, direction * 3.0).unwrap();

		let footprint = resolve(&format!("corner_{degrees}"), &graph, &FINE);
		assert!(!has_bevel(&footprint), "{degrees} degrees");
		// A miter moves as much material past the node on one face as it removes on the other.
		assert_close(area(&footprint), 0.2 * 6.0, 1e-5);
	}
}

#[test]
fn sharp_corners_bevel_within_the_miter_limit() {
	let mut graph = WallGraph::new();
	graph.add_wall(Vec2::ZERO, Vec2::new(3.0, 0.0)).unwrap();
	graph
		.add_wall(Vec2::ZERO, Vec2::from_angle(20f32.to_radians()) * 3.0)
		.unwrap();

	let footprint = resolve("corner_20", &graph, &FINE);
	assert!(has_bevel(&footprint));
	assert!(area(&footprint) < 0.2 * 6.0);
	let reach = footprint
		.vertices
		.iter()
		.filter(|vertex| vertex.x < 0.0)
		.map(|vertex| vertex.length())
		.fold(0.0, f64::max);
	assert!(reach <= FINE.miter_limit * 0.1);
}

#[test]
fn a_five_degree_corner_does_not_grow_a_long_outside_miter() {
	let mut graph = WallGraph::new();
	graph.add_wall(Vec2::ZERO, Vec2::new(30.0, 0.0)).unwrap();
	graph
		.add_wall(Vec2::ZERO, Vec2::from_angle(5f32.to_radians()) * 30.0)
		.unwrap();

	let footprint = resolve("corner_5", &graph, &FINE);
	assert!(has_bevel(&footprint));
	let outside_reach = footprint
		.vertices
		.iter()
		.filter(|vertex| vertex.x < 0.0)
		.map(|vertex| vertex.length())
		.fold(0.0, f64::max);
	assert!(outside_reach <= FINE.miter_limit * 0.1);
}

#[test]
fn a_mixed_thickness_t_gets_one_core_under_the_through_wall() {
	let mut graph = WallGraph::new();
	graph
		.add_wall(Vec2::new(-2.0, 0.0), Vec2::new(2.0, 0.0))
		.unwrap();
	graph
		.add_wall_with_dimensions(Vec2::ZERO, Vec2::new(0.0, 3.0), thickness(0.1))
		.unwrap();

	let footprint = resolve("t_mixed", &graph, &FINE);
	assert_eq!(cores(&footprint), 1);
	assert_close(area(&footprint), 0.2 * 4.0 + 0.1 * (3.0 - 0.1), 1e-12);
}

#[test]
fn a_crossing_counts_its_shared_square_once() {
	let mut graph = WallGraph::new();
	graph
		.add_wall(Vec2::new(-2.0, 0.0), Vec2::new(2.0, 0.0))
		.unwrap();
	graph
		.add_wall(Vec2::new(0.0, -2.0), Vec2::new(0.0, 2.0))
		.unwrap();

	let footprint = resolve("crossing", &graph, &FINE);
	assert_eq!(cores(&footprint), 1);
	assert_close(area(&footprint), 0.8 + 0.8 - 0.04, 1e-12);
}

#[test]
fn collinear_walls_of_different_thickness_meet_without_a_core() {
	let mut graph = WallGraph::new();
	graph.add_wall(Vec2::new(-2.0, 0.0), Vec2::ZERO).unwrap();
	graph
		.add_wall_with_dimensions(Vec2::ZERO, Vec2::new(2.0, 0.0), thickness(0.3))
		.unwrap();

	let footprint = resolve("collinear_step", &graph, &FINE);
	assert_eq!(cores(&footprint), 0);
	assert_close(area(&footprint), 0.4 + 0.6, 1e-12);
	// The wider end is split where the narrower end's corners touch it.
	let wide = footprint
		.cells
		.iter()
		.max_by(|a, b| {
			polygon_area(&footprint.vertices, &a.boundary)
				.total_cmp(&polygon_area(&footprint.vertices, &b.boundary))
		})
		.unwrap();
	assert_eq!(wide.boundary.len(), 6);
}

#[test]
fn nearly_straight_walls_of_different_thickness_step_instead_of_reaching_far() {
	let mut graph = WallGraph::new();
	graph.add_wall(Vec2::new(-2.0, 0.0), Vec2::ZERO).unwrap();
	graph
		.add_wall_with_dimensions(Vec2::ZERO, Vec2::new(2.0, 0.02), thickness(0.3))
		.unwrap();

	let footprint = resolve("near_straight_step", &graph, &FINE);
	assert_close(area(&footprint), 0.4 + 0.3 * 2.0001, 1e-3);
	assert!(footprint.vertices.iter().all(|vertex| vertex.x.abs() < 2.1));
}

#[test]
fn arcs_of_both_turns_cover_their_annular_sectors() {
	for (name, sweep) in [("arc_ccw", PI), ("arc_cw", -PI), ("arc_major", -1.5 * PI)] {
		let mut graph = WallGraph::new();
		graph
			.add_arc(Vec2::new(2.0, 0.0), Vec2::from_angle(sweep) * 2.0, sweep)
			.unwrap();

		let footprint = resolve(name, &graph, &FINE);
		// An annular sector's area is its sweep times centerline radius times thickness.
		assert_close(area(&footprint), sweep.abs() as f64 * 2.0 * 0.2, 1e-4);
	}
}

#[test]
fn straight_and_curved_walls_join_in_both_combinations() {
	let mut graph = WallGraph::new();
	graph.add_wall(Vec2::new(-2.0, 0.0), Vec2::ZERO).unwrap();
	graph
		.add_arc(Vec2::ZERO, Vec2::new(2.0, 2.0), FRAC_PI_2)
		.unwrap();
	graph
		.add_arc_with_dimensions(
			Vec2::new(2.0, 2.0),
			Vec2::new(4.0, 0.0),
			-FRAC_PI_2,
			thickness(0.3),
		)
		.unwrap();

	let footprint = resolve("arc_joins", &graph, &FINE);
	// The line continues smoothly into the first arc, so only the arc-to-arc corner needs a core.
	assert_eq!(cores(&footprint), 1);
}

#[test]
fn openings_split_a_wall_into_stretches() {
	let mut graph = WallGraph::new();
	let wall = graph.add_wall(Vec2::ZERO, Vec2::new(4.0, 0.0)).unwrap()[0];
	graph
		.add_opening(
			wall,
			OpeningSpec {
				center_distance: 2.0,
				width: 1.0,
				bottom: 0.0,
				height: 2.1,
			},
		)
		.unwrap();

	let footprint = resolve("door", &graph, &FINE);
	let spans: Vec<usize> = footprint
		.cells
		.iter()
		.map(|cell| match &cell.kind {
			CellKind::Body { openings, .. } => openings.len(),
			CellKind::Core { .. } => unreachable!(),
		})
		.collect();
	assert_eq!(spans, vec![0, 1, 0]);
	assert_close(area(&footprint), 0.8, 1e-12);
}

#[test]
fn a_tight_arc_is_rejected() {
	let mut graph = WallGraph::new();
	let arc = graph
		.add_arc(Vec2::new(0.1, 0.0), Vec2::new(-0.1, 0.0), PI)
		.unwrap()[0];

	assert_eq!(
		try_resolve(&graph, &FINE).err(),
		Some(MeshError::ArcTooTight { wall: arc })
	);
}

#[test]
fn junctions_consuming_a_short_wall_are_rejected() {
	let mut graph = WallGraph::new();
	graph.add_wall(Vec2::ZERO, Vec2::new(0.5, 0.0)).unwrap();
	graph
		.add_wall(Vec2::ZERO, Vec2::from_angle(10f32.to_radians()) * 0.5)
		.unwrap();

	assert!(matches!(
		try_resolve(&graph, &FINE),
		Err(MeshError::JunctionTrimsOverlap { .. })
	));
}

#[test]
fn an_opening_inside_a_resolved_miter_is_rejected() {
	let mut graph = WallGraph::new();
	let wall = graph.add_wall(Vec2::ZERO, Vec2::new(3.0, 0.0)).unwrap()[0];
	graph
		.add_wall(Vec2::ZERO, Vec2::from_angle(45f32.to_radians()) * 3.0)
		.unwrap();
	// The graph's swept-centerline clearance accepts this; the inside miter reaches 0.24 m.
	let opening = graph
		.add_opening(
			wall,
			OpeningSpec {
				center_distance: 0.47,
				width: 0.5,
				bottom: 0.0,
				height: 2.0,
			},
		)
		.unwrap();

	assert_eq!(
		try_resolve(&graph, &FINE).err(),
		Some(MeshError::OpeningTooCloseToJunction { opening })
	);
}

#[test]
fn unconnected_overlapping_walls_are_rejected() {
	let mut graph = WallGraph::new();
	let first = graph.add_wall(Vec2::ZERO, Vec2::new(3.0, 0.0)).unwrap()[0];
	let second = graph
		.add_wall(Vec2::new(0.0, 0.15), Vec2::new(3.0, 0.15))
		.unwrap()[0];

	let Err(MeshError::FootprintOverlap {
		first: a,
		second: b,
	}) = try_resolve(&graph, &FINE)
	else {
		panic!("expected an overlap");
	};
	let mut sources = [a, b];
	sources.sort_by_key(|source| format!("{source:?}"));
	let mut expected = [SurfaceSource::Wall(first), SurfaceSource::Wall(second)];
	expected.sort_by_key(|source| format!("{source:?}"));
	assert_eq!(sources, expected);
}

#[test]
fn resolution_is_deterministic() {
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

	let first = resolve("determinism", &graph, &FINE);
	let second = try_resolve(&graph.clone(), &FINE).unwrap();
	assert_eq!(first.vertices, second.vertices);
	let boundaries = |footprint: &Footprint| -> Vec<Vec<usize>> {
		footprint
			.cells
			.iter()
			.map(|cell| cell.boundary.clone())
			.collect()
	};
	assert_eq!(boundaries(&first), boundaries(&second));
}
