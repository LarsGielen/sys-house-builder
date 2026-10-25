use super::*;
use crate::document::edit::{OpeningProposal, WallProposal, propose_opening, propose_wall};
use house_document::OpeningCategory;
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
	app.add_plugins(crate::input::ActionsPlugin)
		.insert_resource(house)
		.insert_resource(keyboard)
		.insert_resource(mouse)
		.insert_resource(ActiveTool(Tool::Select))
		.insert_resource(GroundPointer {
			ray: Some(Ray3d::new(Vec3::new(2.0, 5.0, 0.0), Dir3::NEG_Y)),
			..Default::default()
		})
		.insert_resource(SelectedTarget(Some(SelectionTarget::Wall(wall))))
		.init_resource::<HoveredTarget>()
		.init_resource::<HoveredHandle>()
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
