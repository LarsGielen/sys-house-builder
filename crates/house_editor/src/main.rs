mod coordinates;
mod edit;
mod editor_schedule;
mod editor_ui;
mod ground_pointer;
mod house;
mod orbit_camera;
mod snapping;
mod wall_render;
mod wall_tool_active;
mod wall_tool_drawing;
mod wall_tool_opening;
mod wall_tool_selection;

use bevy::prelude::*;
use editor_schedule::EditorSchedulePlugin;
use editor_ui::EditorUiPlugin;
use ground_pointer::GroundPointerPlugin;
use house::House;
use orbit_camera::{OrbitCamera, OrbitCameraPlugin};
use snapping::SnappingPlugin;
use wall_render::WallRenderPlugin;
use wall_tool_active::WallToolActivePlugin;
use wall_tool_drawing::WallToolDrawingPlugin;
use wall_tool_opening::WallToolOpeningPlugin;
use wall_tool_selection::WallToolSelectionPlugin;

fn main() {
	App::new()
		.add_plugins(DefaultPlugins.set(WindowPlugin {
			primary_window: Some(Window {
				name: Some("bevy-dev".to_string()),
				title: "House Editor".to_string(),
				..default()
			}),
			..default()
		}))
		.add_plugins((
			EditorSchedulePlugin,
			EditorUiPlugin,
			OrbitCameraPlugin,
			WallRenderPlugin,
			GroundPointerPlugin,
			SnappingPlugin,
			WallToolActivePlugin,
			WallToolDrawingPlugin,
			WallToolOpeningPlugin,
			WallToolSelectionPlugin,
		))
		.init_resource::<House>()
		.add_systems(Startup, setup_scene)
		.run();
}

fn setup_scene(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	commands.spawn((
		Name::new("editor-floor"),
		Mesh3d(meshes.add(Rectangle::new(4.0, 4.0))),
		MeshMaterial3d(materials.add(Color::WHITE)),
		Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
	));

	commands.spawn((
		PointLight {
			shadow_maps_enabled: true,
			..default()
		},
		Transform::from_xyz(4.0, 8.0, 4.0),
	));

	commands.spawn((Camera3d::default(), OrbitCamera::default()));
}

#[cfg(test)]
#[path = "tests/editor.rs"]
mod tests;
