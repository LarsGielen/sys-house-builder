use super::*;
use crate::tests::support::editor_ui_app;
use crate::tools::opening::OpeningSettings;

fn press(app: &mut App, wanted: DockTool) {
	let button = app
		.world_mut()
		.query::<(Entity, &DockToolButton)>()
		.iter(app.world())
		.find(|(_, button)| button.0 == wanted)
		.map(|(entity, _)| entity)
		.unwrap();
	app.world_mut()
		.entity_mut(button)
		.insert(Interaction::Pressed);
	for _ in 0..3 {
		app.update();
	}
}

fn visible_catalogs(app: &mut App) -> Vec<DockTool> {
	app.world_mut()
		.query::<(&CatalogGroup, &Node)>()
		.iter(app.world())
		.filter(|(_, node)| node.display != Display::None)
		.map(|(group, _)| group.0)
		.collect()
}

#[test]
fn tool_buttons_choose_the_tool_and_the_opening_category() {
	let mut app = editor_ui_app();
	app.update();
	assert_eq!(visible_catalogs(&mut app), []);

	press(&mut app, DockTool::Window);
	assert_eq!(app.world().resource::<ActiveTool>().0, Tool::PlaceOpening);
	assert_eq!(
		app.world().resource::<OpeningSettings>().category(),
		OpeningCategory::Window
	);
	assert_eq!(visible_catalogs(&mut app), [DockTool::Window]);

	press(&mut app, DockTool::Door);
	assert_eq!(
		app.world().resource::<OpeningSettings>().category(),
		OpeningCategory::Door
	);
	assert_eq!(visible_catalogs(&mut app), [DockTool::Door]);

	press(&mut app, DockTool::Wall);
	assert_eq!(app.world().resource::<ActiveTool>().0, Tool::DrawWall);
	assert_eq!(visible_catalogs(&mut app), []);
}

#[test]
fn furniture_only_opens_its_catalog_and_leaves_when_another_tool_is_chosen() {
	let mut app = editor_ui_app();
	app.update();
	press(&mut app, DockTool::Furniture);
	assert_eq!(app.world().resource::<ActiveTool>().0, Tool::Select);
	assert_eq!(visible_catalogs(&mut app), [DockTool::Furniture]);

	app.world_mut()
		.resource_mut::<Messages<crate::actions::EditorAction>>()
		.write(crate::actions::EditorAction::from(ActivateTool(
			Tool::DrawWall,
		)));
	for _ in 0..3 {
		app.update();
	}
	assert_eq!(visible_catalogs(&mut app), []);
}

#[test]
fn choosing_an_unavailable_entry_says_so() {
	let mut app = editor_ui_app();
	app.update();
	press(&mut app, DockTool::Door);
	let card = app
		.world_mut()
		.query::<(Entity, &CatalogCard)>()
		.iter(app.world())
		.find(|(_, card)| card.entry.name == "Arched")
		.map(|(entity, _)| entity)
		.unwrap();
	app.world_mut()
		.entity_mut(card)
		.insert(Interaction::Pressed);
	app.update();
	assert_eq!(
		app.world().resource::<EditNotice>().message.as_deref(),
		Some("Arched is not available yet")
	);
}
