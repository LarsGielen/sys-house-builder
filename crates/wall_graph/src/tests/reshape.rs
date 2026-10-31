use glam::Vec2;

use crate::{OpeningSpec, Wall, WallError, WallGraph, WallNodeId, arc_tangent_at_start};

fn node_at(graph: &WallGraph, position: Vec2) -> WallNodeId {
	graph
		.nodes()
		.find(|(_, node)| node.distance(position) < 1e-4)
		.map(|(id, _)| id)
		.expect("a node at the position")
}

fn wall_between(graph: &WallGraph, first: Vec2, second: Vec2) -> Wall {
	let (first, second) = (node_at(graph, first), node_at(graph, second));
	graph
		.walls()
		.find(|wall| {
			[wall.origin(), wall.destination()].contains(&first)
				&& [wall.origin(), wall.destination()].contains(&second)
		})
		.expect("a wall between the positions")
}

/// Everything that identifies the graph's contents, including identifiers, to compare
/// before and after a rejected operation.
fn fingerprint(graph: &WallGraph) -> String {
	let mut nodes: Vec<_> = graph.nodes().map(|node| format!("{node:?}")).collect();
	let mut walls: Vec<_> = graph
		.walls()
		.map(|wall| format!("{wall:?} {:?}", graph.wall_curve(wall)))
		.collect();
	let mut openings: Vec<_> = graph
		.openings()
		.map(|opening| format!("{opening:?}"))
		.collect();
	nodes.sort();
	walls.sort();
	openings.sort();
	format!(
		"{nodes:?}{walls:?}{openings:?}{} {} {}",
		graph.next_node_id, graph.next_edge_id, graph.next_opening_id
	)
}

fn door(center_distance: f64) -> OpeningSpec {
	OpeningSpec {
		center_distance,
		width: 0.9,
		bottom: 0.0,
		height: 2.1,
	}
}

/// A wall along +X, then an arc that continues it smoothly up and to the right.
fn line_then_tangent_arc() -> WallGraph {
	let mut graph = WallGraph::new();
	graph.add_wall(Vec2::ZERO, Vec2::new(2.0, 0.0)).unwrap();
	let end = Vec2::new(4.0, 2.0);
	let sweep = arc_tangent_at_start(Vec2::new(2.0, 0.0), Vec2::X, end).unwrap();
	graph.add_arc(Vec2::new(2.0, 0.0), end, sweep).unwrap();
	graph
}

#[test]
fn smooth_joins_are_inferred_from_geometry() {
	let graph = line_then_tangent_arc();
	let joins = graph.smooth_joins();
	assert_eq!(joins.len(), 1);
	assert_eq!(joins[0].node, node_at(&graph, Vec2::new(2.0, 0.0)));

	let mut corner = WallGraph::new();
	corner.add_wall(Vec2::ZERO, Vec2::new(2.0, 0.0)).unwrap();
	corner
		.add_wall(Vec2::new(2.0, 0.0), Vec2::new(2.0, 2.0))
		.unwrap();
	assert!(corner.smooth_joins().is_empty());
}

#[test]
fn moving_a_free_end_keeps_everything_else_and_the_node_ids() {
	let mut graph = WallGraph::new();
	graph.add_wall(Vec2::ZERO, Vec2::new(4.0, 0.0)).unwrap();
	graph
		.add_wall(Vec2::new(4.0, 0.0), Vec2::new(4.0, 3.0))
		.unwrap();
	let corner = node_at(&graph, Vec2::new(4.0, 0.0));
	let far = node_at(&graph, Vec2::new(4.0, 3.0));
	let untouched = wall_between(&graph, Vec2::ZERO, Vec2::new(4.0, 0.0));

	let report = graph.move_nodes(&[(far, Vec2::new(5.0, 3.0))]).unwrap();

	assert_eq!(graph.node_position(far), Some(Vec2::new(5.0, 3.0)));
	assert_eq!(graph.node_position(corner), Some(Vec2::new(4.0, 0.0)));
	assert_eq!(report.rebuilt_walls.len(), 1);
	assert!(
		graph.wall_length(untouched).is_some(),
		"an unaffected handle stays valid"
	);
	assert_eq!(graph.walls().count(), 2);
}

#[test]
fn moving_a_shared_junction_stretches_every_wall_that_ends_there() {
	let mut graph = WallGraph::new();
	for end in [
		Vec2::new(4.0, 0.0),
		Vec2::new(0.0, 4.0),
		Vec2::new(-4.0, 0.0),
	] {
		graph.add_wall(Vec2::ZERO, end).unwrap();
	}
	let junction = node_at(&graph, Vec2::ZERO);
	let report = graph
		.move_nodes(&[(junction, Vec2::new(0.5, 0.5))])
		.unwrap();
	assert_eq!(report.rebuilt_walls.len(), 3);
	assert_eq!(graph.node_position(junction), Some(Vec2::new(0.5, 0.5)));
	assert_eq!(graph.node_walls(junction).unwrap().len(), 3);
}

