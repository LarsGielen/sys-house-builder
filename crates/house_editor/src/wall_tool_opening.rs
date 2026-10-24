use bevy::{ecs::system::SystemParam, prelude::*};
use wall_graph::{OpeningSpec, WallGraph};
use wall_mesh::WallMesh;

use crate::{
	coordinates::bevy_to_graph,
	edit::{EditNotice, EditPreview, OpeningProposal, Proposal},
	ground_pointer::GroundPointer,
	house::House,
	opening_area::closest_distance_along_wall,
	opening_category::OpeningCategory,
	opening_snap::{OpeningSnapOptions, snap_opening},
	snapping::{SnapSettings, snapping_disabled},
	wall_tool_active::{Tool, tool_active},
	wall_tool_selection::pick_wall_hit,
};

const DEFAULT_WIDTH_METRES: f32 = 0.9;
const DEFAULT_HEIGHT_METRES: f32 = 2.1;
const DEFAULT_BOTTOM_METRES: f32 = 0.0;

/// Places an opening on the wall under the pointer with a click.
pub struct WallToolOpeningPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WallToolOpeningSystems;

impl Plugin for WallToolOpeningPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<OpeningSettings>()
			.init_resource::<EditPreview>()
			.init_resource::<EditNotice>()
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
	category: OpeningCategory,
	width: f32,
	height: f32,
	bottom: f32,
}

impl Default for OpeningSettings {
	fn default() -> Self {
		Self {
			category: OpeningCategory::default(),
			width: DEFAULT_WIDTH_METRES,
			height: DEFAULT_HEIGHT_METRES,
			bottom: DEFAULT_BOTTOM_METRES,
		}
	}
}

impl OpeningSettings {
	pub fn category(&self) -> OpeningCategory {
		self.category
	}

	pub fn set_category(&mut self, category: OpeningCategory) {
		self.category = category;
	}

	fn spec_at(&self, center_distance: f64) -> OpeningSpec {
		OpeningSpec {
			center_distance,
			width: self.width,
			bottom: self.bottom,
			height: self.height,
		}
	}
}

#[derive(SystemParam)]
struct PlacementInput<'w> {
	mouse: Res<'w, ButtonInput<MouseButton>>,
	keyboard: Res<'w, ButtonInput<KeyCode>>,
	ground_pointer: Res<'w, GroundPointer>,
}

fn update_opening_placement(
	input: PlacementInput,
	settings: Res<OpeningSettings>,
	snap_settings: Res<SnapSettings>,
	mut house: ResMut<House>,
	mut notice: ResMut<EditNotice>,
	mut preview: ResMut<EditPreview>,
) {
	let proposal = input
		.ground_pointer
		.ray
		.and_then(|ray| propose_at_ray(house.graph(), house.mesh(), &settings, ray))
		.map(|proposal| {
			if snapping_disabled(&input.keyboard) {
				return proposal;
			}
			let options = OpeningSnapOptions {
				grid_spacing: snap_settings.grid_spacing(),
				gap: snap_settings.opening_gap(),
				moving: None,
			};
			OpeningProposal {
				spec: snap_opening(house.graph(), proposal.wall, proposal.spec, options),
				..proposal
			}
		});

	// Reborrowing retains Bevy's change tracking until the helper actually writes.
	update_preview(proposal, &house, preview.reborrow());

	if input.mouse.just_pressed(MouseButton::Left)
		&& let Some(Proposal::Opening(proposal)) =
			preview.state.as_ref().map(|state| state.proposal)
		&& let Some(candidate) = preview.take_valid(Proposal::Opening(proposal), house.revision())
	{
		house.commit(candidate);
		notice.message = None;
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
	let center_distance = closest_distance_along_wall(graph, hit.wall, hit.position.truncate())?;
	Some(OpeningProposal {
		wall: hit.wall,
		spec: settings.spec_at(center_distance),
		category: settings.category(),
	})
}

fn update_preview(proposal: Option<OpeningProposal>, house: &House, mut preview: Mut<EditPreview>) {
	let Some(proposal) = proposal else {
		if preview.holds_opening_proposal() {
			preview.state = None;
		}
		return;
	};
	EditPreview::update(
		preview,
		Proposal::Opening(proposal),
		house.revision(),
		house.committed(),
	);
}

#[cfg(test)]
#[path = "tests/wall_tool_opening.rs"]
mod tests;
