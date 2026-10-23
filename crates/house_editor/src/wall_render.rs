use bevy::{
	asset::RenderAssetUsages,
	mesh::{Indices, PrimitiveTopology},
	prelude::*,
};
use wall_graph::WallGraph;
use wall_mesh::WallMesh;

use crate::{
	coordinates::graph_to_bevy,
	edit::{EditPreview, OpeningProposal, Proposal, WallProposal},
	house::House,
};

// Lifts the line off the ground so it is not hidden by the floor.
const PREVIEW_LINE_HEIGHT_METRES: f32 = 0.02;
// Keeps the opening outline off the wall faces it is drawn around.
const OUTLINE_LIFT_METRES: f32 = 0.01;

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
				draw_preview_gizmos,
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

/// Shows the proposed wall line or opening position even when the proposal is invalid and
/// has no mesh.
fn draw_preview_gizmos(preview: Res<EditPreview>, house: Res<House>, mut gizmos: Gizmos) {
	let Some(state) = preview.state.as_ref() else {
		return;
	};
	let color = if state.outcome.is_ok() {
		Color::srgb(0.2, 0.85, 0.3)
	} else {
		Color::srgb(0.9, 0.2, 0.2)
	};
	match &state.proposal {
		Proposal::Wall(wall) => draw_wall_proposal(wall, color, &mut gizmos),
		Proposal::Opening(opening) => {
			draw_opening_proposal(house.graph(), opening, color, &mut gizmos)
		}
	}
}

fn draw_wall_proposal(proposal: &WallProposal, color: Color, gizmos: &mut Gizmos) {
	let to_bevy = |position: Vec2| graph_to_bevy(position.extend(PREVIEW_LINE_HEIGHT_METRES));
	gizmos.line(to_bevy(proposal.start), to_bevy(proposal.end), color);
}

/// Outlines the opening as a box through the host wall, so an invalid proposal, which has
/// no mesh, is still visible where it would go. Positions beyond the wall's ends are
/// clamped onto it.
fn draw_opening_proposal(
	graph: &WallGraph,
	proposal: &OpeningProposal,
	color: Color,
	gizmos: &mut Gizmos,
) {
	let spec = proposal.spec;
	let (Some(length), Some(dimensions)) = (
		graph.wall_length(proposal.wall),
		graph.wall_dimensions(proposal.wall),
	) else {
		return;
	};
	let half_width = f64::from(spec.width) / 2.0;
	let parameter_at = |distance: f64| (distance / length).clamp(0.0, 1.0) as f32;
	let center_parameter = parameter_at(spec.center_distance);
	let (Some(start), Some(end), Some(tangent)) = (
		graph.wall_position(
			proposal.wall,
			parameter_at(spec.center_distance - half_width),
		),
		graph.wall_position(
			proposal.wall,
			parameter_at(spec.center_distance + half_width),
		),
		graph.wall_tangent(proposal.wall, center_parameter),
	) else {
		return;
	};

	// Slightly outside the faces so the outline is not hidden by the wall surface.
	let offset = tangent.perp() * (dimensions.thickness / 2.0 + OUTLINE_LIFT_METRES);
	let top = spec.bottom + spec.height;
	for side in [offset, -offset] {
		let corner =
			|position: Vec2, elevation: f32| graph_to_bevy((position + side).extend(elevation));
		gizmos.linestrip(
			[
				corner(start, spec.bottom),
				corner(end, spec.bottom),
				corner(end, top),
				corner(start, top),
				corner(start, spec.bottom),
			],
			color,
		);
	}
	for (position, elevation) in [
		(start, spec.bottom),
		(end, spec.bottom),
		(end, top),
		(start, top),
	] {
		gizmos.line(
			graph_to_bevy((position + offset).extend(elevation)),
			graph_to_bevy((position - offset).extend(elevation)),
			color,
		);
	}
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
