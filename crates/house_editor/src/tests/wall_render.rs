use super::*;
use crate::{
	edit::{PreviewState, Proposal, WallProposal, propose_wall},
	editor_schedule::EditorSchedulePlugin,
};
use bevy::{asset::AssetPlugin, gizmos::GizmoPlugin, transform::TransformPlugin};
use wall_graph::WallDimensions;

fn render_app() -> App {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin::default(),
		TransformPlugin,
		bevy::mesh::MeshPlugin,
		bevy::camera::CameraPlugin,
		GizmoPlugin,
	))
	.init_resource::<Assets<Mesh>>()
	.init_resource::<Assets<StandardMaterial>>()
	.init_resource::<House>()
	.init_resource::<EditPreview>()
	.add_plugins((EditorSchedulePlugin, WallRenderPlugin));
	app
}

fn preview_wall(app: &mut App, end: Vec2) {
	let house = app.world().resource::<House>();
	let proposal = WallProposal {
		start: Vec2::ZERO,
		end,
		dimensions: WallDimensions::default(),
	};
	let state = PreviewState {
		house_revision: house.revision(),
		proposal: Proposal::Wall(proposal),
		outcome: propose_wall(house.graph(), &proposal),
	};
	app.world_mut().resource_mut::<EditPreview>().state = Some(state);
}

#[test]
fn unchanged_preview_keeps_its_entity_and_mesh_asset() {
	let mut app = render_app();
	preview_wall(&mut app, Vec2::X);
	app.update();
	let preview_entity = app
		.world_mut()
		.query_filtered::<Entity, With<PreviewDisplay>>()
		.single(app.world())
		.unwrap();
	let mesh_count = app.world().resource::<Assets<Mesh>>().len();
	app.update();
	assert_eq!(
		app.world_mut()
			.query_filtered::<Entity, With<PreviewDisplay>>()
			.single(app.world())
			.unwrap(),
		preview_entity
	);
	assert_eq!(app.world().resource::<Assets<Mesh>>().len(), mesh_count);
	preview_wall(&mut app, Vec2::Y);
	app.update();
	assert!(app.world().get_entity(preview_entity).is_err());
	assert_eq!(
		app.world_mut()
			.query_filtered::<Entity, With<PreviewDisplay>>()
			.iter(app.world())
			.count(),
		1
	);
}

#[test]
fn new_committed_entities_are_hidden_during_preview_and_visible_after_commit() {
	let mut app = render_app();
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
	preview_wall(&mut app, Vec2::Y);
	app.update();
	assert_eq!(
		*app.world_mut()
			.query_filtered::<&Visibility, With<WallDisplay>>()
			.single(app.world())
			.unwrap(),
		Visibility::Hidden
	);
	let state = app
		.world_mut()
		.resource_mut::<EditPreview>()
		.state
		.take()
		.unwrap();
	app.world_mut()
		.resource_mut::<House>()
		.commit(state.outcome.unwrap());
	app.update();
	assert_eq!(
		*app.world_mut()
			.query_filtered::<&Visibility, With<WallDisplay>>()
			.single(app.world())
			.unwrap(),
		Visibility::Inherited
	);
	assert_eq!(
		app.world_mut()
			.query_filtered::<Entity, With<PreviewDisplay>>()
			.iter(app.world())
			.count(),
		0
	);
}
