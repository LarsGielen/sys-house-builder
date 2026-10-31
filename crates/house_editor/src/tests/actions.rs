use super::*;
use crate::tools::active::Tool;

#[test]
fn every_kind_converts_into_the_stream_and_back() {
	assert_eq!(
		EditorAction::from(HistoryRequest::Undo).history(),
		Some(HistoryRequest::Undo)
	);
	assert_eq!(
		EditorAction::from(ActivateTool(Tool::Delete)).activate_tool(),
		Some(ActivateTool(Tool::Delete))
	);
	assert_eq!(EditorAction::from(HistoryRequest::Undo).settings(), None);
}

#[test]
fn actions_arrive_in_the_order_they_were_sent() {
	let mut app = App::new();
	app.add_plugins(EditorActionPlugin);
	app.add_systems(Update, |mut sender: ActionSender| {
		sender.send(HistoryRequest::Undo);
		sender.send(ActivateTool(Tool::Delete));
		sender.send(HistoryRequest::Redo);
	});
	app.insert_resource(Seen::default());
	app.add_systems(
		PostUpdate,
		|mut stream: MessageReader<EditorAction>, mut seen: ResMut<Seen>| {
			seen.0.extend(stream.read().copied());
		},
	);
	app.update();
	assert_eq!(
		app.world().resource::<Seen>().0,
		vec![
			EditorAction::History(HistoryRequest::Undo),
			EditorAction::ActivateTool(ActivateTool(Tool::Delete)),
			EditorAction::History(HistoryRequest::Redo),
		]
	);
}

#[derive(Resource, Default)]
struct Seen(Vec<EditorAction>);
