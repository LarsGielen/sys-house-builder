use std::{fmt::Display, ops::RangeInclusive};

use bevy::prelude::{Mut, Resource, Vec2};
use wall_graph::{
	MIN_OPENING_GAP, OpeningId, OpeningSpec, Wall, WallDimensions, WallError, WallGraph,
};
use wall_mesh::{MeshError, MeshSettings, WallMesh};

use crate::opening_category::{OpeningCategories, OpeningCategory};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditError {
	Graph(WallError),
	Mesh(MeshError),
}

impl Display for EditError {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			EditError::Graph(error) => error.fmt(formatter),
			EditError::Mesh(error) => error.fmt(formatter),
		}
	}
}

impl std::error::Error for EditError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Self::Graph(error) => Some(error),
			Self::Mesh(error) => Some(error),
		}
	}
}

impl From<WallError> for EditError {
	fn from(value: WallError) -> Self {
		EditError::Graph(value)
	}
}

impl From<MeshError> for EditError {
	fn from(value: MeshError) -> Self {
		EditError::Mesh(value)
	}
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallProposal {
	pub start: Vec2,
	pub end: Vec2,
	pub dimensions: WallDimensions,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OpeningProposal {
	pub wall: Wall,
	pub spec: OpeningSpec,
	pub category: OpeningCategory,
}

/// New dimensions or placement for an existing opening, which keeps its identity and category.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OpeningEditProposal {
	pub opening: OpeningId,
	pub spec: OpeningSpec,
}

/// New height and thickness for an existing wall piece.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallDimensionsProposal {
	pub wall: Wall,
	pub dimensions: WallDimensions,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Proposal {
	Wall(WallProposal),
	WallDimensions(WallDimensionsProposal),
	/// Deleting a wall, which also deletes its openings.
	WallRemoval(Wall),
	OpeningRemoval(OpeningId),
	Opening(OpeningProposal),
	OpeningEdit(OpeningEditProposal),
}

/// A complete proposed document: the graph, the mesh generated from it, and the editor's
/// own metadata about it. They are only ever built together, so they cannot disagree.
#[derive(Default)]
pub struct EditCandidate {
	graph: WallGraph,
	mesh: WallMesh,
	categories: OpeningCategories,
}

impl EditCandidate {
	pub fn graph(&self) -> &WallGraph {
		&self.graph
	}

	pub fn mesh(&self) -> &WallMesh {
		&self.mesh
	}

	pub fn categories(&self) -> &OpeningCategories {
		&self.categories
	}

