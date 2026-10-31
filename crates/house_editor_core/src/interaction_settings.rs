use std::fmt;

const DEFAULT_DRAG_THRESHOLD_PIXELS: f32 = 6.0;
const DEFAULT_SNAP_ACQUIRE_RADIUS_PIXELS: f32 = 12.0;
const DEFAULT_SNAP_RELEASE_RADIUS_PIXELS: f32 = 20.0;

/// Pointer tuning that changes how editing feels but never what is valid: how far a press must
/// travel to count as a drag, and how near the pointer must come to a snap target to take it and
/// how far it may drift before letting go. These are screen distances, unrelated to the
/// document's geometric tolerances.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InteractionSettings {
	drag_threshold_pixels: f32,
	snap_acquire_radius_pixels: f32,
	snap_release_radius_pixels: f32,
}

/// Why a setting was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidInteractionSetting {
	/// A distance was not a positive, finite number of pixels.
	NotPositive,
	/// Letting go of a snap target must take at least as far as taking it, or a target would
	/// flicker on and off at the boundary.
	ReleaseBelowAcquire,
}

impl fmt::Display for InvalidInteractionSetting {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.write_str(match self {
			Self::NotPositive => "Distances must be positive, finite pixel counts",
			Self::ReleaseBelowAcquire => {
				"The snap release radius must be at least the snap acquire radius"
			}
		})
	}
}

impl std::error::Error for InvalidInteractionSetting {}

impl Default for InteractionSettings {
	fn default() -> Self {
		Self {
			drag_threshold_pixels: DEFAULT_DRAG_THRESHOLD_PIXELS,
			snap_acquire_radius_pixels: DEFAULT_SNAP_ACQUIRE_RADIUS_PIXELS,
			snap_release_radius_pixels: DEFAULT_SNAP_RELEASE_RADIUS_PIXELS,
		}
	}
}

impl InteractionSettings {
	pub fn new(
		drag_threshold_pixels: f32,
		snap_acquire_radius_pixels: f32,
		snap_release_radius_pixels: f32,
	) -> Result<Self, InvalidInteractionSetting> {
		let positive = |value: f32| value.is_finite() && value > 0.0;
		if ![
			drag_threshold_pixels,
			snap_acquire_radius_pixels,
			snap_release_radius_pixels,
		]
		.into_iter()
		.all(positive)
		{
			return Err(InvalidInteractionSetting::NotPositive);
		}
		if snap_release_radius_pixels < snap_acquire_radius_pixels {
			return Err(InvalidInteractionSetting::ReleaseBelowAcquire);
		}
		Ok(Self {
			drag_threshold_pixels,
			snap_acquire_radius_pixels,
			snap_release_radius_pixels,
		})
	}

	pub fn drag_threshold_pixels(&self) -> f32 {
		self.drag_threshold_pixels
	}

	pub fn snap_acquire_radius_pixels(&self) -> f32 {
		self.snap_acquire_radius_pixels
	}

	pub fn snap_release_radius_pixels(&self) -> f32 {
		self.snap_release_radius_pixels
	}
}

#[cfg(test)]
#[path = "tests/interaction_settings.rs"]
mod tests;
