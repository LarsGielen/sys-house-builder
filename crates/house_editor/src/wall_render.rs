use bevy::{
	asset::RenderAssetUsages,
	mesh::{Indices, PrimitiveTopology},
	prelude::*,
};
use wall_graph::{OpeningSpec, Wall, WallGraph};
use wall_mesh::{SurfaceSource, WallMesh};

use crate::{
	coordinates::graph_to_bevy,
	edit::{EditPreview, Proposal, WallProposal},
	house::House,
	opening_area::{OpeningArea, opening_area},
};

// Lifts the line off the ground so it is not hidden by the floor.
const PREVIEW_LINE_HEIGHT_METRES: f32 = 0.02;
// Keeps the opening outline off the wall faces it is drawn around.
const OUTLINE_LIFT_METRES: f32 = 0.01;
const SAMPLE_DEVIATION_METRES: f32 = 0.01;
// Keeps the deletion tint in front of the surfaces it covers.
const REMOVAL_LIFT_METRES: f32 = 0.003;
const REMOVAL_COLOR: Color = Color::srgb(0.9, 0.2, 0.2);

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
				rebuild_removal_entity.run_if(resource_changed::<EditPreview>),
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

/// Marks the entity that tints the surfaces a previewed deletion would remove.
#[derive(Component)]
struct RemovalDisplay;

#[derive(Resource)]
struct WallMaterials {
	committed: Handle<StandardMaterial>,
	preview: Handle<StandardMaterial>,
	removal: Handle<StandardMaterial>,
}

