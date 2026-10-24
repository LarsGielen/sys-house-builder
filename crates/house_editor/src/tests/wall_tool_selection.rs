use super::*;
use crate::{
	edit::{OpeningProposal, WallProposal, propose_opening, propose_wall},
	opening_category::OpeningCategory,
};
use wall_graph::{OpeningSpec, WallDimensions};

fn house_with_walls(walls: &[(Vec2, Vec2)]) -> House {
	let mut house = House::default();
	for &(start, end) in walls {
		let proposal = WallProposal {
			start,
			end,
			dimensions: WallDimensions::default(),
		};
		let candidate = propose_wall(house.committed(), &proposal).unwrap();
		house.commit(candidate);
	}
	house
}

fn pick_wall(graph: &WallGraph, mesh: &WallMesh, origin: Vec3, direction: Vec3) -> Option<Wall> {
	pick_wall_hit(graph, mesh, origin, direction).map(|hit| hit.wall)
}

fn pick_down_at(house: &House, position: Vec2) -> Option<Wall> {
	pick_wall(house.graph(), house.mesh(), position.extend(5.0), -Vec3::Z)
}

#[test]
fn ray_through_a_wall_top_picks_that_wall() {
	let house = house_with_walls(&[(Vec2::ZERO, Vec2::new(4.0, 0.0))]);
	let wall = house.graph().walls().next().unwrap();

	assert_eq!(pick_down_at(&house, Vec2::new(2.0, 0.0)), Some(wall));
}

#[test]
fn ray_beside_every_wall_picks_nothing() {
	let house = house_with_walls(&[(Vec2::ZERO, Vec2::new(4.0, 0.0))]);

	assert_eq!(pick_down_at(&house, Vec2::new(2.0, 1.0)), None);
}

#[test]
fn end_cap_picks_its_wall() {
	let house = house_with_walls(&[(Vec2::ZERO, Vec2::new(4.0, 0.0))]);
	let wall = house.graph().walls().next().unwrap();

	let picked = pick_wall(
		house.graph(),
		house.mesh(),
		Vec3::new(-2.0, 0.0, 1.0),
		Vec3::X,
	);

	assert_eq!(picked, Some(wall));
}

#[test]
fn junction_hit_picks_the_wall_nearest_the_hit() {
	let house = house_with_walls(&[
		(Vec2::new(-4.0, 0.0), Vec2::ZERO),
		(Vec2::ZERO, Vec2::new(0.0, 4.0)),
	]);

	let west = pick_down_at(&house, Vec2::new(-0.05, 0.0)).unwrap();
	let north = pick_down_at(&house, Vec2::new(0.0, 0.05)).unwrap();

	assert_ne!(west, north);
	assert_eq!(
		house
			.graph()
			.wall_closest_point(west, Vec2::new(-4.0, 0.0))
			.unwrap()
			.1,
		Vec2::new(-4.0, 0.0)
	);
	assert_eq!(
		house
			.graph()
			.wall_closest_point(north, Vec2::new(0.0, 4.0))
			.unwrap()
			.1,
		Vec2::new(0.0, 4.0)
	);
}

#[test]
fn commit_clears_the_selection() {
	let mut app = App::new();
	app.init_resource::<House>()
		.init_resource::<SelectedTarget>()
		.init_resource::<WallSelectionCarryOver>()
		.add_systems(
			Update,
			clear_selection_after_commit.run_if(resource_changed::<House>),
		);
	app.update();
	let wall = house_with_walls(&[(Vec2::ZERO, Vec2::X)])
		.graph()
		.walls()
		.next();
	app.world_mut().resource_mut::<SelectedTarget>().0 = wall.map(SelectionTarget::Wall);
	app.update();
	assert!(app.world().resource::<SelectedTarget>().0.is_some());

	let candidate = propose_wall(
		app.world().resource::<House>().committed(),
		&WallProposal {
			start: Vec2::ZERO,
			end: Vec2::X,
			dimensions: WallDimensions::default(),
		},
	)
	.unwrap();
	app.world_mut().resource_mut::<House>().commit(candidate);
	app.update();

	assert!(app.world().resource::<SelectedTarget>().0.is_none());
}

