use bevy::{
	asset::AssetPlugin,
	camera::{CameraPlugin, CameraUpdateSystems},
	gizmos::GizmoPlugin,
	image::ImagePlugin,
	input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
	mesh::MeshPlugin,
	prelude::*,
	render::{camera::camera_system, texture::ManualTextureViews},
	transform::TransformPlugin,
	window::PrimaryWindow,
};

use crate::{
	edit::EditPreview,
	editor_schedule::EditorSchedulePlugin,
	ground_pointer::{GroundPointer, GroundPointerPlugin},
	house::House,
	orbit_camera::{OrbitCamera, OrbitCameraPlugin},
	snapping::{SnappedPointer, SnappingPlugin},
	wall_drawing::WallDrawingPlugin,
	wall_render::WallRenderPlugin,
};

fn editor_app() -> App {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin::default(),
		TransformPlugin,
		MeshPlugin,
		CameraPlugin,
		ImagePlugin::default(),
		WindowPlugin::default(),
		GizmoPlugin,
	))
	.init_resource::<Assets<StandardMaterial>>()
	.init_resource::<ManualTextureViews>()
	.init_resource::<ButtonInput<MouseButton>>()
	.init_resource::<ButtonInput<KeyCode>>()
	.init_resource::<AccumulatedMouseMotion>()
	.init_resource::<AccumulatedMouseScroll>()
	.init_resource::<House>()
	.add_plugins((
		EditorSchedulePlugin,
		OrbitCameraPlugin,
		GroundPointerPlugin,
		SnappingPlugin,
		WallDrawingPlugin,
		WallRenderPlugin,
	))
	.add_systems(PostUpdate, camera_system.in_set(CameraUpdateSystems));
	let mut window = app
		.world_mut()
		.query_filtered::<&mut Window, With<PrimaryWindow>>()
		.single_mut(app.world_mut())
		.unwrap();
	window.focused = true;
	let center = window.size() / 2.0;
	window.set_cursor_position(Some(center));
	app.world_mut()
		.spawn((Camera3d::default(), OrbitCamera::default()));
	app
}

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

#[test]
fn editor_ui_runs_with_the_ordered_picking_and_presentation_schedule() {
	let mut app = editor_app();
	app.init_resource::<Assets<bevy::image::TextureAtlasLayout>>()
		.add_plugins((
			bevy::input::InputPlugin,
			bevy::picking::DefaultPickingPlugins,
			bevy::text::TextPlugin,
			bevy::ui::UiPlugin,
			crate::editor_ui::EditorUiPlugin,
		));
	app.update();
	app.update();
	assert!(app.world().resource::<GroundPointer>().hit.is_some());
	assert!(
		app.world_mut()
			.query::<&Text>()
			.iter(app.world())
			.any(|text| text.0.starts_with("Snap: "))
	);
}
