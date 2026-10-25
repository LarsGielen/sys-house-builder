use super::*;
use crate::ui::{opening as opening_ui, wall as wall_ui};
use crate::{
	document::edit::EditError, tests::support::editor_ui_app, tools::active::ActivateTool,
	viewport::ground_pointer::GroundPointer,
};
use bevy::{
	input::mouse::{MouseScrollUnit, MouseWheel},
	window::PrimaryWindow,
};
use house_document::OpeningCategory;
use wall_graph::{OpeningId, Wall, WallError};

fn house_with_opening() -> (House, Wall, OpeningId) {
	crate::tests::support::house_with_opening(6.0, 3.0, OpeningCategory::Window)
}

#[test]
fn panel_sections_follow_the_tool_and_selection() {
	let (house, wall, opening) = house_with_opening();
	let mut app = editor_ui_app();
	app.insert_resource(house);
	app.update();
	for (tool, selected, expected_section) in [
		(Tool::Select, None, None),
		(
			Tool::Select,
			Some(SelectionTarget::Wall(wall)),
			Some(PanelSection::SelectedWall),
		),
		(
			Tool::Select,
			Some(SelectionTarget::Opening(opening)),
			Some(PanelSection::SelectedOpening),
		),
		(
			Tool::PlaceOpening,
			None,
			Some(PanelSection::OpeningPlacement),
		),
		(Tool::DrawWall, None, Some(PanelSection::WallDrawing)),
		(Tool::DrawArc, None, Some(PanelSection::ArcDrawing)),
		(Tool::Delete, None, None),
	] {
		app.world_mut()
			.write_message(crate::actions::EditorAction::from(ActivateTool(tool)));
		app.insert_resource(SelectedTarget(selected));
		app.update();
		let visible: Vec<_> = app
			.world_mut()
			.query::<(&PanelSection, &Node)>()
			.iter(app.world())
			.filter(|(_, node)| node.display != Display::None)
			.map(|(section, _)| std::mem::discriminant(section))
			.collect();
		let expected: Vec<_> = expected_section
			.as_ref()
			.map(std::mem::discriminant)
			.into_iter()
			.collect();
		assert_eq!(visible, expected, "tool {tool:?}, selection {selected:?}");
		assert!(
			app.world_mut()
				.query::<&Text>()
				.iter(app.world())
				.any(|text| text.0.starts_with("Snap: "))
		);
	}
}

#[test]
fn small_windows_wrap_the_toolbar_and_allow_scrolling_the_panel() {
	let mut app = editor_ui_app();
	let window_entity = app
		.world_mut()
		.query_filtered::<Entity, With<PrimaryWindow>>()
		.single(app.world())
		.unwrap();
	{
		let mut window = app.world_mut().get_mut::<Window>(window_entity).unwrap();
		window.resolution.set(240.0, 300.0);
		window.set_cursor_position(Some(Vec2::new(120.0, 150.0)));
	}
	app.world_mut()
		.write_message(crate::actions::EditorAction::from(ActivateTool(
			Tool::DrawWall,
		)));
	for _ in 0..3 {
		app.update();
	}
	let panel = app
		.world_mut()
		.query_filtered::<(Entity, &ComputedNode), With<EditorPanel>>()
		.single(app.world())
		.unwrap();
	let panel_entity = panel.0;
	assert!(panel.1.content_size().y > panel.1.size().y);
	let buttons: Vec<_> = app
		.world_mut()
		.query_filtered::<&UiGlobalTransform, With<ToolButton>>()
		.iter(app.world())
		.map(|transform| transform.translation)
		.collect();
	assert!(buttons.iter().any(|position| position.y != buttons[0].y));
	let camera_before = *app
		.world_mut()
		.query_filtered::<&Transform, With<Camera3d>>()
		.single(app.world())
		.unwrap();
	app.world_mut().write_message(MouseWheel {
		unit: MouseScrollUnit::Line,
		x: 0.0,
		y: -2.0,
		window: window_entity,
		phase: bevy::input::touch::TouchPhase::Moved,
	});
	app.update();
	assert!(app.world().get::<ScrollPosition>(panel_entity).unwrap().y > 0.0);
	let camera_after = *app
		.world_mut()
		.query_filtered::<&Transform, With<Camera3d>>()
		.single(app.world())
		.unwrap();
	assert_eq!(camera_after, camera_before);
	assert!(app.world().resource::<GroundPointer>().ray.is_none());
}

#[test]
fn deletion_readouts_include_the_underlying_error() {
	let (house, wall, opening) = house_with_opening();
	let error = EditError::Graph(WallError::UnknownOpening);
	assert!(
		opening_ui::describe_opening_removal(&house, opening, Some(&error))
			.contains(&error.to_string())
	);
	let error = EditError::Graph(WallError::UnknownWall);
	assert!(
		wall_ui::describe_wall_removal(&house, wall, Err(&error), Tool::Select)
			.contains(&error.to_string())
	);
}
