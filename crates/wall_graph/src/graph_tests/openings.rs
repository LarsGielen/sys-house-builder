use std::f32::consts::PI;

use glam::Vec2;

use super::support::assert_consistent;
use crate::{OpeningSpec, WallDimensions, WallError, WallGraph};

fn spec(center_distance: f64, width: f32, bottom: f32, height: f32) -> OpeningSpec {
	OpeningSpec {
		center_distance,
		width,
		bottom,
		height,
	}
}

#[test]
fn openings_can_be_placed_and_queried_on_lines_and_arcs() {
	let mut graph = WallGraph::new();
	let line = graph.add_wall(Vec2::ZERO, Vec2::new(5.0, 0.0)).unwrap()[0];
	let line_id = graph.add_opening(line, spec(2.0, 1.0, 0.0, 2.0)).unwrap();
	assert_eq!(graph.opening(line_id).unwrap().wall, line);
	assert_eq!(graph.opening_position(line_id), Some(Vec2::new(2.0, 0.0)));

	let arc = graph
		.add_arc(Vec2::new(8.0, 0.0), Vec2::new(6.0, 0.0), PI)
		.unwrap()[0];
	let arc_id = graph
		.add_opening(arc, spec(std::f64::consts::FRAC_PI_2, 0.5, 0.8, 1.0))
		.unwrap();
	assert!(
		graph
			.opening_position(arc_id)
			.unwrap()
			.distance(Vec2::new(7.0, 1.0))
			< 1e-5
	);
	assert_eq!(graph.openings().count(), 2);
	assert_consistent(&graph);
}

#[test]
fn invalid_openings_and_overlaps_are_rejected_without_mutation() {
	let mut graph = WallGraph::new();
	let wall = graph.add_wall(Vec2::ZERO, Vec2::new(5.0, 0.0)).unwrap()[0];
	let first = graph.add_opening(wall, spec(2.0, 1.0, 0.0, 1.0)).unwrap();
	for invalid in [
		spec(f64::NAN, 1.0, 0.0, 1.0),
		spec(2.0, 0.0, 0.0, 1.0),
		spec(2.0, 0.00001, 0.0, 1.0),
		spec(2.0, 1.0, -0.1, 1.0),
		spec(2.0, 1.0, 2.0, 1.0),
		spec(0.1, 1.0, 0.0, 1.0),
	] {
		assert_eq!(
			graph.add_opening(wall, invalid),
			Err(WallError::InvalidOpening)
		);
	}
	assert_eq!(
		graph.add_opening(wall, spec(2.0, 1.0, 1.01, 1.0)),
		Err(WallError::OpeningOverlap)
	);
	let stacked = graph.add_opening(wall, spec(2.0, 1.0, 1.02, 1.0)).unwrap();
	assert_eq!(
		graph.set_opening(stacked, spec(2.0, 1.0, 1.01, 1.0)),
		Err(WallError::OpeningOverlap)
	);
	assert_eq!(graph.openings().count(), 2);
	assert_eq!(graph.opening(first).unwrap().spec, spec(2.0, 1.0, 0.0, 1.0));
	assert_eq!(graph.opening(stacked).unwrap().spec.bottom, 1.02);
	assert_consistent(&graph);
}

#[test]
fn openings_are_editable_removable_and_keep_ids_through_optimization() {
	let mut graph = WallGraph::new();
	let wall = graph.add_wall(Vec2::ZERO, Vec2::new(5.0, 0.0)).unwrap()[0];
	let removed = graph.add_opening(wall, spec(1.0, 0.5, 0.0, 2.0)).unwrap();
	let kept = graph.add_opening(wall, spec(3.0, 0.5, 0.0, 2.0)).unwrap();
	graph.remove_opening(removed).unwrap();
	assert_eq!(graph.opening(removed), None);
	graph.set_opening(kept, spec(3.5, 0.7, 0.4, 1.5)).unwrap();
	let map = graph.optimize();
	assert_eq!(graph.opening(kept).unwrap().wall, map.wall(wall).unwrap());
	assert_eq!(graph.opening_position(kept), Some(Vec2::new(3.5, 0.0)));
	assert_eq!(
		graph.remove_opening(removed),
		Err(WallError::UnknownOpening)
	);
	graph.remove_wall(map.wall(wall).unwrap()).unwrap();
	assert_eq!(graph.opening(kept), None);
	assert_consistent(&graph);
}

