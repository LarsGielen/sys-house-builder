use super::*;

const PIXELS_PER_METRE: f32 = 100.0;

fn project(graph_position: Vec2) -> Option<Vec2> {
	Some(graph_position * PIXELS_PER_METRE)
}

fn pointer_at(graph_position: Vec2) -> PointerOnGround {
	PointerOnGround {
		viewport_position: graph_position * PIXELS_PER_METRE,
		graph_position,
	}
}

fn settings_with_spacing(spacing: f32) -> SnapSettings {
	let grid_spacing_index = GRID_SPACINGS_METRES
		.iter()
		.position(|&candidate| candidate == spacing)
		.expect("spacing is one of the supported grid spacings");
	SnapSettings {
		grid_spacing_index,
		..default()
	}
}

fn graph_with_one_wall() -> WallGraph {
	let mut graph = WallGraph::new();
	graph
		.add_wall(Vec2::new(0.0, 0.0), Vec2::new(4.0, 0.0))
		.unwrap();
	graph
}

#[test]
fn junction_beats_a_nearby_wall_and_grid_point() {
	let graph = graph_with_one_wall();
	// Near the wall end: the wall point and a grid point are also within the radius.
	let target = choose_snap_target(
		&graph,
		pointer_at(Vec2::new(3.95, 0.04)),
		&settings_with_spacing(0.5),
		None,
		project,
	);

	assert!(matches!(target.source, SnapSource::Junction(_)));
	assert_eq!(target.position, Vec2::new(4.0, 0.0));
}

#[test]
fn wall_beats_a_grid_point() {
	let graph = graph_with_one_wall();
	let target = choose_snap_target(
		&graph,
		pointer_at(Vec2::new(2.0, 0.05)),
		&settings_with_spacing(0.5),
		None,
		project,
	);

	assert!(matches!(target.source, SnapSource::Wall(_)));
	assert!(target.position.distance(Vec2::new(2.0, 0.0)) < 1e-5);
}

#[test]
fn grid_is_used_when_nothing_else_is_selectable() {
	let target = choose_snap_target(
		&graph_with_one_wall(),
		pointer_at(Vec2::new(1.03, 3.0)),
		&settings_with_spacing(0.5),
		None,
		project,
	);

	assert_eq!(target.source, SnapSource::Grid);
	assert_eq!(target.position, Vec2::new(1.0, 3.0));
}

#[test]
fn nearer_junction_wins_within_a_class() {
	let mut graph = WallGraph::new();
	graph
		.add_wall(Vec2::new(0.0, 0.0), Vec2::new(0.1, 0.0))
		.unwrap();
	// Both ends are within the radius, 7 px and 3 px away.
	let target = choose_snap_target(
		&graph,
		pointer_at(Vec2::new(0.07, 0.0)),
		&settings_with_spacing(2.0),
		None,
		project,
	);

	assert!(matches!(target.source, SnapSource::Junction(_)));
	assert_eq!(target.position, Vec2::new(0.1, 0.0));
}

#[test]
fn nothing_within_the_radius_gives_the_unsnapped_point() {
	let pointer = pointer_at(Vec2::new(1.3, 3.3));
	let target = choose_snap_target(
		&graph_with_one_wall(),
		pointer,
		&settings_with_spacing(2.0),
		None,
		project,
	);

	assert_eq!(target, SnapTarget::free(pointer.graph_position));
}

#[test]
fn grid_spacing_changes_only_the_grid_candidate() {
	let graph = graph_with_one_wall();
	let near_junction = pointer_at(Vec2::new(3.95, 0.04));
	let near_grid_only = pointer_at(Vec2::new(1.1, 3.0));

	let junction_with_fine_grid = choose_snap_target(
		&graph,
		near_junction,
		&settings_with_spacing(0.1),
		None,
		project,
	);
	let junction_with_coarse_grid = choose_snap_target(
		&graph,
		near_junction,
		&settings_with_spacing(2.0),
		None,
		project,
	);
	assert_eq!(junction_with_fine_grid, junction_with_coarse_grid);

	let fine_grid = choose_snap_target(
		&graph,
		near_grid_only,
		&settings_with_spacing(0.1),
		None,
		project,
	);
	let coarse_grid = choose_snap_target(
		&graph,
		near_grid_only,
		&settings_with_spacing(0.5),
		None,
		project,
	);
	assert_eq!(fine_grid.source, SnapSource::Grid);
	assert_eq!(coarse_grid.source, SnapSource::Grid);
	assert!(fine_grid.position.distance(Vec2::new(1.1, 3.0)) < 1e-5);
	assert!(coarse_grid.position.distance(Vec2::new(1.0, 3.0)) < 1e-5);
}

