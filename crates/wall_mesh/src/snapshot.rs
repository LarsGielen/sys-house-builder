//! A deterministic, double-precision copy of the graph data that meshing reads.

use std::collections::HashMap;

use glam::DVec2;
use wall_graph::{OpeningId, Wall, WallGraph, WallNodeId};

use crate::path::Path;

pub(crate) struct Snapshot {
	pub(crate) nodes: Vec<NodeSnapshot>,
	pub(crate) walls: Vec<WallSnapshot>,
}

pub(crate) struct NodeSnapshot {
	pub(crate) id: WallNodeId,
	pub(crate) position: DVec2,
	/// Indices into [`Snapshot::walls`], counterclockwise by departure tangent.
	pub(crate) walls: Vec<usize>,
}

pub(crate) struct WallSnapshot {
	pub(crate) handle: Wall,
	pub(crate) path: Path,
	pub(crate) half_thickness: f64,
	pub(crate) height: f64,
	/// Index into [`Snapshot::nodes`].
	pub(crate) origin: usize,
	/// Sorted by path start, then bottom.
	pub(crate) openings: Vec<OpeningSnapshot>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct OpeningSnapshot {
	pub(crate) id: OpeningId,
	/// Path distances along the owning wall.
	pub(crate) start: f64,
	pub(crate) end: f64,
	/// Elevations above the floor.
	pub(crate) bottom: f64,
	pub(crate) top: f64,
}

impl Snapshot {
	/// Orders nodes by ID and walls by first appearance in those nodes' rings, so the result
	/// does not depend on the graph's hash-map iteration order.
	pub(crate) fn new(graph: &WallGraph) -> Self {
		let mut node_ids: Vec<WallNodeId> = graph.nodes().map(|(id, _)| id).collect();
		node_ids.sort();
		let node_index: HashMap<WallNodeId, usize> = node_ids
			.iter()
			.enumerate()
			.map(|(index, &id)| (id, index))
			.collect();

		let mut walls: Vec<WallSnapshot> = Vec::new();
		let mut wall_index: HashMap<Wall, usize> = HashMap::new();
		let mut nodes = Vec::with_capacity(node_ids.len());
		for &id in &node_ids {
			let incident = graph.node_walls(id).expect("listed node exists");
			let ring = incident
				.into_iter()
				.map(|wall| {
					*wall_index.entry(wall).or_insert_with(|| {
						let dimensions = graph.wall_dimensions(wall).expect("live wall");
						walls.push(WallSnapshot {
							handle: wall,
							path: Path::new(graph.wall_curve(wall).expect("live wall")),
							half_thickness: dimensions.thickness as f64 * 0.5,
							height: dimensions.height as f64,
							origin: node_index[&wall.origin()],
							openings: Vec::new(),
						});
						walls.len() - 1
					})
				})
				.collect();
			nodes.push(NodeSnapshot {
				id,
				position: graph.node_position(id).expect("listed node").as_dvec2(),
				walls: ring,
			});
		}

		for (id, opening) in graph.openings() {
			let half_width = opening.spec.width as f64 * 0.5;
			let bottom = opening.spec.bottom as f64;
			walls[wall_index[&opening.wall]]
				.openings
				.push(OpeningSnapshot {
					id,
					start: opening.spec.center_distance - half_width,
					end: opening.spec.center_distance + half_width,
					bottom,
					top: bottom + opening.spec.height as f64,
				});
		}
		for wall in &mut walls {
			wall.openings.sort_by(|a, b| {
				a.start
					.total_cmp(&b.start)
					.then(a.bottom.total_cmp(&b.bottom))
			});
		}
		Self { nodes, walls }
	}
}