#[test]
fn splits_and_merges_preserve_opening_position() {
	for curved in [false, true] {
		let mut graph = WallGraph::new();
		let initial = if curved {
			graph.add_arc(Vec2::X, -Vec2::X, PI).unwrap()[0]
		} else {
			graph.add_wall(Vec2::ZERO, Vec2::new(4.0, 0.0)).unwrap()[0]
		};
		let center_distance = if curved { 0.5 } else { 1.0 };
		let id = graph
			.add_opening(initial, spec(center_distance, 0.4, 0.0, 2.0))
			.unwrap();
		let before = graph.opening_position(id).unwrap();
		let split = if curved { Vec2::Y } else { Vec2::new(3.0, 0.0) };
		graph.add_wall(split, split + Vec2::Y).unwrap();
		assert!(graph.opening_position(id).unwrap().distance(before) < 1e-4);
		assert_ne!(graph.opening(id).unwrap().wall, initial);
		assert_consistent(&graph);
	}

	let mut graph = WallGraph::new();
	let initial = graph.add_wall(Vec2::ZERO, Vec2::new(2.0, 0.0)).unwrap()[0];
	let id = graph
		.add_opening(initial, spec(1.0, 0.4, 0.0, 2.0))
		.unwrap();
	graph.add_wall(Vec2::ZERO, Vec2::new(-2.0, 0.0)).unwrap();
	assert_eq!(graph.opening_position(id), Some(Vec2::X));
	assert_eq!(
		graph.wall_length(graph.opening(id).unwrap().wall),
		Some(4.0)
	);
	assert_consistent(&graph);
}

#[test]
fn curved_extensions_keep_openings_at_the_same_world_position() {
	for reverse in [false, true] {
		let mut graph = WallGraph::new();
		let first = graph
			.add_arc(Vec2::X, Vec2::Y, std::f32::consts::FRAC_PI_2)
			.unwrap()[0];
		let id = graph.add_opening(first, spec(0.5, 0.3, 0.0, 2.0)).unwrap();
		let before = graph.opening_position(id).unwrap();
		let (from, to, sweep) = if reverse {
			(-Vec2::X, Vec2::Y, -std::f32::consts::FRAC_PI_2)
		} else {
			(Vec2::Y, -Vec2::X, std::f32::consts::FRAC_PI_2)
		};
		graph.add_arc(from, to, sweep).unwrap();
		assert!(graph.opening_position(id).unwrap().distance(before) < 1e-4);
		assert!(
			(graph.wall_length(graph.opening(id).unwrap().wall).unwrap() - std::f64::consts::PI)
				.abs() < 1e-5
		);
		assert_consistent(&graph);
	}
}

#[test]
fn openings_on_both_sides_of_a_split_keep_their_ids_and_positions() {
	let mut graph = WallGraph::new();
	let wall = graph.add_wall(Vec2::ZERO, Vec2::new(8.0, 0.0)).unwrap()[0];
	let left = graph.add_opening(wall, spec(2.0, 0.8, 0.0, 2.0)).unwrap();
	let right = graph.add_opening(wall, spec(6.0, 0.8, 0.0, 2.0)).unwrap();
	graph
		.add_wall(Vec2::new(4.0, 0.0), Vec2::new(4.0, 2.0))
		.unwrap();
	assert_eq!(graph.opening_position(left), Some(Vec2::new(2.0, 0.0)));
	assert_eq!(graph.opening_position(right), Some(Vec2::new(6.0, 0.0)));
	assert_ne!(
		graph.opening(left).unwrap().wall,
		graph.opening(right).unwrap().wall
	);
	assert_consistent(&graph);
}

#[test]
fn junctions_through_or_too_close_to_openings_reject_the_whole_insertion() {
	let mut graph = WallGraph::new();
	let wall = graph.add_wall(Vec2::ZERO, Vec2::new(4.0, 0.0)).unwrap()[0];
	let id = graph.add_opening(wall, spec(2.0, 1.0, 0.0, 2.0)).unwrap();
	for x in [2.0, 1.6] {
		assert_eq!(
			graph.add_wall(Vec2::new(x, 0.0), Vec2::new(x, 1.0)),
			Err(WallError::OpeningTooCloseToJunction)
		);
		assert_eq!(graph.walls().count(), 1);
		assert_eq!(graph.nodes().count(), 2);
		assert_eq!(graph.opening(id).unwrap().wall, wall);
	}
	assert_consistent(&graph);
}

