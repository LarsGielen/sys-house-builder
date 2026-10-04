use bevy::{
	asset::RenderAssetUsages,
	mesh::{Indices, PrimitiveTopology},
	prelude::*,
};
use wall_mesh::WallMesh;

use crate::{House, coordinates::graph_to_bevy};

/// Rebuilds the displayed walls whenever the committed house changes.
pub struct WallRenderPlugin;
impl Plugin for WallRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Startup, setup_wall_material);
		app.add_systems(
			Update,
			rebuild_wall_entities.run_if(resource_changed::<House>),
		);
	}
}

/// Marks entities that display the committed walls and are replaced on every commit.
#[derive(Component)]
struct WallDisplay;

#[derive(Resource)]
struct WallMaterial(Handle<StandardMaterial>);

fn setup_wall_material(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
	commands.insert_resource(WallMaterial(materials.add(Color::srgb(0.85, 0.82, 0.78))));
}

fn rebuild_wall_entities(
	mut commands: Commands,
	house: Res<House>,
	wall_material: Res<WallMaterial>,
	mut meshes: ResMut<Assets<Mesh>>,
	old_walls: Query<Entity, With<WallDisplay>>,
) {
	for entity in &old_walls {
		commands.entity(entity).despawn();
	}

	if house.mesh.is_empty() {
		return;
	}

	commands.spawn((
		Name::new("house-walls"),
		WallDisplay,
		Mesh3d(meshes.add(to_bevy_mesh(&house.mesh))),
		MeshMaterial3d(wall_material.0.clone()),
	));
}

fn to_bevy_mesh(wall_mesh: &WallMesh) -> Mesh {
	let positions: Vec<[f32; 3]> = wall_mesh
		.positions()
		.iter()
		.map(|&position| graph_to_bevy(position).to_array())
		.collect();
	let normals: Vec<[f32; 3]> = wall_mesh
		.normals()
		.iter()
		.map(|&normal| graph_to_bevy(normal).to_array())
		.collect();

	Mesh::new(
		PrimitiveTopology::TriangleList,
		RenderAssetUsages::default(),
	)
	.with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
	.with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
	.with_inserted_indices(Indices::U32(wall_mesh.indices().to_vec()))
}
