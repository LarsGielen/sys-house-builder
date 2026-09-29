//! Turns footprint cells and their solid elevation intervals into boundary triangles.
//!
//! A vertical face appears wherever a cell is solid and its neighbour across an edge is not.
//! Every solid interval of a cell gets a top and a bottom face. Faces on the vertical line
//! above a vertex are all split at the same elevations, so the welded mesh has no T-junctions.

use std::collections::HashMap;

use glam::{DVec2, DVec3};

use crate::footprint::{Cell, CellKind, EdgeKind, Footprint};
use crate::intervals::{ELEVATION_TOLERANCE, Intervals, Levels};
use crate::output::MeshSection;
use crate::snapshot::Snapshot;
use crate::triangulate::triangulate;
use crate::{MeshError, MeshSettings, SurfaceKind, SurfaceSource, SurfaceTag, WallMesh};

pub(crate) fn extrude(
	snapshot: &Snapshot,
	footprint: &Footprint,
	settings: &MeshSettings,
) -> Result<WallMesh, MeshError> {
	let levels = Levels::new(footprint.cells.iter().flat_map(|cell| cell.solid.bounds()));
	let solids: Vec<Intervals> = footprint
		.cells
		.iter()
		.map(|cell| cell.solid.snapped(&levels))
		.collect();

	let mut neighbours: HashMap<(usize, usize), usize> = HashMap::new();
	for (index, cell) in footprint.cells.iter().enumerate() {
		for edge in cell.edge_vertices() {
			if let Some(&other) = neighbours.get(&edge) {
				return Err(MeshError::FootprintOverlap {
					first: footprint.cells[other].source(snapshot),
					second: cell.source(snapshot),
				});
			}
			neighbours.insert(edge, index);
		}
	}

	let mut breaks: Vec<Vec<f64>> = vec![Vec::new(); footprint.vertices.len()];
	for (cell, solid) in footprint.cells.iter().zip(&solids) {
		for &vertex in &cell.boundary {
			breaks[vertex].extend(solid.bounds());
		}
	}
	for levels in &mut breaks {
		levels.sort_by(f64::total_cmp);
		levels.dedup();
	}

	let faces = Faces {
		snapshot,
		footprint,
		breaks: &breaks,
	};
	let mut mesh = MeshBuilder::new(settings.max_triangles);
	for (index, cell) in footprint.cells.iter().enumerate() {
		let solid = &solids[index];
		for (k, (from, to)) in cell.edge_vertices().enumerate() {
			let neighbour = neighbours.get(&(to, from)).copied();
			let exposed = match neighbour {
				Some(other) => solid.difference(&solids[other]),
				None => solid.clone(),
			};
			for (bottom, top) in exposed.iter() {
				let tag = faces.vertical_tag(cell.edges[k], neighbour, bottom, top);
				faces.vertical(&mut mesh, tag, cell.edges[k], from, to, bottom, top)?;
			}
		}

		let points: Vec<DVec2> = cell
			.boundary
			.iter()
			.map(|&vertex| footprint.vertices[vertex])
			.collect();
		let triangles = triangulate(&points).ok_or(MeshError::TriangulationFailed {
			source: cell.source(snapshot),
		})?;
		for (bottom, top) in solid.iter() {
			mesh.horizontal(faces.top_tag(cell, top), &points, &triangles, top, true)?;
			mesh.horizontal(
				faces.bottom_tag(cell, bottom),
				&points,
				&triangles,
				bottom,
				false,
			)?;
		}
	}
	Ok(mesh.finish())
}

struct Faces<'a> {
	snapshot: &'a Snapshot,
	footprint: &'a Footprint,
	breaks: &'a [Vec<f64>],
}

