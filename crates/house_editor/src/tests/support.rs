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
	arc_fields::ArcFieldsPlugin,
	edit::{OpeningProposal, WallProposal, propose_opening, propose_wall},
	editor_schedule::EditorSchedulePlugin,
	ground_pointer::GroundPointerPlugin,
	history::HistoryPlugin,
	house::House,
	opening_fields::OpeningFieldsPlugin,
	orbit_camera::{OrbitCamera, OrbitCameraPlugin},
	snapping::SnappingPlugin,
	text_fields::TextFieldsPlugin,
	wall_fields::WallFieldsPlugin,
	wall_render::WallRenderPlugin,
	wall_tool_active::WallToolActivePlugin,
	wall_tool_arc::WallToolArcPlugin,
	wall_tool_delete::WallToolDeletePlugin,
	wall_tool_drawing::WallToolDrawingPlugin,
	wall_tool_edit_opening::WallToolEditOpeningPlugin,
	wall_tool_edit_wall::WallToolEditWallPlugin,
	wall_tool_handles::WallToolHandlesPlugin,
	wall_tool_opening::WallToolOpeningPlugin,
	wall_tool_selection::WallToolSelectionPlugin,
};

/// The Bevy plugins and resources the editor's systems need, without any editor plugin, for
/// tests that drive one tool with hand-made pointer resources.
pub(crate) fn tool_test_app() -> App {
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
	.init_resource::<AccumulatedMouseScroll>();
	app
}

pub(crate) fn editor_app() -> App {
	let mut app = tool_test_app();
	app.init_resource::<House>()
		.add_plugins((
			EditorSchedulePlugin,
			OrbitCameraPlugin,
			GroundPointerPlugin,
			SnappingPlugin,
			WallToolActivePlugin,
			WallToolDrawingPlugin,
			WallToolOpeningPlugin,
			WallToolDeletePlugin,
			WallToolSelectionPlugin,
			WallToolEditOpeningPlugin,
			WallToolEditWallPlugin,
			TextFieldsPlugin,
			OpeningFieldsPlugin,
			WallFieldsPlugin,
			WallRenderPlugin,
		))
		.add_plugins((
			HistoryPlugin,
			WallToolArcPlugin,
			ArcFieldsPlugin,
			WallToolHandlesPlugin,
			crate::project::ProjectPlugin,
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

pub(crate) fn editor_ui_app() -> App {
	let mut app = editor_app();
	app.init_resource::<Assets<bevy::image::TextureAtlasLayout>>()
		.add_plugins((
			bevy::input::InputPlugin,
			bevy::input_focus::InputFocusPlugin,
			bevy::input_focus::InputDispatchPlugin,
			bevy::ui_widgets::EditableTextInputPlugin,
			bevy::picking::DefaultPickingPlugins,
			bevy::text::TextPlugin,
			bevy::ui::UiPlugin,
			crate::editor_ui::EditorUiPlugin,
		));
	app
}

pub(crate) fn house_with_wall(start: Vec2, end: Vec2) -> (House, wall_graph::Wall) {
	let mut house = House::default();
	house.commit(
		propose_wall(
			house.committed(),
			&WallProposal {
				start,
				end,
				dimensions: wall_graph::WallDimensions::default(),
			},
		)
		.unwrap(),
	);
	let wall = house.graph().walls().next().unwrap();
	(house, wall)
}

pub(crate) fn house_with_opening(
	length: f32,
	center_distance: f64,
	category: crate::opening_category::OpeningCategory,
) -> (House, wall_graph::Wall, wall_graph::OpeningId) {
	let (mut house, wall) = house_with_wall(Vec2::ZERO, Vec2::new(length, 0.0));
	house.commit(
		propose_opening(
			house.committed(),
			&OpeningProposal {
				wall,
				spec: wall_graph::OpeningSpec {
					center_distance,
					width: 0.9,
					bottom: 0.0,
					height: 2.1,
				},
				category,
			},
		)
		.unwrap(),
	);
	let opening = house.graph().openings().next().unwrap().0;
	(house, wall, opening)
}

pub(crate) fn press_key(app: &mut App, key: KeyCode) {
	let mut keyboard = ButtonInput::<KeyCode>::default();
	keyboard.press(key);
	app.insert_resource(keyboard);
}

pub(crate) fn pressed_mouse() -> ButtonInput<MouseButton> {
	let mut mouse = ButtonInput::default();
	mouse.press(MouseButton::Left);
	mouse
}

pub(crate) fn released_mouse() -> ButtonInput<MouseButton> {
	let mut mouse = pressed_mouse();
	mouse.clear();
	mouse.release(MouseButton::Left);
	mouse
}