#[test]
fn moving_both_ends_translates_a_wall_and_its_openings_rigidly() {
	let mut graph = WallGraph::new();
	let wall = graph.add_wall(Vec2::ZERO, Vec2::new(5.0, 0.0)).unwrap()[0];
	let opening = graph.add_opening(wall, door(2.0)).unwrap();
	let (start, end) = (wall.origin(), wall.destination());

	let report = graph
		.move_nodes(&[(start, Vec2::new(1.0, 3.0)), (end, Vec2::new(6.0, 3.0))])
		.unwrap();

	let moved = graph.opening(opening).unwrap();
	assert_eq!(report.rebuilt_walls[0].1, moved.wall);
	assert_eq!(moved.spec, door(2.0));
	assert!(
		graph
			.opening_position(opening)
			.unwrap()
			.distance(Vec2::new(3.0, 3.0))
			< 1e-5
	);
}

#[test]
fn a_stretched_wall_keeps_openings_at_the_same_fraction_of_its_length() {
	let mut graph = WallGraph::new();
	let wall = graph.add_wall(Vec2::ZERO, Vec2::new(4.0, 0.0)).unwrap()[0];
	let opening = graph.add_opening(wall, door(1.0)).unwrap();
	graph
		.move_nodes(&[(wall.destination(), Vec2::new(8.0, 0.0))])
		.unwrap();
	assert!((graph.opening(opening).unwrap().spec.center_distance - 2.0).abs() < 1e-9);
}

#[test]
fn a_move_that_leaves_an_opening_without_room_is_rejected_whole() {
	let mut graph = WallGraph::new();
	let wall = graph.add_wall(Vec2::ZERO, Vec2::new(4.0, 0.0)).unwrap()[0];
	graph.add_opening(wall, door(2.0)).unwrap();
	graph
		.add_wall(Vec2::new(4.0, 0.0), Vec2::new(4.0, 3.0))
		.unwrap();
	let before = fingerprint(&graph);

	let result = graph.move_nodes(&[(wall.origin(), Vec2::new(3.5, 0.0))]);

	assert_eq!(result, Err(WallError::InvalidOpening));
	assert_eq!(fingerprint(&graph), before);
}

#[test]
fn a_move_that_would_cross_another_wall_is_rejected_without_changes() {
	let mut graph = WallGraph::new();
	graph.add_wall(Vec2::ZERO, Vec2::new(4.0, 0.0)).unwrap();
	graph
		.add_wall(Vec2::new(0.0, 2.0), Vec2::new(4.0, 2.0))
		.unwrap();
	let top = node_at(&graph, Vec2::new(4.0, 2.0));
	let before = fingerprint(&graph);

	assert_eq!(
		graph.move_nodes(&[(top, Vec2::new(4.0, -2.0))]),
		Err(WallError::CrossesWall)
	);
	assert_eq!(fingerprint(&graph), before);
}

#[test]
fn a_move_onto_another_node_or_an_unknown_node_is_rejected_without_changes() {
	let mut graph = WallGraph::new();
	graph.add_wall(Vec2::ZERO, Vec2::new(4.0, 0.0)).unwrap();
	graph
		.add_wall(Vec2::new(0.0, 2.0), Vec2::new(4.0, 2.0))
		.unwrap();
	let before = fingerprint(&graph);
	let a = node_at(&graph, Vec2::ZERO);
	let b = node_at(&graph, Vec2::new(0.0, 2.0));
	assert_eq!(
		graph.move_nodes(&[(a, graph.node_position(b).unwrap())]),
		Err(WallError::CrossesWall)
	);
	assert_eq!(
		graph.move_nodes(&[(a, Vec2::new(f32::NAN, 0.0))]),
		Err(WallError::InvalidPosition)
	);
	let mut other = WallGraph::new();
	let foreign = other.add_wall(Vec2::ZERO, Vec2::X).unwrap()[0].origin();
	let mut empty = WallGraph::new();
	assert_eq!(
		empty.move_nodes(&[(foreign, Vec2::ONE)]),
		Err(WallError::UnknownNode)
	);
	assert_eq!(fingerprint(&graph), before);
}

#[test]
fn a_move_to_the_same_position_changes_nothing() {
	let mut graph = line_then_tangent_arc();
	let before = fingerprint(&graph);
	let node = node_at(&graph, Vec2::new(4.0, 2.0));
	let report = graph.move_nodes(&[(node, Vec2::new(4.0, 2.0))]).unwrap();
	assert_eq!(report, Default::default());
	assert_eq!(fingerprint(&graph), before);
}

