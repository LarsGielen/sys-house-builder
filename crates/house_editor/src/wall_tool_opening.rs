use bevy::prelude::*;
use wall_graph::{OpeningSpec, WallGraph};
use wall_mesh::WallMesh;

use crate::{
	coordinates::bevy_to_graph,
	edit::{EditPreview, OpeningProposal, PreviewState, Proposal, opening_bounds, propose_opening},
	ground_pointer::GroundPointer,
	house::House,
	snapping::{SnapSettings, snapping_disabled},
	wall_tool_active::{Tool, tool_active},
	wall_tool_selection::pick_wall_hit,
};

const DEFAULT_WIDTH_METRES: f32 = 0.9;
const DEFAULT_HEIGHT_METRES: f32 = 2.1;
const DEFAULT_BOTTOM_METRES: f32 = 0.0;
// How close along the wall a snap candidate must be to take the opening's centre.
const SNAP_RADIUS_METRES: f64 = 0.15;

/// Places an opening on the wall under the pointer with a click.
pub struct WallToolOpeningPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WallToolOpeningSystems;

impl Plugin for WallToolOpeningPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<OpeningSettings>()
			.init_resource::<EditPreview>()
			.add_systems(
				PostUpdate,
				(
					update_opening_placement.run_if(tool_active(Tool::PlaceOpening)),
					cancel_opening_placement.run_if(not(tool_active(Tool::PlaceOpening))),
				)
					.in_set(WallToolOpeningSystems),
			);
	}
}

/// Size and elevation applied to every opening the tool proposes.
#[derive(Resource)]
pub struct OpeningSettings {
	width: f32,
	height: f32,
	bottom: f32,
}

impl Default for OpeningSettings {
	fn default() -> Self {
		Self {
			width: DEFAULT_WIDTH_METRES,
			height: DEFAULT_HEIGHT_METRES,
			bottom: DEFAULT_BOTTOM_METRES,
		}
	}
}

impl OpeningSettings {
	fn spec_at(&self, center_distance: f64) -> OpeningSpec {
		OpeningSpec {
			center_distance,
			width: self.width,
			bottom: self.bottom,
			height: self.height,
		}
	}
}

fn update_opening_placement(
	mouse: Res<ButtonInput<MouseButton>>,
	ground_pointer: Res<GroundPointer>,
	settings: Res<OpeningSettings>,
	snap_settings: Res<SnapSettings>,
	keyboard: Res<ButtonInput<KeyCode>>,
	mut house: ResMut<House>,
	mut preview: ResMut<EditPreview>,
) {
	let proposal = ground_pointer
		.ray
		.and_then(|ray| propose_at_ray(house.graph(), house.mesh(), &settings, ray))
		.map(|proposal| {
			if snapping_disabled(&keyboard) {
				proposal
			} else {
				snap_proposal(house.graph(), proposal, snap_settings.grid_spacing())
			}
		});

	// Reborrowing retains Bevy's change tracking until the helper actually writes.
	update_preview(proposal, &house, preview.reborrow());

	if mouse.just_pressed(MouseButton::Left)
		&& preview.valid_candidate().is_some()
		&& let Some(PreviewState {
			outcome: Ok(candidate),
			..
		}) = preview.state.take()
	{
		house.commit(candidate);
	}
}

/// Leaving the tool drops its preview, which would otherwise keep hiding the committed walls.
fn cancel_opening_placement(mut preview: ResMut<EditPreview>) {
	if preview.holds_opening_proposal() {
		preview.state = None;
	}
}

/// Hosts the opening on the wall the ray meets, centred where the ray meets it.
fn propose_at_ray(
	graph: &WallGraph,
	mesh: &WallMesh,
	settings: &OpeningSettings,
	ray: Ray3d,
) -> Option<OpeningProposal> {
	let hit = pick_wall_hit(
		graph,
		mesh,
		bevy_to_graph(ray.origin),
		bevy_to_graph(*ray.direction),
	)?;
	let (parameter, _) = graph.wall_closest_point(hit.wall, hit.position.truncate())?;
	// The graph places an opening at `center_distance / length` along the path, which is
	// how an arc's parameter becomes a distance as well.
	let center_distance = f64::from(parameter) * graph.wall_length(hit.wall)?;
	Some(OpeningProposal {
		wall: hit.wall,
		spec: settings.spec_at(center_distance),
	})
}

