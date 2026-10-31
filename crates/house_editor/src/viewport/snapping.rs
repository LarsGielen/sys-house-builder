use bevy::{ecs::system::SystemParam, prelude::*, transform::helper::TransformHelper};
use house_editor_core::{SnapSettings as CoreSnapSettings, choose_snap_target};
pub use house_editor_core::{SnapSource, SnapTarget};

use crate::{
	actions::EditorAction,
	actions::settings::{SettingsAction, SettingsActionPlugin, SettingsActionSystems},
	config::interaction::InteractionConfig,
	document::edit::EditNotice,
	document::house::House,
	input::{Action, ActionState, ActionsPlugin},
	viewport::coordinates::graph_to_bevy,
	viewport::ground_pointer::GroundPointer,
};

/// Grid, radii, and opening-gap policy for snapping, as a Bevy resource.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct SnapSettings(CoreSnapSettings);

const SNAP_MARKER_RADIUS: f32 = 0.08;

/// Chooses a snap target under the pointer, shows it, makes it available to editor tools.
pub struct SnappingPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SnappingSystems {
	Pick,
	Presentation,
}
impl Plugin for SnappingPlugin {
	fn build(&self, app: &mut App) {
		ActionsPlugin::ensure(app);
		SettingsActionPlugin::ensure(app);
		app.init_resource::<InteractionConfig>();
		app.add_systems(
			PostUpdate,
			apply_snap_settings.in_set(SettingsActionSystems),
		);
		app.init_resource::<SnapSettings>()
			.init_resource::<EditNotice>()
			.init_resource::<SnappedPointer>()
			.add_systems(Startup, spawn_snap_marker)
			.add_systems(
				PostUpdate,
				update_snapped_pointer.in_set(SnappingSystems::Pick),
			)
			.add_systems(
				PostUpdate,
				update_snap_marker.in_set(SnappingSystems::Presentation),
			);
	}
}

/// The snap target under the pointer; `None` when the pointer has no ground hit.
#[derive(Resource, Default)]
pub struct SnappedPointer {
	pub target: Option<SnapTarget>,
}

pub fn snapping_disabled(actions: &ActionState) -> bool {
	actions.pressed(Action::SuppressSnapping)
}

pub(crate) fn apply_snap_settings(
	mut actions: MessageReader<EditorAction>,
	mut settings: ResMut<SnapSettings>,
	mut notice: ResMut<EditNotice>,
) {
	for action in actions.read().filter_map(|action| action.settings()) {
		match action {
			SettingsAction::StepGridSpacing(step) => settings.step_grid_spacing(step),
			SettingsAction::SetOpeningGap(gap) => {
				notice.message = settings
					.set_opening_gap(gap)
					.err()
					.map(|error| error.to_string());
			}
			_ => {}
		}
	}
}

#[derive(SystemParam)]
struct SnapPolicy<'w> {
	settings: Res<'w, SnapSettings>,
	config: Res<'w, InteractionConfig>,
	actions: Res<'w, ActionState>,
}

fn update_snapped_pointer(
	mut snapped_pointer: ResMut<SnappedPointer>,
	ground_pointer: Res<GroundPointer>,
	house: Res<House>,
	policy: SnapPolicy,
	camera: Single<(Entity, &Camera), With<Camera3d>>,
	transforms: TransformHelper,
) {
	let (camera_entity, camera) = *camera;
	let camera_transform = transforms
		.compute_global_transform(camera_entity)
		.expect("the editor camera and its ancestors must have transforms");
	let snapping_disabled = snapping_disabled(&policy.actions);
	let previous_source = snapped_pointer.target.map(|target| target.source);

	snapped_pointer.target = ground_pointer.hit.map(|pointer| {
		if snapping_disabled {
			return SnapTarget::free(pointer.graph_position);
		}
		choose_snap_target(
			house.graph(),
			pointer,
			&policy.settings,
			&policy.config,
			previous_source,
			|graph_position| {
				camera
					.world_to_viewport(&camera_transform, graph_to_bevy(graph_position.extend(0.0)))
					.ok()
			},
		)
	});
}

#[derive(Component)]
struct SnapMarker;

#[derive(Resource)]
struct SnapMarkerMaterials {
	junction: Handle<StandardMaterial>,
	wall: Handle<StandardMaterial>,
	grid: Handle<StandardMaterial>,
}

fn spawn_snap_marker(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let mut unlit = |color: Color| {
		materials.add(StandardMaterial {
			base_color: color,
			unlit: true,
			..default()
		})
	};
	let marker_materials = SnapMarkerMaterials {
		junction: unlit(Color::srgb(0.2, 0.85, 0.3)),
		wall: unlit(Color::srgb(0.95, 0.8, 0.2)),
		grid: unlit(Color::srgb(0.25, 0.5, 0.95)),
	};

	commands.spawn((
		Name::new("snap-marker"),
		SnapMarker,
		Mesh3d(meshes.add(Sphere::new(SNAP_MARKER_RADIUS))),
		MeshMaterial3d(marker_materials.grid.clone()),
		Visibility::Hidden,
	));
	commands.insert_resource(marker_materials);
}

fn update_snap_marker(
	snapped_pointer: Res<SnappedPointer>,
	marker_materials: Res<SnapMarkerMaterials>,
	marker: Single<
		(
			&mut Transform,
			&mut Visibility,
			&mut MeshMaterial3d<StandardMaterial>,
		),
		With<SnapMarker>,
	>,
) {
	let (mut transform, mut visibility, mut material) = marker.into_inner();

	let snapped = snapped_pointer.target.and_then(|target| {
		let material = match target.source {
			SnapSource::Junction(_) => &marker_materials.junction,
			SnapSource::Wall(_) => &marker_materials.wall,
			SnapSource::Grid => &marker_materials.grid,
			SnapSource::Free => return None,
		};
		Some((target.position, material))
	});

	let Some((position, wanted_material)) = snapped else {
		*visibility = Visibility::Hidden;
		return;
	};
	transform.translation = graph_to_bevy(position.extend(0.0));
	*visibility = Visibility::Inherited;
	if material.0 != *wanted_material {
		material.0 = wanted_material.clone();
	}
}
