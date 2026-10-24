use super::*;
use crate::edit::OpeningProposal;
use crate::edit::{WallProposal, propose_opening, propose_wall};
use crate::opening_category::OpeningCategory;
use bevy::prelude::Vec2;
use wall_graph::WallDimensions;

fn spec(center_distance: f64) -> OpeningSpec {
	OpeningSpec {
		center_distance,
		width: 0.9,
		bottom: 0.0,
		height: 2.1,
	}
}

const NO_GAP: f32 = 0.0;

fn graph_with_wall_and_openings(length: f32, centers: &[f64]) -> (WallGraph, Wall) {
	let mut graph = WallGraph::new();
	let wall = graph.add_wall(Vec2::ZERO, Vec2::new(length, 0.0)).unwrap()[0];
	for &center_distance in centers {
		graph.add_opening(wall, spec(center_distance)).unwrap();
	}
	(graph, wall)
}

/// Snaps a default opening proposed at `center_distance`. A huge grid spacing keeps grid
/// candidates out of range unless a test is about the grid.
fn snapped_center(graph: &WallGraph, wall: Wall, center_distance: f64, grid_spacing: f32) -> f64 {
	snapped_with(graph, wall, center_distance, grid_spacing, NO_GAP, None)
}

fn snapped_with(
	graph: &WallGraph,
	wall: Wall,
	center_distance: f64,
	grid_spacing: f32,
	gap: f32,
	moving: Option<OpeningId>,
) -> f64 {
	let options = OpeningSnapOptions {
		grid_spacing,
		gap,
		moving,
	};
	snap_opening(graph, wall, spec(center_distance), options).center_distance
}

#[test]
fn an_empty_wall_snaps_to_its_centre() {
	let (graph, wall) = graph_with_wall_and_openings(4.0, &[]);

	assert_eq!(snapped_center(&graph, wall, 2.08, 100.0), 2.0);
}

#[test]
fn opening_snaps_flush_to_either_wall_end() {
	let (graph, wall) = graph_with_wall_and_openings(4.0, &[]);

	// The width is an f32, so its half is not exactly 0.45.
	assert!((snapped_center(&graph, wall, 0.5, 100.0) - 0.45).abs() < 1e-6);
	assert!((snapped_center(&graph, wall, 3.5, 100.0) - 3.55).abs() < 1e-6);
}

#[test]
fn opening_snaps_to_the_middle_between_two_openings() {
	let (graph, wall) = graph_with_wall_and_openings(6.0, &[1.0, 5.0]);

	assert_eq!(snapped_center(&graph, wall, 3.1, 100.0), 3.0);
}

#[test]
fn a_span_too_narrow_for_the_opening_is_not_a_snap_target() {
	// The gap between the openings is 0.5 m, narrower than the 0.9 m opening.
	let (graph, wall) = graph_with_wall_and_openings(6.0, &[2.0, 3.4]);

	assert_eq!(snapped_center(&graph, wall, 2.7, 100.0), 2.7);
}

#[test]
fn positions_out_of_range_of_every_candidate_stay_free() {
	let (graph, wall) = graph_with_wall_and_openings(6.0, &[1.0, 5.0]);

	assert_eq!(snapped_center(&graph, wall, 2.5, 100.0), 2.5);
}

#[test]
fn grid_distance_snaps_when_nothing_structural_is_near() {
	let (graph, wall) = graph_with_wall_and_openings(6.0, &[1.0, 5.0]);

	assert_eq!(snapped_center(&graph, wall, 2.45, 0.5), 2.5);
}

#[test]
fn structural_candidates_beat_a_nearer_grid_distance() {
	let (graph, wall) = graph_with_wall_and_openings(4.0, &[]);

	assert_eq!(snapped_center(&graph, wall, 2.08, 0.1), 2.0);
}

#[test]
fn edge_snap_lands_on_the_nearest_valid_position_beside_a_junction() {
	let mut document = crate::edit::EditCandidate::default();
	for (start, end) in [
		(Vec2::ZERO, Vec2::new(4.0, 0.0)),
		(Vec2::ZERO, Vec2::new(0.0, 4.0)),
	] {
		let proposal = WallProposal {
			start,
			end,
			dimensions: WallDimensions::default(),
		};
		document = propose_wall(&document, &proposal).unwrap();
	}
	let graph = document.graph();
	let wall = graph
		.walls()
		.find(|&wall| graph.node_position(wall.destination()) == Some(Vec2::new(4.0, 0.0)))
		.unwrap();
	let valid_at = |center_distance: f64| {
		let proposal = OpeningProposal {
			wall,
			spec: spec(center_distance),
			category: OpeningCategory::default(),
		};
		propose_opening(&document, &proposal).is_ok()
	};
	let flush = f64::from(spec(0.0).width) / 2.0;
	assert!(!valid_at(flush));

	let snapped = snapped_center(graph, wall, flush + 0.05, 100.0);

	assert!(snapped > flush);
	assert!(valid_at(snapped));
	// Any closer to the junction and the opening is rejected.
	assert!(!valid_at(snapped - 0.005));
}

#[test]
fn opening_snaps_to_another_openings_centre() {
	let (graph, wall) = graph_with_wall_and_openings(8.0, &[2.0]);

	assert_eq!(snapped_center(&graph, wall, 2.08, 100.0), 2.0);
}

#[test]
fn opening_snaps_beside_a_neighbour_keeping_the_minimum_gap() {
	let (graph, wall) = graph_with_wall_and_openings(8.0, &[2.0]);
	let neighbour_end = 2.0 + f64::from(spec(2.0).width) / 2.0;
	let expected = neighbour_end + MIN_OPENING_GAP + f64::from(spec(2.0).width) / 2.0;

	// With no configured gap the graph's minimum still applies.
	let snapped = snapped_center(&graph, wall, expected + 0.04, 100.0);

	assert!((snapped - expected).abs() < 1e-9);
}

#[test]
fn a_configured_gap_moves_the_snap_away_from_the_neighbour() {
	let (graph, wall) = graph_with_wall_and_openings(8.0, &[2.0]);
	let half_width = f64::from(spec(2.0).width) / 2.0;
	let before = 2.0 - half_width - 0.1 - half_width;

	let snapped = snapped_with(&graph, wall, before - 0.04, 100.0, 0.1, None);

	assert!((snapped - before).abs() < 1e-6);
}

#[test]
fn a_moving_opening_does_not_snap_to_its_own_old_position() {
	let (graph, wall) = graph_with_wall_and_openings(8.0, &[2.0]);
	let (moving, _) = graph.openings().next().unwrap();

	assert_eq!(
		snapped_with(&graph, wall, 2.08, 100.0, NO_GAP, Some(moving)),
		2.08
	);
}