/// A place the opening's centre can snap to. Lower priorities win when several are in range.
struct SnapCandidate {
	center_distance: f64,
	priority: u8,
}

const STRUCTURAL_PRIORITY: u8 = 0;
const GRID_PRIORITY: u8 = 1;

/// Moves the opening's centre to a nearby wall end, the middle of a free span, or a grid
/// distance, in that order of priority. Everything is measured along the host wall's path
/// and stays within the stretch where junctions allow an opening.
fn snap_proposal(
	graph: &WallGraph,
	proposal: OpeningProposal,
	grid_spacing: f32,
) -> OpeningProposal {
	let (Some(length), Some(bounds)) = (
		graph.wall_length(proposal.wall),
		opening_bounds(graph, proposal.wall),
	) else {
		return proposal;
	};
	let width = f64::from(proposal.spec.width);
	let raw = proposal.spec.center_distance;
	let in_range = |center_distance: f64| (center_distance - raw).abs() <= SNAP_RADIUS_METRES;
	let centers = (bounds.start() + width / 2.0)..=(bounds.end() - width / 2.0);
	if centers.is_empty() {
		return proposal;
	}

	// Both ends of the allowed stretch count as obstacles, so an empty wall has one span and
	// snaps to the middle of what is allowed.
	let mut occupied: Vec<(f64, f64)> = graph
		.openings()
		.filter(|(_, opening)| opening.wall == proposal.wall)
		.map(|(_, opening)| {
			let half = f64::from(opening.spec.width) / 2.0;
			(
				opening.spec.center_distance - half,
				opening.spec.center_distance + half,
			)
		})
		.collect();
	occupied.sort_by(|a, b| a.0.total_cmp(&b.0));

	let mut spans = Vec::new();
	let mut free_from = *bounds.start();
	for (start, end) in occupied {
		spans.push((free_from, start));
		free_from = free_from.max(end);
	}
	spans.push((free_from, *bounds.end()));

	// The pointer is attracted by the wall end itself, but a junction keeps the opening off
	// the end, so the snap lands on the nearest position the junction allows.
	let edges = [
		(width / 2.0, *centers.start()),
		(length - width / 2.0, *centers.end()),
	]
	.into_iter()
	.filter(|&(flush, _)| in_range(flush))
	.map(|(_, nearest_allowed)| nearest_allowed);
	let span_centers = spans
		.into_iter()
		// A span narrower than the opening cannot hold it, so centring in it only
		// leads to a rejected proposal.
		.filter(|(start, end)| end - start >= width)
		.map(|(start, end)| (start + end) / 2.0)
		.filter(|&center_distance| in_range(center_distance));
	let structural = edges
		.chain(span_centers)
		.map(|center_distance| SnapCandidate {
			center_distance,
			priority: STRUCTURAL_PRIORITY,
		});
	// Distances from the wall's start line up with the grid when its ends are on the grid.
	let spacing = f64::from(grid_spacing);
	let grid_distance = (raw / spacing).round() * spacing;
	let grid =
		(in_range(grid_distance) && centers.contains(&grid_distance)).then_some(SnapCandidate {
			center_distance: grid_distance,
			priority: GRID_PRIORITY,
		});

	let snapped = structural.chain(grid).min_by(|a, b| {
		a.priority.cmp(&b.priority).then(
			(a.center_distance - raw)
				.abs()
				.total_cmp(&(b.center_distance - raw).abs()),
		)
	});

	match snapped {
		Some(candidate) => OpeningProposal {
			spec: OpeningSpec {
				center_distance: candidate.center_distance,
				..proposal.spec
			},
			..proposal
		},
		None => proposal,
	}
}

fn update_preview(proposal: Option<OpeningProposal>, house: &House, mut preview: Mut<EditPreview>) {
	let Some(proposal) = proposal else {
		if preview.holds_opening_proposal() {
			preview.state = None;
		}
		return;
	};
	let up_to_date = preview.state.as_ref().is_some_and(|state| {
		state.proposal == Proposal::Opening(proposal) && state.house_revision == house.revision()
	});
	if up_to_date {
		return;
	}
	preview.state = Some(PreviewState {
		house_revision: house.revision(),
		proposal: Proposal::Opening(proposal),
		outcome: propose_opening(house.graph(), &proposal),
	});
}

#[cfg(test)]
#[path = "tests/wall_tool_opening.rs"]
mod tests;