#[test]
fn a_feasible_drag_keeps_the_arc_tangent_to_its_neighbour() {
	let mut graph = line_then_tangent_arc();
	let end = node_at(&graph, Vec2::new(4.0, 2.0));
	let joint = node_at(&graph, Vec2::new(2.0, 0.0));

	let report = graph.move_nodes(&[(end, Vec2::new(5.0, 1.0))]).unwrap();

	assert!(report.lost_smooth_joins.is_empty());
	let joins = graph.smooth_joins();
	assert_eq!(joins.len(), 1);
	assert_eq!(joins[0].node, joint);
}

#[test]
fn dragging_the_junction_of_a_line_and_arc_keeps_the_arc_tangent_to_the_line() {
	let mut graph = line_then_tangent_arc();
	let joint = node_at(&graph, Vec2::new(2.0, 0.0));
	let report = graph.move_nodes(&[(joint, Vec2::new(2.0, 1.0))]).unwrap();
	assert!(report.lost_smooth_joins.is_empty(), "{report:?}");
	assert_eq!(graph.smooth_joins().len(), 1);
}

#[test]
fn an_infeasible_drag_keeps_the_dragged_ends_and_reports_the_lost_join() {
	// line - arc - line, smooth at both joints.
	let mut graph = WallGraph::new();
	let first_end = Vec2::new(2.0, 0.0);
	let arc_end = Vec2::new(6.0, 2.0);
	let sweep = arc_tangent_at_start(first_end, Vec2::X, arc_end).unwrap();
	let arriving = Vec2::from_angle((arc_end - first_end).to_angle() + sweep / 2.0);
	graph.add_wall(Vec2::ZERO, first_end).unwrap();
	graph.add_arc(first_end, arc_end, sweep).unwrap();
	graph.add_wall(arc_end, arc_end + arriving * 2.0).unwrap();
	assert_eq!(graph.smooth_joins().len(), 2);
	let tail = node_at(&graph, arc_end + arriving * 2.0);
	let far_joint = node_at(&graph, arc_end);

	// Swinging the last wall means the arc cannot meet both neighbours any more.
	let target = arc_end + Vec2::new(0.0, 2.0);
	let report = graph.move_nodes(&[(tail, target)]).unwrap();

	assert_eq!(graph.node_position(tail), Some(target));
	assert_eq!(graph.node_position(far_joint), Some(arc_end));
	assert_eq!(report.lost_smooth_joins, vec![far_joint]);
	assert_eq!(graph.smooth_joins().len(), 1);
}

#[test]
fn bending_a_wall_keeps_its_endpoints_and_reports_the_joins_it_breaks() {
	let mut graph = WallGraph::new();
	graph.add_wall(Vec2::ZERO, Vec2::new(4.0, 0.0)).unwrap();
	graph
		.add_wall(Vec2::new(4.0, 0.0), Vec2::new(8.0, 0.0))
		.unwrap();
	// The two collinear walls merged, so there is no join to break yet.
	assert_eq!(graph.walls().count(), 1);

	let wall = graph.walls().next().unwrap();
	let report = graph.bend_wall(wall, 1.0).unwrap();
	assert!(report.lost_smooth_joins.is_empty());
	let bent = report.rebuilt_walls[0].1;
	assert_eq!(graph.node_position(bent.origin()), Some(Vec2::ZERO));
	assert_eq!(
		graph.node_position(bent.destination()),
		Some(Vec2::new(8.0, 0.0))
	);
	assert!(
		graph
			.wall_curve(bent)
			.is_some_and(|curve| matches!(curve, crate::WallCurve::CircularArc { .. }))
	);

	let report = graph.bend_wall(bent, 0.0).unwrap();
	let straight = report.rebuilt_walls[0].1;
	assert!(matches!(
		graph.wall_curve(straight),
		Some(crate::WallCurve::Straight { .. })
	));
}

#[test]
fn bending_next_to_a_smooth_join_reports_it_lost_and_rejects_bad_sweeps() {
	let mut graph = line_then_tangent_arc();
	let arc = wall_between(&graph, Vec2::new(2.0, 0.0), Vec2::new(4.0, 2.0));
	let before = fingerprint(&graph);
	assert_eq!(graph.bend_wall(arc, f32::NAN), Err(WallError::InvalidArc));
	assert_eq!(graph.bend_wall(arc, 7.0), Err(WallError::InvalidArc));
	assert_eq!(fingerprint(&graph), before);

	let report = graph.bend_wall(arc, 2.5).unwrap();
	assert_eq!(
		report.lost_smooth_joins,
		vec![node_at(&graph, Vec2::new(2.0, 0.0))]
	);
}

#[test]
fn a_rebuilt_wall_handle_replaces_the_old_one() {
	let mut graph = WallGraph::new();
	let wall = graph.add_wall(Vec2::ZERO, Vec2::new(4.0, 0.0)).unwrap()[0];
	let report = graph
		.move_nodes(&[(wall.destination(), Vec2::new(4.0, 1.0))])
		.unwrap();
	assert_eq!(graph.wall_length(wall), None);
	assert!(graph.wall_length(report.rebuilt_walls[0].1).is_some());
}
