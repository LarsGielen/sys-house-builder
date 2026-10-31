//! The smooth joins the editor's tangent tools rely on, and the ones that must stay invalid.
use std::f32::consts::{FRAC_PI_2, PI};

use glam::Vec2;

use crate::{WallError, WallGraph, arc_tangent_at_both, arc_tangent_at_start};

#[test]
fn an_arc_continues_a_wall_end_smoothly() {
	let mut graph = WallGraph::new();
	graph.add_wall(Vec2::ZERO, Vec2::new(2.0, 0.0)).unwrap();
	let end = Vec2::new(4.0, 2.0);
	let sweep = arc_tangent_at_start(Vec2::new(2.0, 0.0), Vec2::X, end).unwrap();
	graph
		.add_arc(Vec2::new(2.0, 0.0), end, sweep)
		.expect("one-connection tangent arc");
	assert_eq!(graph.walls().count(), 2);
}

#[test]
fn an_arc_joins_two_wall_ends_smoothly() {
	let start = Vec2::new(2.0, 0.0);
	let end = Vec2::new(6.0, 2.0);
	// The first wall fixes the arc, and the far wall continues it from where it arrives.
	let sweep = arc_tangent_at_start(start, Vec2::X, end).unwrap();
	let arriving = Vec2::from_angle((end - start).to_angle() + sweep / 2.0);
	assert_eq!(
		arc_tangent_at_both(start, Vec2::X, end, arriving),
		Ok(sweep)
	);

	let mut graph = WallGraph::new();
	graph.add_wall(Vec2::ZERO, start).unwrap();
	graph.add_wall(end, end + arriving * 2.0).unwrap();
	graph
		.add_arc(start, end, sweep)
		.expect("two-connection tangent arc");
	assert_eq!(graph.walls().count(), 3);
}

#[test]
fn a_line_between_two_arcs_is_a_valid_smooth_chain() {
	let mut graph = WallGraph::new();
	graph
		.add_arc(Vec2::new(1.0, 0.0), Vec2::new(-1.0, 0.0), PI)
		.unwrap();
	// A wall leaving the arc's end along its tangent, continuing it smoothly.
	graph
		.add_wall(Vec2::new(-1.0, 0.0), Vec2::new(-1.0, -3.0))
		.expect("tangent continuation of an arc end");
	assert_eq!(graph.walls().count(), 2);
}

#[test]
fn a_wall_that_only_touches_an_arc_in_its_middle_is_rejected() {
	let mut graph = WallGraph::new();
	graph
		.add_arc(Vec2::new(1.0, 0.0), Vec2::new(-1.0, 0.0), PI)
		.unwrap();
	// Tangent to the unit circle at its top.
	assert_eq!(
		graph.add_wall(Vec2::new(-1.0, 1.0), Vec2::new(1.0, 1.0)),
		Err(WallError::TangentialContact)
	);
	assert_eq!(graph.walls().count(), 1);
}

#[test]
fn a_wall_leaving_an_end_back_along_the_same_way_is_rejected() {
	let mut graph = WallGraph::new();
	graph.add_wall(Vec2::ZERO, Vec2::new(2.0, 0.0)).unwrap();
	// Straight on top of the existing wall.
	assert_eq!(
		graph.add_wall(Vec2::new(2.0, 0.0), Vec2::new(1.0, 0.0)),
		Err(WallError::Overlapping)
	);
	// An arc that leaves its start along the existing wall's own direction back into it.
	let sweep = arc_tangent_at_start(Vec2::new(2.0, 0.0), -Vec2::X, Vec2::new(0.0, 2.0)).unwrap();
	assert!(
		graph
			.add_arc(Vec2::new(2.0, 0.0), Vec2::new(0.0, 2.0), sweep)
			.is_err()
	);
	assert_eq!(graph.walls().count(), 1);
}

#[test]
fn two_arcs_leaving_a_node_in_the_same_direction_are_rejected() {
	let mut graph = WallGraph::new();
	graph
		.add_arc(Vec2::ZERO, Vec2::new(2.0, 0.0), FRAC_PI_2)
		.unwrap();
	// Same departure tangent, different curvature.
	assert_eq!(
		graph.add_arc(Vec2::ZERO, Vec2::new(4.0, 0.0), FRAC_PI_2),
		Err(WallError::TangentialContact)
	);
}