	/// The one place a candidate is assembled. Metadata for openings the graph dropped is
	/// removed here, so edits that remove openings need no bookkeeping of their own.
	fn build(graph: WallGraph, mut categories: OpeningCategories) -> Result<Self, EditError> {
		categories.retain_existing(&graph);
		let mesh = wall_mesh::generate(&graph, &mesh_settings())?;
		Ok(Self {
			graph,
			mesh,
			categories,
		})
	}
}

pub fn propose_wall(
	committed: &EditCandidate,
	wall: &WallProposal,
) -> Result<EditCandidate, EditError> {
	let mut candidate_graph = committed.graph().clone();
	candidate_graph.add_wall_with_dimensions(wall.start, wall.end, wall.dimensions)?;
	EditCandidate::build(candidate_graph, committed.categories().clone())
}

/// Adding an opening can pass the graph's checks and still fail meshing near a junction,
/// so the mesh is generated before the candidate is accepted.
pub fn propose_opening(
	committed: &EditCandidate,
	opening: &OpeningProposal,
) -> Result<EditCandidate, EditError> {
	propose_opening_with_id(committed, opening).map(|(candidate, _)| candidate)
}

fn propose_opening_with_id(
	committed: &EditCandidate,
	opening: &OpeningProposal,
) -> Result<(EditCandidate, OpeningId), EditError> {
	let mut candidate_graph = committed.graph().clone();
	let id = candidate_graph.add_opening(opening.wall, opening.spec)?;
	let mut categories = committed.categories().clone();
	categories.insert(id, opening.category);
	Ok((EditCandidate::build(candidate_graph, categories)?, id))
}

fn mesh_settings() -> MeshSettings {
	MeshSettings::default()
}

/// Path distances between which an opening on `wall` passes both the graph's and the mesh's
/// junction clearance, or `None` when the wall has no such stretch.
pub fn opening_bounds(graph: &WallGraph, wall: Wall) -> Option<RangeInclusive<f64>> {
	let graph_bounds = graph.opening_bounds(wall)?;
	let mesh_bounds = wall_mesh::opening_bounds(graph, wall, &mesh_settings()).ok()?;
	Some(
		graph_bounds.start().max(*mesh_bounds.start())..=graph_bounds.end().min(*mesh_bounds.end()),
	)
}

pub fn propose_wall_dimensions(
	committed: &EditCandidate,
	edit: &WallDimensionsProposal,
) -> Result<EditCandidate, EditError> {
	let mut candidate_graph = committed.graph().clone();
	candidate_graph.set_wall_dimensions(edit.wall, edit.dimensions)?;
	EditCandidate::build(candidate_graph, committed.categories().clone())
}

/// Removing a wall removes the openings on it; `build` drops their categories with them.
pub fn propose_wall_removal(
	committed: &EditCandidate,
	wall: Wall,
) -> Result<EditCandidate, EditError> {
	let mut candidate_graph = committed.graph().clone();
	candidate_graph.remove_wall(wall)?;
	EditCandidate::build(candidate_graph, committed.categories().clone())
}

pub fn propose_opening_edit(
	committed: &EditCandidate,
	edit: &OpeningEditProposal,
) -> Result<EditCandidate, EditError> {
	let mut candidate_graph = committed.graph().clone();
	candidate_graph.set_opening(edit.opening, edit.spec)?;
	EditCandidate::build(candidate_graph, committed.categories().clone())
}

pub fn propose_opening_removal(
	committed: &EditCandidate,
	opening: OpeningId,
) -> Result<EditCandidate, EditError> {
	let mut candidate_graph = committed.graph().clone();
	candidate_graph.remove_opening(opening)?;
	EditCandidate::build(candidate_graph, committed.categories().clone())
}

/// Copies an opening, with its category, to the nearest position beside an opening on the
/// same wall where the copy is valid. Returns the candidate and the copy's ID.
///
/// `gap` is kept between the copy and the opening it sits beside, raised to the graph's
/// minimum when smaller.
pub fn propose_opening_duplicate(
	committed: &EditCandidate,
	original: OpeningId,
	gap: f32,
) -> Result<(EditCandidate, OpeningId), EditError> {
	let graph = committed.graph();
	let source = graph.opening(original).ok_or(WallError::UnknownOpening)?;
	let category = committed.categories().get(original).unwrap_or_default();
	let gap = f64::from(gap).max(MIN_OPENING_GAP);
	let half_width = f64::from(source.spec.width) / 2.0;

	let mut centers: Vec<f64> = graph
		.openings()
		.filter(|(_, opening)| opening.wall == source.wall)
		.flat_map(|(_, opening)| {
			let half = f64::from(opening.spec.width) / 2.0;
			[
				opening.spec.center_distance + half + gap + half_width,
				opening.spec.center_distance - half - gap - half_width,
			]
		})
		.collect();
	centers.sort_by(|a, b| {
		(a - source.spec.center_distance)
			.abs()
			.total_cmp(&(b - source.spec.center_distance).abs())
	});

	let mut first_error = None;
	for center_distance in centers {
		let proposal = OpeningProposal {
			wall: source.wall,
			spec: OpeningSpec {
				center_distance,
				..source.spec
			},
			category,
		};
		match propose_opening_with_id(committed, &proposal) {
			Ok(duplicate) => return Ok(duplicate),
			Err(error) => {
				first_error.get_or_insert(error);
			}
		}
	}
	Err(first_error.unwrap_or(EditError::Graph(WallError::InvalidOpening)))
}

/// Why the last edit that had no preview was refused, such as a duplicate with no room.
/// The next successful edit or a new selection clears it.
#[derive(Resource, Default)]
pub struct EditNotice {
	pub message: Option<String>,
}

pub struct PreviewState {
	pub house_revision: u64,
	pub proposal: Proposal,
	pub outcome: Result<EditCandidate, EditError>,
}

/// The edit being proposed and what it would produce, shared between the tool that
/// proposes it and the views that display it. Committing takes exactly this candidate,
/// so the preview and the committed house cannot disagree.
#[derive(Resource, Default)]
pub struct EditPreview {
	pub state: Option<PreviewState>,
}

impl EditPreview {
	pub fn matches(&self, proposal: Proposal, house_revision: u64) -> bool {
		self.state.as_ref().is_some_and(|state| {
			state.proposal == proposal && state.house_revision == house_revision
		})
	}

