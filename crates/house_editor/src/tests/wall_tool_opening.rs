use super::*;
use crate::{
	coordinates::graph_to_bevy,
	edit::{WallProposal, propose_wall},
	editor_schedule::EditorSchedulePlugin,
	wall_tool_active::ActiveTool,
};
use wall_graph::{Wall, WallDimensions};

fn house_with_wall(start: Vec2, end: Vec2) -> House {
	let mut house = House::default();
	let proposal = WallProposal {
		start,
		end,
		dimensions: WallDimensions::default(),
	};
	house.commit(propose_wall(house.graph(), &proposal).unwrap());
	house
}

/// A ray from above the floor plan point straight down onto the wall top.
fn ray_down_at(position: Vec2) -> Ray3d {
	Ray3d::new(graph_to_bevy(position.extend(5.0)), Dir3::NEG_Y)
}

fn propose_down_at(house: &House, position: Vec2) -> Option<OpeningProposal> {
	propose_at_ray(
		house.graph(),
		house.mesh(),
		&OpeningSettings::default(),
		ray_down_at(position),
	)
}

#[test]
fn center_distance_is_measured_along_the_wall_handle() {
	let house = house_with_wall(Vec2::ZERO, Vec2::new(4.0, 0.0));

	let proposal = propose_down_at(&house, Vec2::new(1.0, 0.0)).unwrap();

	assert!((proposal.spec.center_distance - 1.0).abs() < 1e-4);
}

#[test]
fn center_distance_follows_a_curved_wall() {
	let mut graph = WallGraph::new();
	let arc = graph
		.add_arc(
			Vec2::new(1.0, 0.0),
			Vec2::new(-1.0, 0.0),
			std::f32::consts::PI,
		)
		.unwrap()[0];
	let mesh = wall_mesh::generate(&graph, &wall_mesh::MeshSettings::default()).unwrap();
	let top = Vec2::new(0.0, 1.0);

	let proposal =
		propose_at_ray(&graph, &mesh, &OpeningSettings::default(), ray_down_at(top)).unwrap();

	assert_eq!(proposal.wall, arc);
	// A quarter of the semicircle, not the world X coordinate of the hit.
	assert!((proposal.spec.center_distance - std::f64::consts::FRAC_PI_2).abs() < 1e-3);
}

#[test]
fn ray_beside_every_wall_proposes_nothing() {
	let house = house_with_wall(Vec2::ZERO, Vec2::new(4.0, 0.0));

	assert!(propose_down_at(&house, Vec2::new(2.0, 2.0)).is_none());
}

fn placement_app(house: House) -> App {
	let mut app = App::new();
	app.add_plugins((EditorSchedulePlugin, WallToolOpeningPlugin))
		.init_resource::<SnapSettings>()
		.insert_resource(ActiveTool(Tool::PlaceOpening))
		.insert_resource(house)
		.init_resource::<GroundPointer>()
		.init_resource::<ButtonInput<MouseButton>>()
		.init_resource::<ButtonInput<KeyCode>>();
	app
}

fn pointer_ray_at(position: Vec2) -> GroundPointer {
	GroundPointer {
		ray: Some(ray_down_at(position)),
		..Default::default()
	}
}

fn pressed() -> ButtonInput<MouseButton> {
	let mut mouse = ButtonInput::default();
	mouse.press(MouseButton::Left);
	mouse
}

#[test]
fn hovering_a_wall_previews_the_opening_without_changing_the_house() {
	let mut app = placement_app(house_with_wall(Vec2::ZERO, Vec2::new(4.0, 0.0)));
	app.insert_resource(pointer_ray_at(Vec2::new(2.0, 0.0)));
	app.update();

	assert!(
		app.world()
			.resource::<EditPreview>()
			.valid_candidate()
			.is_some()
	);
	assert_eq!(
		app.world().resource::<House>().graph().openings().count(),
		0
	);
}

#[test]
fn click_commits_the_exact_previewed_mesh() {
	let mut app = placement_app(house_with_wall(Vec2::ZERO, Vec2::new(4.0, 0.0)));
	app.insert_resource(pointer_ray_at(Vec2::new(2.0, 0.0)));
	app.update();
	let expected_mesh = app
		.world()
		.resource::<EditPreview>()
		.valid_candidate()
		.unwrap()
		.mesh()
		.clone();

	app.insert_resource(pressed());
	app.update();

	let house = app.world().resource::<House>();
	assert_eq!(house.graph().openings().count(), 1);
	assert_eq!(house.mesh(), &expected_mesh);
}