#[test]
fn connected_wall_footprints_and_dimension_edits_enforce_clearance() {
	let mut graph = WallGraph::new();
	let host = graph.add_wall(Vec2::ZERO, Vec2::new(3.0, 0.0)).unwrap()[0];
	let branch = graph.add_wall(Vec2::ZERO, Vec2::new(0.0, 2.0)).unwrap()[0];
	assert_eq!(
		graph.add_opening(host, spec(0.6, 1.0, 0.0, 2.0)),
		Err(WallError::OpeningTooCloseToJunction)
	);
	let id = graph.add_opening(host, spec(0.7, 1.0, 0.0, 2.0)).unwrap();
	let thick = WallDimensions {
		thickness: 0.40,
		height: 2.50,
	};
	assert_eq!(
		graph.set_wall_dimensions(branch, thick),
		Err(WallError::OpeningTooCloseToJunction)
	);
	assert_eq!(
		graph.wall_dimensions(branch),
		Some(WallDimensions::default())
	);
	assert_eq!(graph.opening_position(id), Some(Vec2::new(0.7, 0.0)));
	assert_consistent(&graph);
}

#[test]
fn lowering_wall_height_below_an_opening_is_rejected() {
	let mut graph = WallGraph::new();
	let wall = graph.add_wall(Vec2::ZERO, Vec2::new(3.0, 0.0)).unwrap()[0];
	let id = graph.add_opening(wall, spec(1.0, 0.5, 1.5, 0.8)).unwrap();
	let shorter = WallDimensions {
		thickness: 0.20,
		height: 2.0,
	};
	assert_eq!(
		graph.set_wall_dimensions(wall, shorter),
		Err(WallError::InvalidOpening)
	);
	assert_eq!(graph.wall_dimensions(wall), Some(WallDimensions::default()));
	assert_eq!(graph.opening(id).unwrap().wall, wall);
	assert_consistent(&graph);
}

#[test]
fn a_new_connected_wall_can_be_rejected_by_an_existing_opening() {
	let mut graph = WallGraph::new();
	let host = graph.add_wall(Vec2::ZERO, Vec2::new(3.0, 0.0)).unwrap()[0];
	let id = graph.add_opening(host, spec(0.7, 1.0, 0.0, 2.0)).unwrap();
	let thick = WallDimensions {
		thickness: 0.40,
		height: 2.50,
	};
	assert_eq!(
		graph.add_wall_with_dimensions(Vec2::ZERO, Vec2::new(0.0, 2.0), thick),
		Err(WallError::OpeningTooCloseToJunction)
	);
	assert_eq!(graph.walls().count(), 1);
	assert_eq!(graph.opening(id).unwrap().wall, host);
	assert_consistent(&graph);
}

#[test]
fn an_angled_and_a_curved_junction_use_connected_wall_thickness() {
	let mut angled = WallGraph::new();
	let host = angled.add_wall(Vec2::ZERO, Vec2::new(3.0, 0.0)).unwrap()[0];
	angled.add_wall(Vec2::ZERO, Vec2::new(2.0, 2.0)).unwrap();
	assert_eq!(
		angled.add_opening(host, spec(0.65, 0.9, 0.0, 2.0)),
		Err(WallError::OpeningTooCloseToJunction)
	);
	angled.add_opening(host, spec(0.85, 0.9, 0.0, 2.0)).unwrap();
	assert_consistent(&angled);

	let mut curved = WallGraph::new();
	let arc = curved.add_arc(Vec2::X, -Vec2::X, PI).unwrap()[0];
	curved.add_wall(Vec2::X, Vec2::new(2.0, 0.0)).unwrap();
	assert_eq!(
		curved.add_opening(arc, spec(0.1, 0.1, 0.0, 2.0)),
		Err(WallError::OpeningTooCloseToJunction)
	);
	curved.add_opening(arc, spec(0.3, 0.1, 0.0, 2.0)).unwrap();
	assert_consistent(&curved);
}