impl Faces<'_> {
	#[allow(clippy::too_many_arguments)]
	fn vertical(
		&self,
		mesh: &mut MeshBuilder,
		tag: SurfaceTag,
		kind: EdgeKind,
		from: usize,
		to: usize,
		bottom: f64,
		top: f64,
	) -> Result<(), MeshError> {
		let (start, end) = (self.footprint.vertices[from], self.footprint.vertices[to]);
		// Cells wind counterclockwise, so the outside lies to the right of each edge.
		let flat = (end - start).perp().normalize() * -1.0;
		let normal = |point: DVec2| match kind {
			EdgeKind::Side {
				center: Some(center),
				..
			} => {
				let radial = (point - center).normalize();
				if radial.dot(flat) < 0.0 {
					-radial
				} else {
					radial
				}
			}
			_ => flat,
		};
		let column = |vertex: usize| -> Vec<f64> {
			std::iter::once(bottom)
				.chain(
					self.breaks[vertex]
						.iter()
						.copied()
						.filter(|&level| level > bottom && level < top),
				)
				.chain(std::iter::once(top))
				.collect()
		};
		mesh.ladder(
			tag,
			(start, normal(start), &column(from)),
			(end, normal(end), &column(to)),
		)
	}

	fn vertical_tag(
		&self,
		kind: EdgeKind,
		neighbour: Option<usize>,
		bottom: f64,
		top: f64,
	) -> SurfaceTag {
		let walls = &self.snapshot.walls;
		match kind {
			EdgeKind::Side { wall, side, .. } => SurfaceTag {
				kind: SurfaceKind::Side(side),
				source: SurfaceSource::Wall(walls[wall].handle),
			},
			EdgeKind::Bevel { node } => SurfaceTag {
				kind: SurfaceKind::Bevel,
				source: SurfaceSource::Junction(self.snapshot.nodes[node].id),
			},
			EdgeKind::CrossSection {
				node: Some(node), ..
			} => SurfaceTag {
				kind: SurfaceKind::JunctionStep,
				source: SurfaceSource::Junction(self.snapshot.nodes[node].id),
			},
			EdgeKind::CrossSection { wall, node: None } => {
				// Inside a wall, a cross-section is exposed only where the next stretch has an
				// opening; at a free end it is the wall's end cap.
				let middle = (bottom + top) * 0.5;
				let opening = neighbour.and_then(|other| match &self.footprint.cells[other].kind {
					CellKind::Body {
						wall: other_wall,
						openings,
					} => openings
						.iter()
						.map(|&index| &walls[*other_wall].openings[index])
						.find(|opening| opening.bottom <= middle && middle <= opening.top),
					CellKind::Core { .. } => None,
				});
				match opening {
					Some(opening) => SurfaceTag {
						kind: SurfaceKind::Jamb,
						source: SurfaceSource::Opening(opening.id),
					},
					None => SurfaceTag {
						kind: SurfaceKind::EndCap,
						source: SurfaceSource::Wall(walls[wall].handle),
					},
				}
			}
		}
	}

	fn top_tag(&self, cell: &Cell, level: f64) -> SurfaceTag {
		self.horizontal_tag(
			cell,
			level,
			SurfaceKind::Top,
			SurfaceKind::Sill,
			|opening| opening.bottom,
		)
	}

	fn bottom_tag(&self, cell: &Cell, level: f64) -> SurfaceTag {
		self.horizontal_tag(
			cell,
			level,
			SurfaceKind::Bottom,
			SurfaceKind::Head,
			|opening| opening.top,
		)
	}

	/// Tags a horizontal face as a reveal when an opening in the cell starts or ends at its level.
	fn horizontal_tag(
		&self,
		cell: &Cell,
		level: f64,
		plain: SurfaceKind,
		reveal: SurfaceKind,
		opening_level: impl Fn(&crate::snapshot::OpeningSnapshot) -> f64,
	) -> SurfaceTag {
		if let CellKind::Body { wall, openings } = &cell.kind {
			let wall = &self.snapshot.walls[*wall];
			if let Some(opening) = openings
				.iter()
				.map(|&index| &wall.openings[index])
				.find(|opening| (opening_level(opening) - level).abs() <= ELEVATION_TOLERANCE)
			{
				return SurfaceTag {
					kind: reveal,
					source: SurfaceSource::Opening(opening.id),
				};
			}
		}
		SurfaceTag {
			kind: plain,
			source: cell.source(self.snapshot),
		}
	}
}

struct MeshBuilder {
	mesh: WallMesh,
	max_triangles: usize,
}

impl MeshBuilder {
	fn new(max_triangles: usize) -> Self {
		Self {
			mesh: WallMesh::default(),
			max_triangles,
		}
	}

	fn finish(self) -> WallMesh {
		self.mesh
	}

	/// Appends one face's vertices and triangles, extending the previous section when its tag
	/// matches.
	fn face(
		&mut self,
		tag: SurfaceTag,
		vertices: impl Iterator<Item = (DVec3, DVec3)>,
		triangles: impl Iterator<Item = [usize; 3]>,
	) -> Result<(), MeshError> {
		let base = self.mesh.positions.len();
		for (position, normal) in vertices {
			self.mesh.positions.push(position.as_vec3());
			self.mesh.normals.push(normal.as_vec3());
		}
		if self.mesh.positions.len() > u32::MAX as usize {
			return Err(MeshError::OutputLimitExceeded);
		}
		let start = self.mesh.indices.len();
		for triangle in triangles {
			if self.mesh.indices.len() / 3 >= self.max_triangles {
				return Err(MeshError::OutputLimitExceeded);
			}
			self.mesh
				.indices
				.extend(triangle.map(|index| (base + index) as u32));
		}
		let end = self.mesh.indices.len();
		match self.mesh.sections.last_mut() {
			Some(section) if section.tag == tag && section.indices.end == start => {
				section.indices.end = end;
			}
			_ => self.mesh.sections.push(MeshSection {
				tag,
				indices: start..end,
			}),
		}
		Ok(())
	}

	/// A vertical face between two columns of elevations, zipped into triangles from the bottom.
	fn ladder(
		&mut self,
		tag: SurfaceTag,
		(start, start_normal, start_levels): (DVec2, DVec2, &[f64]),
		(end, end_normal, end_levels): (DVec2, DVec2, &[f64]),
	) -> Result<(), MeshError> {
		let vertices = start_levels
			.iter()
			.map(|&level| (start.extend(level), start_normal.extend(0.0)))
			.chain(
				end_levels
					.iter()
					.map(|&level| (end.extend(level), end_normal.extend(0.0))),
			);
		let offset = start_levels.len();
		let mut triangles = Vec::with_capacity(start_levels.len() + end_levels.len());
		let (mut i, mut j) = (0, 0);
		while i + 1 < start_levels.len() || j + 1 < end_levels.len() {
			let advance_end = i + 1 == start_levels.len()
				|| (j + 1 < end_levels.len() && end_levels[j + 1] <= start_levels[i + 1]);
			if advance_end {
				triangles.push([i, offset + j, offset + j + 1]);
				j += 1;
			} else {
				triangles.push([i, offset + j, i + 1]);
				i += 1;
			}
		}
		self.face(tag, vertices, triangles.into_iter())
	}

	fn horizontal(
		&mut self,
		tag: SurfaceTag,
		points: &[DVec2],
		triangles: &[[usize; 3]],
		level: f64,
		upward: bool,
	) -> Result<(), MeshError> {
		let normal = if upward { DVec3::Z } else { DVec3::NEG_Z };
		self.face(
			tag,
			points.iter().map(|point| (point.extend(level), normal)),
			triangles
				.iter()
				.map(|&[a, b, c]| if upward { [a, b, c] } else { [a, c, b] }),
		)
	}
}
