use crate::{
	edit::EditPreview, ground_pointer::GroundPointer, house::House, snapping::SnappedPointer,
};
use bevy::prelude::*;

#[path = "support.rs"]
pub(crate) mod support;

use support::editor_app;

#[test]
fn picking_uses_current_camera_ancestors_and_markers_propagate_in_the_same_frame() {
	#[derive(Component)]
	struct CameraParent;
	let mut app = editor_app();
	let camera = app
		.world_mut()
		.query_filtered::<Entity, With<Camera3d>>()
		.single(app.world())
		.unwrap();
	let parent = app
		.world_mut()
		.spawn((CameraParent, Transform::default()))
		.id();
	app.world_mut().entity_mut(parent).add_child(camera);
	app.add_systems(
		Update,
		|mut parent: Single<&mut Transform, With<CameraParent>>| {
			parent.translation.x = 3.0;
		},
	);
	app.update();
	let hit = app.world().resource::<GroundPointer>().hit.unwrap();
	assert!(hit.graph_position.distance(Vec2::new(3.0, 0.0)) < 1e-4);
	assert!(app.world().resource::<SnappedPointer>().target.is_some());
	let marker_position = app
		.world_mut()
		.query::<(&Name, &GlobalTransform)>()
		.iter(app.world())
		.find(|(name, _)| name.as_str() == "cursor-marker")
		.unwrap()
		.1
		.translation();
	assert!(
		marker_position.distance(crate::coordinates::graph_to_bevy(
			hit.graph_position.extend(0.0)
		)) < 1e-4
	);
}

#[test]
fn moving_over_ui_and_clicking_does_not_use_the_previous_ground_hit() {
	let mut app = editor_app();
	app.update();
	assert!(app.world().resource::<GroundPointer>().hit.is_some());
	let panel = app.world_mut().spawn(Interaction::Hovered).id();
	app.world_mut()
		.resource_mut::<ButtonInput<MouseButton>>()
		.press(MouseButton::Left);
	app.update();
	assert!(app.world().resource::<GroundPointer>().hit.is_none());
	assert!(app.world().resource::<SnappedPointer>().target.is_none());
	app.world_mut().entity_mut(panel).insert(Interaction::None);
	app.world_mut()
		.resource_mut::<ButtonInput<MouseButton>>()
		.clear();
	app.world_mut()
		.resource_mut::<ButtonInput<MouseButton>>()
		.release(MouseButton::Left);
	app.update();
	assert!(app.world().resource::<EditPreview>().state.is_none());
	assert_eq!(app.world().resource::<House>().revision(), 0);
}
