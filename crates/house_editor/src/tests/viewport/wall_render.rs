use super::*;
use crate::{
	document::edit::{PreviewState, Proposal, WallProposal, propose_wall, propose_wall_removal},
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
		outcome: propose_wall(house.committed(), &proposal),
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
		app.world().resource::<House>().committed(),
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

fn preview_removal(app: &mut App, wall: wall_graph::Wall) {
	let house = app.world().resource::<House>();
	let state = PreviewState {
		house_revision: house.revision(),
		proposal: Proposal::WallRemoval(wall),
		outcome: propose_wall_removal(house.committed(), wall),
	};
	app.world_mut().resource_mut::<EditPreview>().state = Some(state);
}

fn count<C: Component>(app: &mut App) -> usize {
	app.world_mut()
		.query_filtered::<Entity, With<C>>()
		.iter(app.world())
		.count()
}

#[test]
fn deletion_preview_tints_only_the_target_and_clears_when_cancelled() {
	let mut app = render_app();
	for y in [0.0, 3.0] {
		let candidate = propose_wall(
			app.world().resource::<House>().committed(),
			&WallProposal {
				start: Vec2::new(0.0, y),
				end: Vec2::new(4.0, y),
				dimensions: WallDimensions::default(),
			},
		)
		.unwrap();
		app.world_mut().resource_mut::<House>().commit(candidate);
	}
	let wall = app
		.world()
		.resource::<House>()
		.graph()
		.walls()
		.next()
		.unwrap();
	preview_removal(&mut app, wall);
	app.update();

	assert_eq!(
		*app.world_mut()
			.query_filtered::<&Visibility, With<WallDisplay>>()
			.single(app.world())
			.unwrap(),
		Visibility::Inherited
	);
	assert_eq!(count::<PreviewDisplay>(&mut app), 0);
	assert_eq!(count::<RemovalDisplay>(&mut app), 1);
	let tint = app
		.world_mut()
		.query_filtered::<&Mesh3d, With<RemovalDisplay>>()
		.single(app.world())
		.unwrap()
		.0
		.clone();
	let tinted_indices = app
		.world()
		.resource::<Assets<Mesh>>()
		.get(&tint)
		.unwrap()
		.indices()
		.unwrap()
		.len();
	let committed_indices = app.world().resource::<House>().mesh().indices().len();
	assert!(tinted_indices > 0 && tinted_indices < committed_indices);
	let mesh = app.world().resource::<House>().mesh();
	let expected_indices: Vec<_> = mesh
		.sections()
		.iter()
		.filter(|section| section.tag.source == SurfaceSource::Wall(wall))
		.flat_map(|section| {
			mesh.indices()[section.indices.clone()]
				.iter()
				.map(|&index| index as usize)
		})
		.collect();
	let actual_indices: Vec<_> = app
		.world()
		.resource::<Assets<Mesh>>()
		.get(&tint)
		.unwrap()
		.indices()
		.unwrap()
		.iter()
		.collect();
	assert_eq!(actual_indices, expected_indices);
	app.world_mut().resource_mut::<EditPreview>().state = None;
	app.update();
	assert_eq!(count::<RemovalDisplay>(&mut app), 0);
}