#[test]
fn invalid_placement_shows_its_error_and_never_changes_the_house() {
	// A 0.5 m wall cannot hold a 0.9 m opening.
	let mut app = placement_app(house_with_wall(Vec2::ZERO, Vec2::new(0.5, 0.0)));
	app.insert_resource(pointer_ray_at(Vec2::new(0.25, 0.0)))
		.insert_resource(pressed());
	app.update();

	assert!(matches!(
		app.world().resource::<EditPreview>().state,
		Some(PreviewState {
			outcome: Err(_),
			..
		})
	));
	let house = app.world().resource::<House>();
	assert_eq!(house.revision(), 1);
	assert_eq!(house.graph().openings().count(), 0);
}

#[test]
fn moving_off_the_wall_or_leaving_the_tool_clears_the_preview() {
	let mut app = placement_app(house_with_wall(Vec2::ZERO, Vec2::new(4.0, 0.0)));
	app.insert_resource(pointer_ray_at(Vec2::new(2.0, 0.0)));
	app.update();
	app.insert_resource(pointer_ray_at(Vec2::new(2.0, 2.0)));
	app.update();
	assert!(app.world().resource::<EditPreview>().state.is_none());

	app.insert_resource(pointer_ray_at(Vec2::new(2.0, 0.0)));
	app.update();
	app.insert_resource(ActiveTool(Tool::Select));
	app.update();
	assert!(app.world().resource::<EditPreview>().state.is_none());
}

fn graph_with_wall_and_openings(length: f32, centers: &[f64]) -> (WallGraph, Wall) {
	let mut graph = WallGraph::new();
	let wall = graph.add_wall(Vec2::ZERO, Vec2::new(length, 0.0)).unwrap()[0];
	for &center_distance in centers {
		graph
			.add_opening(wall, OpeningSettings::default().spec_at(center_distance))
			.unwrap();
	}
	(graph, wall)
}

/// Snaps a default opening proposed at `center_distance`. A huge grid spacing keeps grid
/// candidates out of range unless a test is about the grid.
fn snapped_center(graph: &WallGraph, wall: Wall, center_distance: f64, grid_spacing: f32) -> f64 {
	let proposal = OpeningProposal {
		wall,
		spec: OpeningSettings::default().spec_at(center_distance),
	};
	snap_proposal(graph, proposal, grid_spacing)
		.spec
		.center_distance
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

	assert_eq!(snapped_center(&graph, wall, 2.0, 100.0), 2.0);
}

#[test]
fn grid_distance_snaps_when_nothing_structural_is_near() {
	let (graph, wall) = graph_with_wall_and_openings(6.0, &[1.0, 5.0]);

	assert_eq!(snapped_center(&graph, wall, 1.9, 0.5), 2.0);
}

#[test]
fn structural_candidates_beat_a_nearer_grid_distance() {
	let (graph, wall) = graph_with_wall_and_openings(4.0, &[]);

	assert_eq!(snapped_center(&graph, wall, 2.08, 0.1), 2.0);
}

#[test]
fn holding_shift_disables_opening_snapping() {
	let mut app = placement_app(house_with_wall(Vec2::ZERO, Vec2::new(4.0, 0.0)));
	let mut keyboard = ButtonInput::<KeyCode>::default();
	keyboard.press(KeyCode::ShiftLeft);
	app.insert_resource(keyboard)
		.insert_resource(pointer_ray_at(Vec2::new(2.08, 0.0)));
	app.update();

	let Some(PreviewState {
		proposal: Proposal::Opening(proposal),
		..
	}) = app.world().resource::<EditPreview>().state
	else {
		panic!("expected an opening preview");
	};
	assert!((proposal.spec.center_distance - 2.08).abs() < 1e-3);
}

#[test]
fn edge_snap_lands_on_the_nearest_valid_position_beside_a_junction() {
	let mut graph = WallGraph::new();
	let wall = graph.add_wall(Vec2::ZERO, Vec2::new(4.0, 0.0)).unwrap()[0];
	graph.add_wall(Vec2::ZERO, Vec2::new(0.0, 4.0)).unwrap();
	let valid_at = |center_distance: f64| {
		let proposal = OpeningProposal {
			wall,
			spec: OpeningSettings::default().spec_at(center_distance),
		};
		propose_opening(&graph, &proposal).is_ok()
	};
	let flush = f64::from(OpeningSettings::default().width) / 2.0;
	assert!(!valid_at(flush));

	let snapped = snapped_center(&graph, wall, flush + 0.05, 100.0);

	assert!(snapped > flush);
	assert!(valid_at(snapped));
	// Any closer to the junction and the opening is rejected.
	assert!(!valid_at(snapped - 0.005));
}