fn setup_wall_materials(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
	commands.insert_resource(WallMaterials {
		committed: materials.add(Color::srgb(0.85, 0.82, 0.78)),
		preview: materials.add(Color::srgb(0.55, 0.8, 0.95)),
		removal: materials.add(StandardMaterial {
			base_color: REMOVAL_COLOR.with_alpha(0.8),
			alpha_mode: AlphaMode::Blend,
			..default()
		}),
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

	let Some(candidate) = preview.candidate_to_display() else {
		return;
	};
	// A mesh with no vertices has nothing to show and upsets the renderer's mesh allocator.
	if candidate.mesh().is_empty() {
		return;
	}
	commands.spawn((
		Name::new("wall-preview"),
		PreviewDisplay,
		Mesh3d(meshes.add(to_bevy_mesh(candidate.mesh()))),
		MeshMaterial3d(wall_materials.preview.clone()),
	));
}

/// Tints the surfaces a previewed deletion would remove, drawn just above the committed walls.
fn rebuild_removal_entity(
	mut commands: Commands,
	preview: Res<EditPreview>,
	house: Res<House>,
	wall_materials: Res<WallMaterials>,
	mut meshes: ResMut<Assets<Mesh>>,
	old_removals: Query<Entity, With<RemovalDisplay>>,
) {
	for entity in &old_removals {
		commands.entity(entity).despawn();
	}

	let Some(state) = preview.state.as_ref() else {
		return;
	};
	let sources = removed_sources(house.graph(), &state.proposal);
	if sources.is_empty() {
		return;
	}
	let mesh = tinted_mesh(house.mesh(), &sources);
	if mesh.indices().is_none_or(|indices| indices.is_empty()) {
		return;
	}
	commands.spawn((
		Name::new("removal-preview"),
		RemovalDisplay,
		Mesh3d(meshes.add(mesh)),
		MeshMaterial3d(wall_materials.removal.clone()),
	));
}

/// Everything in the committed mesh that a deletion removes: the wall or opening itself, and
/// the openings of a deleted wall.
fn removed_sources(graph: &WallGraph, proposal: &Proposal) -> Vec<SurfaceSource> {
	match proposal {
		Proposal::WallRemoval(wall) => std::iter::once(SurfaceSource::Wall(*wall))
			.chain(
				graph
					.openings()
					.filter(|(_, opening)| opening.wall == *wall)
					.map(|(id, _)| SurfaceSource::Opening(id)),
			)
			.collect(),
		Proposal::OpeningRemoval(opening) => vec![SurfaceSource::Opening(*opening)],
		_ => Vec::new(),
	}
}

/// The triangles of `sources`, nudged outward so they do not fight the committed surfaces.
fn tinted_mesh(wall_mesh: &WallMesh, sources: &[SurfaceSource]) -> Mesh {
	let positions: Vec<[f32; 3]> = wall_mesh
		.positions()
		.iter()
		.zip(wall_mesh.normals())
		.map(|(&position, &normal)| {
			graph_to_bevy(position + normal * REMOVAL_LIFT_METRES).to_array()
		})
		.collect();
	let normals: Vec<[f32; 3]> = wall_mesh
		.normals()
		.iter()
		.map(|&normal| graph_to_bevy(normal).to_array())
		.collect();
	let indices: Vec<u32> = wall_mesh
		.sections()
		.iter()
		.filter(|section| sources.contains(&section.tag.source))
		.flat_map(|section| wall_mesh.indices()[section.indices.clone()].iter().copied())
		.collect();

	Mesh::new(
		PrimitiveTopology::TriangleList,
		RenderAssetUsages::default(),
	)
	.with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
	.with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
	.with_inserted_indices(Indices::U32(indices))
}

/// The preview mesh is the whole candidate house, so it would overlap the committed
/// walls and z-fight with them.
fn hide_committed_walls_while_previewing(
	preview: Res<EditPreview>,
	mut walls: Query<&mut Visibility, With<WallDisplay>>,
) {
	let visibility = if preview.candidate_to_display().is_some() {
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
		Proposal::Opening(opening) => draw_opening_proposal(
			house.graph(),
			opening.wall,
			&opening.spec,
			color,
			&mut gizmos,
		),
		Proposal::OpeningEdit(edit) => {
			if let Some(opening) = house.graph().opening(edit.opening) {
				draw_opening_proposal(house.graph(), opening.wall, &edit.spec, color, &mut gizmos)
			}
		}
		// A valid change is visible in the preview mesh; a rejected one has none.
		Proposal::WallDimensions(edit) if state.outcome.is_err() => {
			draw_wall_line(house.graph(), edit.wall, color, &mut gizmos)
		}
		Proposal::WallDimensions(_) => {}
		Proposal::WallRemoval(wall) => draw_wall_removal(house.graph(), *wall, &mut gizmos),
		Proposal::OpeningRemoval(opening) => {
			if let Some(opening) = house.graph().opening(*opening) {
				draw_opening_proposal(
					house.graph(),
					opening.wall,
					&opening.spec,
					REMOVAL_COLOR,
					&mut gizmos,
				)
			}
		}
	}
}

/// Marks everything that deleting the wall takes with it: the wall and its openings.
fn draw_wall_removal(graph: &WallGraph, wall: Wall, gizmos: &mut Gizmos) {
	draw_wall_line(graph, wall, REMOVAL_COLOR, gizmos);
	for (_, opening) in graph.openings().filter(|(_, opening)| opening.wall == wall) {
		draw_opening_proposal(graph, wall, &opening.spec, REMOVAL_COLOR, gizmos);
	}
}

/// The wall's centre line along its top.
fn draw_wall_line(graph: &WallGraph, wall: Wall, color: Color, gizmos: &mut Gizmos) {
	let (Some(dimensions), Ok(points)) = (
		graph.wall_dimensions(wall),
		graph.sample_wall(wall, SAMPLE_DEVIATION_METRES),
	) else {
		return;
	};
	let top = dimensions.height + OUTLINE_LIFT_METRES;
	gizmos.linestrip(
		points
			.into_iter()
			.map(|point| graph_to_bevy(point.extend(top))),
		color,
	);
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
	wall: Wall,
	spec: &OpeningSpec,
	color: Color,
	gizmos: &mut Gizmos,
) {
	let (Some(area), Some(dimensions)) =
		(opening_area(graph, wall, spec), graph.wall_dimensions(wall))
	else {
		return;
	};
	let OpeningArea {
		start,
		end,
		tangent,
		..
	} = area;

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
