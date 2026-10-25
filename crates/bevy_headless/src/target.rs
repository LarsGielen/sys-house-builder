use bevy::{
	camera::{CameraUpdateSystems, ImageRenderTarget, RenderTarget},
	prelude::*,
	window::WindowRef,
};

/// The image every primary-window camera is redirected to, and the one captures read.
#[derive(Resource, Clone)]
pub(crate) struct HeadlessTarget {
	pub image: Handle<Image>,
	pub scale_factor: f32,
}

/// Redirects cameras that would render to the primary window onto [`HeadlessTarget`], so an app
/// written for a window renders headless without changes.
pub(crate) struct HeadlessTargetPlugin;

impl Plugin for HeadlessTargetPlugin {
	fn build(&self, app: &mut App) {
		// Before camera sizing, so a camera spawned this frame never computes window-based info.
		app.add_systems(
			PostUpdate,
			retarget_window_cameras.before(CameraUpdateSystems),
		);
	}
}

fn targets_primary_window(target: &RenderTarget) -> bool {
	matches!(target, RenderTarget::Window(WindowRef::Primary))
}

fn retarget_window_cameras(
	mut commands: Commands,
	headless_target: Res<HeadlessTarget>,
	mut cameras: Query<(Entity, &Camera, &mut RenderTarget)>,
	default_ui_cameras: Query<(), With<IsDefaultUiCamera>>,
) {
	let mut retargeted = Vec::new();
	for (entity, camera, mut target) in &mut cameras {
		if !targets_primary_window(&target) {
			continue;
		}
		*target = RenderTarget::Image(ImageRenderTarget {
			handle: headless_target.image.clone(),
			scale_factor: headless_target.scale_factor,
		});
		retargeted.push((camera.order, entity));
	}

	// Bevy UI picks its camera among those rendering to the primary window, which none do now.
	// Choose the one it would have chosen, unless the app already named one.
	if default_ui_cameras.is_empty()
		&& let Some((_, entity)) = retargeted.into_iter().max()
	{
		commands.entity(entity).insert(IsDefaultUiCamera);
	}
}

#[cfg(test)]
#[path = "tests/target.rs"]
mod tests;
