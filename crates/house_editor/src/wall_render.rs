use bevy::{
	asset::RenderAssetUsages,
	mesh::{Indices, PrimitiveTopology},
	prelude::*,
};
use wall_mesh::WallMesh;

use crate::{coordinates::graph_to_bevy, edit::EditPreview, house::House};

// Lifts the line off the ground so it is not hidden by the floor.
const PREVIEW_LINE_HEIGHT_METRES: f32 = 0.02;

/// Displays the committed walls and the preview of the edit being proposed.
pub struct WallRenderPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WallRenderSystems;
impl Plugin for WallRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Startup, setup_wall_materials);
		app.add_systems(
			PostUpdate,
			(
				rebuild_wall_entities.run_if(resource_changed::<House>),
				rebuild_preview_entity.run_if(resource_changed::<EditPreview>),
				hide_committed_walls_while_previewing,
				draw_preview_line,
			)
				.chain()
				.in_set(WallRenderSystems),
		);
	}
}

/// Marks entities that display the committed walls and are replaced on every commit.
#[derive(Component)]
struct WallDisplay;

/// Marks the entity that displays the candidate result of the proposed edit.
#[derive(Component)]
struct PreviewDisplay;

#[derive(Resource)]
struct WallMaterials {
	committed: Handle<StandardMaterial>,
	preview: Handle<StandardMaterial>,
}

fn setup_wall_materials(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
	commands.insert_resource(WallMaterials {
		committed: materials.add(Color::srgb(0.85, 0.82, 0.78)),
		preview: materials.add(Color::srgb(0.55, 0.8, 0.95)),
	});
}

fn rebuild_wall_entities(
	mut commands: Commands,
	house: Res<House>,
	wall_materials: Res<WallMaterials>,
	mut meshes: ResMut<Assets<Mesh>>,
	old_walls: Query<Entity, With<WallDisplay>>,
) {
	for entity in &old_walls {
		commands.entity(entity).despawn();
	}

	if house.mesh().is_empty() {
		return;
	}

	commands.spawn((
		Name::new("house-walls"),
		WallDisplay,
		Mesh3d(meshes.add(to_bevy_mesh(house.mesh()))),
		MeshMaterial3d(wall_materials.committed.clone()),
	));
}

fn rebuild_preview_entity(
	mut commands: Commands,
	preview: Res<EditPreview>,
	wall_materials: Res<WallMaterials>,
	mut meshes: ResMut<Assets<Mesh>>,
	old_previews: Query<Entity, With<PreviewDisplay>>,
) {
	for entity in &old_previews {
		commands.entity(entity).despawn();
	}

	let Some(candidate) = preview.valid_candidate() else {
		return;
	};
	commands.spawn((
		Name::new("wall-preview"),
		PreviewDisplay,
		Mesh3d(meshes.add(to_bevy_mesh(candidate.mesh()))),
		MeshMaterial3d(wall_materials.preview.clone()),
	));
}

/// The preview mesh is the whole candidate house, so it would overlap the committed
/// walls and z-fight with them.
fn hide_committed_walls_while_previewing(
	preview: Res<EditPreview>,
	mut walls: Query<&mut Visibility, With<WallDisplay>>,
) {
	let visibility = if preview.valid_candidate().is_some() {
		Visibility::Hidden
	} else {
		Visibility::Inherited
	};
	for mut wall_visibility in &mut walls {
		wall_visibility.set_if_neq(visibility);
	}
}

/// Shows the proposed centre line even when the proposal is invalid and has no mesh.
fn draw_preview_line(preview: Res<EditPreview>, mut gizmos: Gizmos) {
	let Some(state) = preview.state.as_ref() else {
		return;
	};
	let color = if state.outcome.is_ok() {
		Color::srgb(0.2, 0.85, 0.3)
	} else {
		Color::srgb(0.9, 0.2, 0.2)
	};
	let to_bevy = |position: Vec2| graph_to_bevy(position.extend(PREVIEW_LINE_HEIGHT_METRES));
	gizmos.line(
		to_bevy(state.proposal.start),
		to_bevy(state.proposal.end),
		color,
	);
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

#[cfg(test)]
#[path = "tests/wall_render.rs"]
mod tests;
