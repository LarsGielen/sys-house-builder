use super::*;

#[test]
fn activation_message_switches_the_active_tool() {
	let mut app = App::new();
	app.add_plugins(WallToolActivePlugin);
	assert_eq!(app.world().resource::<ActiveTool>().0, Tool::Select);

	app.world_mut().write_message(ActivateTool(Tool::DrawWall));
	app.update();

	assert_eq!(app.world().resource::<ActiveTool>().0, Tool::DrawWall);
}
