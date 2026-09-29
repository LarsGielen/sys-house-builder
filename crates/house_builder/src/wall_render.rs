//! Adapts engine-independent wall meshes to Bevy.

use std::collections::{HashMap, HashSet};

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use wall_mesh::{SurfaceTag, WallMesh};

/// Maps graph space (floor plan X and Y, elevation Z) into Bevy's Y-up space.
///
/// This is a rotation about the X axis rather than an axis swap, so triangle winding and
/// handedness are preserved. Graph +Y becomes Bevy's forward, -Z.
pub fn graph_to_world(point: Vec3) -> Vec3 {
	Vec3::new(point.x, point.z, -point.y)
}

/// Groups triangles by their surface tag so the demo can color them independently.
pub fn tagged_bevy_meshes(mesh: &WallMesh) -> Vec<(SurfaceTag, Mesh)> {
	let mut groups: Vec<(SurfaceTag, Vec<u32>)> = Vec::new();
	let mut group_indices = HashMap::new();
	for section in mesh.sections() {
		let group = *group_indices.entry(section.tag).or_insert_with(|| {
			groups.push((section.tag, Vec::new()));
			groups.len() - 1
		});
		groups[group]
			.1
			.extend_from_slice(&mesh.indices()[section.indices.clone()]);
	}
	groups
		.into_iter()
		.map(|(tag, indices)| (tag, compact_mesh(mesh, &indices)))
		.collect()
}

fn compact_mesh(mesh: &WallMesh, indices: &[u32]) -> Mesh {
	let mut positions = Vec::new();
	let mut normals = Vec::new();
	let mut compact_indices = Vec::with_capacity(indices.len());
	let mut vertex_indices = HashMap::new();
	for &index in indices {
		let compact = *vertex_indices.entry(index).or_insert_with(|| {
			positions.push(graph_to_world(mesh.positions()[index as usize]).to_array());
			normals.push(graph_to_world(mesh.normals()[index as usize]).to_array());
			(positions.len() - 1) as u32
		});
		compact_indices.push(compact);
	}
	Mesh::new(
		PrimitiveTopology::TriangleList,
		RenderAssetUsages::default(),
	)
	.with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
	.with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
	.with_inserted_indices(Indices::U32(compact_indices))
}

/// Unique triangle edges in world coordinates, including triangulation diagonals.
pub fn wire_edges(mesh: &WallMesh) -> Vec<(Vec3, Vec3)> {
	let mut seen = HashSet::new();
	let mut edges = Vec::new();
	for triangle in mesh.indices().chunks_exact(3) {
		for (from, to) in [(0, 1), (1, 2), (2, 0)] {
			let first = mesh.positions()[triangle[from] as usize];
			let second = mesh.positions()[triangle[to] as usize];
			let first_bits = first.to_array().map(f32::to_bits);
			let second_bits = second.to_array().map(f32::to_bits);
			let key = if first_bits < second_bits {
				(first_bits, second_bits)
			} else {
				(second_bits, first_bits)
			};
			if seen.insert(key) {
				edges.push((graph_to_world(first), graph_to_world(second)));
			}
		}
	}
	edges
}

#[cfg(test)]
mod tests {
	use wall_graph::WallGraph;
	use wall_mesh::{MeshSettings, generate};

	use super::*;

	#[test]
	fn the_axis_mapping_is_a_proper_rotation() {
		let x = graph_to_world(Vec3::X);
		let y = graph_to_world(Vec3::Y);
		let z = graph_to_world(Vec3::Z);
		assert_eq!(z, Vec3::Y);
		// A right-handed basis stays right-handed, so outward winding survives the mapping.
		assert_eq!(x.cross(y), z);
	}

	#[test]
	fn tagged_meshes_cover_every_triangle() {
		let mut graph = WallGraph::new();
		graph.add_wall(Vec2::ZERO, Vec2::new(2.0, 0.0)).unwrap();
		let mesh = generate(&graph, &MeshSettings::default()).unwrap();
		let tagged = tagged_bevy_meshes(&mesh);
		let index_count: usize = tagged
			.iter()
			.map(|(_, part)| match part.indices().unwrap() {
				Indices::U32(indices) => indices.len(),
				Indices::U16(indices) => indices.len(),
			})
			.sum();
		assert_eq!(index_count, mesh.indices().len());
		assert!(!wire_edges(&mesh).is_empty());
	}
}
