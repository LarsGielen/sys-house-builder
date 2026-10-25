use super::*;
use crate::document::edit::PreviewState;
use crate::tests::support::pressed_mouse as pressed;
use crate::{
	editor_schedule::EditorSchedulePlugin, tools::active::ActiveTool,
	viewport::coordinates::graph_to_bevy,
};

fn house_with_wall(start: Vec2, end: Vec2) -> House {
	crate::tests::support::house_with_wall(start, end).0
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