#[test]
fn snap_starts_at_the_start_radius_but_holds_until_the_end_radius() {
	let graph = graph_with_one_wall();
	let settings = settings_with_spacing(2.0);
	let junction = graph.nodes().find(|(_, position)| *position == Vec2::ZERO);
	let junction_source = SnapSource::Junction(junction.unwrap().0);

	// 15 px from the junction: outside the 12 px start radius, inside the 20 px end radius.
	let pointer = pointer_at(Vec2::new(-0.15, 0.0));
	let not_yet_snapped = choose_snap_target(&graph, pointer, &settings, None, project);
	assert_eq!(not_yet_snapped, SnapTarget::free(pointer.graph_position));

	let already_snapped =
		choose_snap_target(&graph, pointer, &settings, Some(junction_source), project);
	assert_eq!(already_snapped.source, junction_source);

	// 25 px away: past the end radius, so the snap lets go.
	let far_pointer = pointer_at(Vec2::new(-0.25, 0.0));
	let released = choose_snap_target(
		&graph,
		far_pointer,
		&settings,
		Some(junction_source),
		project,
	);
	assert_eq!(released, SnapTarget::free(far_pointer.graph_position));
}

#[test]
fn holding_a_lower_priority_snap_does_not_block_a_junction() {
	let graph = graph_with_one_wall();
	let settings = settings_with_spacing(2.0);
	let wall_source = graph
		.walls()
		.next()
		.map(SnapSource::Wall)
		.expect("the graph has one wall");

	// Snapped to the wall, the pointer comes within the start radius of its end junction.
	let target = choose_snap_target(
		&graph,
		pointer_at(Vec2::new(3.95, 0.1)),
		&settings,
		Some(wall_source),
		project,
	);

	assert!(matches!(target.source, SnapSource::Junction(_)));
}

#[test]
fn grid_stays_selectable_up_to_the_end_radius() {
	let settings = settings_with_spacing(1.0);
	// 15 px from the grid point (1, 3): too far to start, close enough to hold.
	let pointer = pointer_at(Vec2::new(1.15, 3.0));

	let not_yet_snapped = choose_snap_target(&WallGraph::new(), pointer, &settings, None, project);
	assert_eq!(not_yet_snapped.source, SnapSource::Free);

	let held = choose_snap_target(
		&WallGraph::new(),
		pointer,
		&settings,
		Some(SnapSource::Grid),
		project,
	);
	assert_eq!(held.source, SnapSource::Grid);
	assert_eq!(held.position, Vec2::new(1.0, 3.0));
}

#[test]
fn grid_spacing_stays_within_the_supported_list() {
	let mut settings = SnapSettings::default();
	for _ in 0..10 {
		settings.step_grid_spacing(1);
	}
	assert_eq!(settings.grid_spacing(), 2.0);
	for _ in 0..10 {
		settings.step_grid_spacing(-1);
	}
	assert_eq!(settings.grid_spacing(), 0.1);
}
#[test]
fn equidistant_junctions_prefer_the_previous_target_then_the_lowest_id() {
	let mut graph = WallGraph::new();
	graph
		.add_wall(Vec2::new(-0.05, 0.0), Vec2::new(0.05, 0.0))
		.unwrap();
	let mut nodes: Vec<_> = graph.nodes().map(|(node, _)| node).collect();
	nodes.sort();
	let pointer = pointer_at(Vec2::ZERO);
	let settings = settings_with_spacing(1.0);
	let target = choose_snap_target(&graph, pointer, &settings, None, project);
	assert_eq!(target.source, SnapSource::Junction(nodes[0]));
	let previous = SnapSource::Junction(nodes[1]);
	let held = choose_snap_target(&graph, pointer, &settings, Some(previous), project);
	assert_eq!(held.source, previous);
}
#[test]
fn different_arcs_with_the_same_endpoints_have_a_stable_order() {
	let mut graph = WallGraph::new();
	let first = graph
		.add_arc(Vec2::ZERO, Vec2::X, std::f32::consts::FRAC_PI_2)
		.unwrap()[0];
	let second = graph
		.add_arc(Vec2::ZERO, Vec2::X, -std::f32::consts::FRAC_PI_2)
		.unwrap()[0];
	let ordering = stable_source_order(&graph, SnapSource::Wall(first), SnapSource::Wall(second));
	assert_ne!(ordering, std::cmp::Ordering::Equal);
	assert_eq!(
		ordering.reverse(),
		stable_source_order(&graph, SnapSource::Wall(second), SnapSource::Wall(first))
	);
}

#[test]
fn invalid_opening_gap_settings_are_rejected_without_mutation() {
	let mut settings = SnapSettings::default();
	settings.set_opening_gap(0.15).unwrap();
	for gap in [-1.0, f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
		assert_eq!(settings.set_opening_gap(gap), Err(InvalidOpeningGap));
		assert_eq!(settings.opening_gap(), 0.15);
	}
}
