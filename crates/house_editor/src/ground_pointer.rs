use bevy::{
	prelude::*,
	window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};

use crate::{
	coordinates::{bevy_to_graph, graph_to_bevy},
	orbit_camera::is_ui_interacted,
};

const CURSOR_MARKER_RADIUS: f32 = 0.05;

/// Tracks where the pointer meets the ground and shows a marker there.
pub struct GroundPointerPlugin;

impl Plugin for GroundPointerPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<GroundPointer>()
			.add_systems(Startup, spawn_cursor_marker)
			.add_systems(Update, move_cursor_marker)
			// The ray must use this frame's camera pose, which only reaches `GlobalTransform`
			// once transforms are propagated.
			.add_systems(
				PostUpdate,
				update_ground_pointer.after(TransformSystems::Propagate),
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
/// `None` when the pointer is not over the viewport, the window is unfocused, the
/// mouse is owned by a camera gesture, or the pointer ray never reaches the ground.
#[derive(Resource, Default)]
pub struct GroundPointer {
	pub hit: Option<PointerOnGround>,
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

pub fn update_ground_pointer(
	mut ground_pointer: ResMut<GroundPointer>,
	window_and_cursor: Single<(&Window, &CursorOptions), With<PrimaryWindow>>,
	camera: Single<(&Camera, &GlobalTransform), With<Camera3d>>,
	interactions: Query<&Interaction>,
) {
	let (window, cursor) = *window_and_cursor;
	let (camera, camera_transform) = *camera;

	// A locked cursor means a camera gesture owns the mouse, and its position is stale.
	let viewport_owns_pointer = window.focused
		&& cursor.grab_mode == CursorGrabMode::None
		&& !is_ui_interacted(&interactions);

	ground_pointer.hit = viewport_owns_pointer
		.then(|| {
			let viewport_position = window.cursor_position()?;
			let ray = camera
				.viewport_to_world(camera_transform, viewport_position)
				.ok()?;
			Some(PointerOnGround {
				viewport_position,
				graph_position: ground_hit(ray)?,
			})
		})
		.flatten();
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
mod tests {
	use super::*;

	#[test]
	fn downward_ray_hits_the_ground_in_graph_coordinates() {
		let ray = Ray3d::new(Vec3::new(1.0, 5.0, 2.0), Dir3::NEG_Y);
		// Bevy z = 2 is graph y = -2.
		assert_eq!(ground_hit(ray), Some(Vec2::new(1.0, -2.0)));
	}

	#[test]
	fn ray_pointing_away_from_the_ground_has_no_hit() {
		let ray = Ray3d::new(Vec3::new(0.0, 5.0, 0.0), Dir3::Y);
		assert_eq!(ground_hit(ray), None);
	}

	#[test]
	fn ray_parallel_to_the_ground_has_no_hit() {
		let ray = Ray3d::new(Vec3::new(0.0, 5.0, 0.0), Dir3::X);
		assert_eq!(ground_hit(ray), None);
	}
}