	/// Refreshes only when the proposal or its base document changed. Avoiding redundant
	/// writes also avoids rebuilding the rendered preview through Bevy's change tracking.
	pub fn update(
		mut preview: Mut<Self>,
		proposal: Proposal,
		house_revision: u64,
		committed: &EditCandidate,
	) {
		if preview.matches(proposal, house_revision) {
			return;
		}
		let outcome = match proposal {
			Proposal::Wall(wall) => propose_wall(committed, &wall),
			Proposal::WallDimensions(edit) => propose_wall_dimensions(committed, &edit),
			Proposal::WallRemoval(wall) => propose_wall_removal(committed, wall),
			Proposal::OpeningRemoval(opening) => propose_opening_removal(committed, opening),
			Proposal::Opening(opening) => propose_opening(committed, &opening),
			Proposal::OpeningEdit(edit) => propose_opening_edit(committed, &edit),
		};
		preview.state = Some(PreviewState {
			house_revision,
			proposal,
			outcome,
		});
	}

	/// Takes only the expected edit, validated against the current document. Invalid or
	/// stale candidates remain available to display their reason or to be refreshed.
	pub fn take_valid(&mut self, proposal: Proposal, house_revision: u64) -> Option<EditCandidate> {
		if !self.matches(proposal, house_revision) || self.valid_candidate().is_none() {
			return None;
		}
		self.state.take()?.outcome.ok()
	}

	/// Tools use these to clear only their own proposal, so that leaving one tool does not
	/// erase the preview of the tool that was just entered.
	pub fn holds_wall_proposal(&self) -> bool {
		matches!(
			self.state,
			Some(PreviewState {
				proposal: Proposal::Wall(_),
				..
			})
		)
	}

	/// A draft built by typing into a field, which Escape in the field discards.
	pub fn holds_field_draft(&self) -> bool {
		self.holds_opening_edit() || self.holds_wall_dimensions()
	}

	/// A deletion the delete tool previews under the pointer.
	pub fn holds_removal(&self) -> bool {
		matches!(
			self.state,
			Some(PreviewState {
				proposal: Proposal::WallRemoval(_) | Proposal::OpeningRemoval(_),
				..
			})
		)
	}

	pub fn holds_wall_removal(&self) -> bool {
		matches!(
			self.state,
			Some(PreviewState {
				proposal: Proposal::WallRemoval(_),
				..
			})
		)
	}

	pub fn holds_wall_dimensions(&self) -> bool {
		matches!(
			self.state,
			Some(PreviewState {
				proposal: Proposal::WallDimensions(_),
				..
			})
		)
	}

	pub fn holds_opening_edit(&self) -> bool {
		matches!(
			self.state,
			Some(PreviewState {
				proposal: Proposal::OpeningEdit(_),
				..
			})
		)
	}

	pub fn holds_opening_proposal(&self) -> bool {
		matches!(
			self.state,
			Some(PreviewState {
				proposal: Proposal::Opening(_),
				..
			})
		)
	}

	pub fn valid_candidate(&self) -> Option<&EditCandidate> {
		self.state.as_ref()?.outcome.as_ref().ok()
	}

	/// The candidate whose mesh replaces the committed walls on screen. A deletion is shown
	/// by tinting what would go instead, so the house around it stays in view.
	pub fn candidate_to_display(&self) -> Option<&EditCandidate> {
		if self.holds_removal() {
			None
		} else {
			self.valid_candidate()
		}
	}
}

#[cfg(test)]
#[path = "tests/edit.rs"]
mod tests;