#[test]
fn escape_wins_over_a_click_in_the_same_frame() {
	let house = house_with_walls(&[(Vec2::ZERO, Vec2::new(4.0, 0.0))]);
	let wall = house.graph().walls().next().unwrap();
	let mut keyboard = ButtonInput::<KeyCode>::default();
	keyboard.press(KeyCode::Escape);
	let mut mouse = ButtonInput::<MouseButton>::default();
	mouse.press(MouseButton::Left);

	let mut app = App::new();
	app.insert_resource(house)
		.insert_resource(keyboard)
		.insert_resource(mouse)
		.insert_resource(ActiveTool(Tool::Select))
		.insert_resource(GroundPointer {
			ray: Some(Ray3d::new(Vec3::new(2.0, 5.0, 0.0), Dir3::NEG_Y)),
			..Default::default()
		})
		.insert_resource(SelectedTarget(Some(SelectionTarget::Wall(wall))))
		.init_resource::<HoveredTarget>()
		.add_systems(Update, update_selection);
	app.update();

	assert_eq!(app.world().resource::<HoveredTarget>().wall(), Some(wall));
	assert_eq!(app.world().resource::<SelectedTarget>().0, None);
}

/// A 6 m wall along X with a door 1.0 m wide and 2.1 m high centred 3 m along it.
fn house_with_door() -> (House, OpeningId) {
	let mut house = house_with_walls(&[(Vec2::ZERO, Vec2::new(6.0, 0.0))]);
	let proposal = OpeningProposal {
		wall: house.graph().walls().next().unwrap(),
		spec: OpeningSpec {
			center_distance: 3.0,
			width: 1.0,
			bottom: 0.0,
			height: 2.1,
		},
		category: OpeningCategory::Door,
	};
	house.commit(propose_opening(house.committed(), &proposal).unwrap());
	let (id, _) = house.graph().openings().next().unwrap();
	(house, id)
}

/// A horizontal ray along Y at the given X and elevation, from the south of the wall.
fn pick_across_at(house: &House, x: f32, elevation: f32) -> Option<SelectionTarget> {
	pick_target(
		house.graph(),
		house.mesh(),
		Vec3::new(x, -5.0, elevation),
		Vec3::Y,
	)
}

#[test]
fn ray_through_the_middle_of_a_doorway_picks_the_opening() {
	let (house, id) = house_with_door();

	assert_eq!(
		pick_across_at(&house, 3.0, 1.0),
		Some(SelectionTarget::Opening(id))
	);
}

#[test]
fn ray_through_solid_wall_beside_or_above_a_doorway_picks_the_wall() {
	let (house, _) = house_with_door();
	let wall = house.graph().walls().next().unwrap();

	assert_eq!(
		pick_across_at(&house, 4.0, 1.0),
		Some(SelectionTarget::Wall(wall))
	);
	assert_eq!(
		pick_across_at(&house, 3.0, 2.3),
		Some(SelectionTarget::Wall(wall))
	);
}

#[test]
fn a_jamb_picks_the_opening_it_belongs_to() {
	let (house, id) = house_with_door();

	// Inside the doorway, looking along the wall at the jamb.
	let picked = pick_target(
		house.graph(),
		house.mesh(),
		Vec3::new(3.0, 0.0, 1.0),
		Vec3::X,
	);

	assert_eq!(picked, Some(SelectionTarget::Opening(id)));
}

#[test]
fn a_nearer_wall_hides_the_opening_behind_it() {
	let (mut house, _) = house_with_door();
	let screen = propose_wall(
		house.committed(),
		&WallProposal {
			start: Vec2::new(2.0, -2.0),
			end: Vec2::new(4.0, -2.0),
			dimensions: WallDimensions::default(),
		},
	)
	.unwrap();
	house.commit(screen);
	let screen_wall = house
		.graph()
		.walls()
		.find(|&wall| house.graph().node_position(wall.origin()) == Some(Vec2::new(2.0, -2.0)))
		.unwrap();

	assert_eq!(
		pick_across_at(&house, 3.0, 1.0),
		Some(SelectionTarget::Wall(screen_wall))
	);
}

#[test]
fn a_commit_keeps_an_opening_selection_unless_the_opening_is_gone() {
	let (house, id) = house_with_door();
	let mut app = App::new();
	app.insert_resource(house)
		.init_resource::<SelectedTarget>()
		.init_resource::<WallSelectionCarryOver>()
		.insert_resource(SelectedTarget(Some(SelectionTarget::Opening(id))))
		.add_systems(
			Update,
			clear_selection_after_commit.run_if(resource_changed::<House>),
		);
	app.update();
	assert_eq!(app.world().resource::<SelectedTarget>().opening(), Some(id));

	// A house without that opening, as after the opening or its wall is removed.
	app.insert_resource(house_with_walls(&[(Vec2::ZERO, Vec2::new(6.0, 0.0))]));
	app.update();
	assert_eq!(app.world().resource::<SelectedTarget>().0, None);
}
