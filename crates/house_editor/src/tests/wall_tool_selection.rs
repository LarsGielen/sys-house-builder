use super::*;
use crate::{edit::WallProposal, edit::propose_wall};
use wall_graph::WallDimensions;

fn house_with_walls(walls: &[(Vec2, Vec2)]) -> House {
	let mut house = House::default();
	for &(start, end) in walls {
		let proposal = WallProposal {
			start,
			end,
			dimensions: WallDimensions::default(),
		};
		let candidate = propose_wall(house.graph(), &proposal).unwrap();
		house.commit(candidate);
	}
	house
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
		.init_resource::<SelectedWall>()
		.add_systems(
			Update,
			clear_selection_after_commit.run_if(resource_changed::<House>),
		);
	app.update();
	let wall = house_with_walls(&[(Vec2::ZERO, Vec2::X)])
		.graph()
		.walls()
		.next();
	app.world_mut().resource_mut::<SelectedWall>().0 = wall;
	app.update();
	assert!(app.world().resource::<SelectedWall>().0.is_some());

	let candidate = propose_wall(
		app.world().resource::<House>().graph(),
		&WallProposal {
			start: Vec2::ZERO,
			end: Vec2::X,
			dimensions: WallDimensions::default(),
		},
	)
	.unwrap();
	app.world_mut().resource_mut::<House>().commit(candidate);
	app.update();

	assert!(app.world().resource::<SelectedWall>().0.is_none());
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
		.insert_resource(SelectedWall(Some(wall)))
		.init_resource::<HoveredWall>()
		.add_systems(Update, update_selection);
	app.update();

	assert_eq!(app.world().resource::<HoveredWall>().0, Some(wall));
	assert_eq!(app.world().resource::<SelectedWall>().0, None);
}
