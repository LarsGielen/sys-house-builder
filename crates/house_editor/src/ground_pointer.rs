use bevy::{
	prelude::*,
	transform::helper::TransformHelper,
	window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};

use crate::coordinates::{bevy_to_graph, graph_to_bevy};

const CURSOR_MARKER_RADIUS: f32 = 0.05;

/// Tracks where the pointer meets the ground and shows a marker there.
pub struct GroundPointerPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GroundPointerSystems {
	Pick,
	Presentation,
}

impl Plugin for GroundPointerPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<GroundPointer>()
			.add_systems(Startup, spawn_cursor_marker)
			.add_systems(
				PostUpdate,
				update_ground_pointer.in_set(GroundPointerSystems::Pick),
			)
			.add_systems(
				PostUpdate,
				move_cursor_marker.in_set(GroundPointerSystems::Presentation),
			);
	}
}

/// A pointer position and the floor-plan point beneath it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerOnGround {
	/// Logical pixels from the window's top-left corner.
	pub viewport_position: Vec2,
	/// Graph metres.
	pub graph_position: Vec2,
}

/// The pointer's ground hit.
///
/// `hit` is `None` when the pointer is not over the viewport, the window is unfocused, the
/// mouse is owned by a camera gesture, the pointer is over UI, or its ray misses the ground.
#[derive(Resource, Default)]
pub struct GroundPointer {
	pub hit: Option<PointerOnGround>,
	/// The pointer's ray in Bevy space. Set under the same ownership rules as `hit`, but
	/// also when the ray misses the ground, so tools can pick things above it.
	pub ray: Option<Ray3d>,
}

#[derive(Component)]
struct CursorMarker;

fn spawn_cursor_marker(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	commands.spawn((
		Name::new("cursor-marker"),
		CursorMarker,
		Mesh3d(meshes.add(Sphere::new(CURSOR_MARKER_RADIUS))),
		MeshMaterial3d(materials.add(StandardMaterial {
			base_color: Color::srgb(0.9, 0.2, 0.2),
			unlit: true,
			..default()
		})),
		Visibility::Hidden,
	));
}

fn update_ground_pointer(
	mut ground_pointer: ResMut<GroundPointer>,
	window_and_cursor: Single<(&Window, &CursorOptions), With<PrimaryWindow>>,
	camera: Single<(Entity, &Camera), With<Camera3d>>,
	transforms: TransformHelper,
	interactions: Query<&Interaction>,
) {
	let (window, cursor) = *window_and_cursor;
	let (camera_entity, camera) = *camera;
	let camera_transform = transforms
		.compute_global_transform(camera_entity)
		.expect("the editor camera and its ancestors must have transforms");

	// A locked cursor means a camera gesture owns the mouse, and its position is stale.
	let viewport_owns_pointer = window.focused
		&& cursor.grab_mode == CursorGrabMode::None
		&& !interactions
			.iter()
			.any(|interaction| *interaction != Interaction::None);

	let pointer = viewport_owns_pointer
		.then(|| {
			let viewport_position = window.cursor_position()?;
			let ray = camera
				.viewport_to_world(&camera_transform, viewport_position)
				.ok()?;
			Some((viewport_position, ray))
		})
		.flatten();

	ground_pointer.ray = pointer.map(|(_, ray)| ray);
	ground_pointer.hit = pointer.and_then(|(viewport_position, ray)| {
		Some(PointerOnGround {
			viewport_position,
			graph_position: ground_hit(ray)?,
		})
	});
}

/// Intersects a Bevy-space ray with the ground and returns the hit in graph metres.
///
/// A ray parallel to the ground or pointing away from it has no hit.
fn ground_hit(ray: Ray3d) -> Option<Vec2> {
	let hit = ray.plane_intersection_point(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))?;
	Some(bevy_to_graph(hit).truncate())
}

fn move_cursor_marker(
	ground_pointer: Res<GroundPointer>,
	marker: Single<(&mut Transform, &mut Visibility), With<CursorMarker>>,
) {
	let (mut transform, mut visibility) = marker.into_inner();
	match ground_pointer.hit {
		Some(hit) => {
			transform.translation = graph_to_bevy(hit.graph_position.extend(0.0));
			*visibility = Visibility::Inherited;
		}
		None => *visibility = Visibility::Hidden,
	}
}

#[cfg(test)]
#[path = "tests/ground_pointer.rs"]
mod tests;
