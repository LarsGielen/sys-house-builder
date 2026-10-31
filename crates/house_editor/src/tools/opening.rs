use bevy::{ecs::system::SystemParam, prelude::*};
use house_editor_core::{
	OpeningDefaults, OpeningSnapOptions, propose_opening_at_ray, snap_opening,
};
use wall_graph::WallGraph;
use wall_mesh::WallMesh;

use crate::{
	actions::EditorAction,
	actions::settings::{SettingsAction, SettingsActionPlugin, SettingsActionSystems},
	document::edit::{EditNotice, EditPreview, OpeningProposal, Proposal},
	document::house::House,
	input::{Action, ActionState, ActionsPlugin},
	tools::active::{Tool, tool_active},
	viewport::coordinates::bevy_to_graph,
	viewport::ground_pointer::GroundPointer,
	viewport::snapping::{SnapSettings, snapping_disabled},
};

/// Places an opening on the wall under the pointer with a click.
pub struct WallToolOpeningPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WallToolOpeningSystems;

impl Plugin for WallToolOpeningPlugin {
	fn build(&self, app: &mut App) {
		ActionsPlugin::ensure(app);
		SettingsActionPlugin::ensure(app);
		app.add_systems(
			PostUpdate,
			apply_opening_category.in_set(SettingsActionSystems),
		);
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

/// Size, elevation and category for new openings, as a Bevy resource.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct OpeningSettings(OpeningDefaults);

#[derive(SystemParam)]
struct PlacementInput<'w> {
	actions: Res<'w, ActionState>,
	ground_pointer: Res<'w, GroundPointer>,
}

fn apply_opening_category(
	mut actions: MessageReader<EditorAction>,
	mut settings: ResMut<OpeningSettings>,
) {
	for action in actions.read().filter_map(|action| action.settings()) {
		if let SettingsAction::SetOpeningCategory(category) = action {
			settings.set_category(category);
		}
	}
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
			if snapping_disabled(&input.actions) {
				return proposal;
			}
			let options = OpeningSnapOptions {
				grid_spacing: snap_settings.grid_spacing(),
				gap: snap_settings.opening_gap(),
				moving: None,
			};
			OpeningProposal {
				spec: snap_opening(
					house.graph(),
					proposal.wall,
					house.opening_bounds(proposal.wall),
					proposal.spec,
					options,
				),
				..proposal
			}
		});

	// Reborrowing retains Bevy's change tracking until the helper actually writes.
	update_preview(proposal, &house, preview.reborrow());

	if input.actions.just_pressed(Action::ToolPress)
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
	propose_opening_at_ray(
		graph,
		mesh,
		settings,
		bevy_to_graph(ray.origin),
		bevy_to_graph(*ray.direction),
	)
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
#[path = "../tests/tools/opening.rs"]
mod tests;
