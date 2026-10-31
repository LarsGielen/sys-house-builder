use std::{f32::consts::FRAC_PI_2, fmt};

use bevy::prelude::Resource;

const DEFAULT_DISTANCE_MIN: f32 = 0.5;
const DEFAULT_DISTANCE_MAX: f32 = 100.0;
const DEFAULT_RESET_VIEW_RADIUS: f32 = 5.0;
const DEFAULT_ORBIT_RADIANS_PER_PIXEL: f32 = 0.005;
const DEFAULT_ELEVATION_MIN: f32 = 0.3;
const DEFAULT_ELEVATION_MAX: f32 = FRAC_PI_2 - 0.01;
const DEFAULT_PAN_FOCUS_DISTANCES_PER_PIXEL: f32 = 0.0015;
const DEFAULT_KEYBOARD_PAN_FOCUS_DISTANCES_PER_SECOND: f32 = 1.0;
const DEFAULT_ZOOM_FACTOR_PER_LINE: f32 = 1.1;

/// Why a camera setting was refused. The setting keeps its previous value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidCameraSetting {
	/// A speed, sensitivity or distance was not a positive, finite number.
	NotPositive,
	/// The minimum of a range was not below its maximum.
	EmptyRange,
	/// The elevation range must stay between the horizon and straight down.
	ElevationOutOfRange,
	/// A zoom factor of 1 or less would not zoom.
	ZoomFactorTooSmall,
}

impl fmt::Display for InvalidCameraSetting {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.write_str(match self {
			Self::NotPositive => "Camera values must be positive, finite numbers",
			Self::EmptyRange => "A camera range needs its minimum below its maximum",
			Self::ElevationOutOfRange => "Camera elevation must lie between 0 and a right angle",
			Self::ZoomFactorTooSmall => "The zoom factor per scroll line must exceed 1",
		})
	}
}

impl std::error::Error for InvalidCameraSetting {}

fn positive(value: f32) -> Result<f32, InvalidCameraSetting> {
	if value.is_finite() && value > 0.0 {
		Ok(value)
	} else {
		Err(InvalidCameraSetting::NotPositive)
	}
}

/// How the orbit camera responds to input and how far it may go. Each setter validates its
/// value and leaves the setting unchanged when refused.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct CameraSettings {
	distance_min: f32,
	distance_max: f32,
	reset_view_radius: f32,
	orbit_radians_per_pixel: f32,
	elevation_min: f32,
	elevation_max: f32,
	pan_focus_distances_per_pixel: f32,
	keyboard_pan_focus_distances_per_second: f32,
	zoom_factor_per_line: f32,
}

impl Default for CameraSettings {
	fn default() -> Self {
		Self {
			distance_min: DEFAULT_DISTANCE_MIN,
			distance_max: DEFAULT_DISTANCE_MAX,
			reset_view_radius: DEFAULT_RESET_VIEW_RADIUS,
			orbit_radians_per_pixel: DEFAULT_ORBIT_RADIANS_PER_PIXEL,
			elevation_min: DEFAULT_ELEVATION_MIN,
			elevation_max: DEFAULT_ELEVATION_MAX,
			pan_focus_distances_per_pixel: DEFAULT_PAN_FOCUS_DISTANCES_PER_PIXEL,
			keyboard_pan_focus_distances_per_second:
				DEFAULT_KEYBOARD_PAN_FOCUS_DISTANCES_PER_SECOND,
			zoom_factor_per_line: DEFAULT_ZOOM_FACTOR_PER_LINE,
		}
	}
}

impl CameraSettings {
	pub fn clamp_distance(&self, distance: f32) -> f32 {
		distance.clamp(self.distance_min, self.distance_max)
	}

	pub fn clamp_elevation(&self, elevation: f32) -> f32 {
		elevation.clamp(self.elevation_min, self.elevation_max)
	}

	pub fn reset_view_radius(&self) -> f32 {
		self.reset_view_radius
	}

	pub fn orbit_radians_per_pixel(&self) -> f32 {
		self.orbit_radians_per_pixel
	}

	pub fn pan_focus_distances_per_pixel(&self) -> f32 {
		self.pan_focus_distances_per_pixel
	}

	pub fn keyboard_pan_focus_distances_per_second(&self) -> f32 {
		self.keyboard_pan_focus_distances_per_second
	}

	pub fn zoom_factor_per_line(&self) -> f32 {
		self.zoom_factor_per_line
	}

	pub fn set_distance_limits(&mut self, min: f32, max: f32) -> Result<(), InvalidCameraSetting> {
		let (min, max) = (positive(min)?, positive(max)?);
		if min >= max {
			return Err(InvalidCameraSetting::EmptyRange);
		}
		(self.distance_min, self.distance_max) = (min, max);
		Ok(())
	}

	pub fn set_elevation_limits(&mut self, min: f32, max: f32) -> Result<(), InvalidCameraSetting> {
		let (min, max) = (positive(min)?, positive(max)?);
		if min >= max {
			return Err(InvalidCameraSetting::EmptyRange);
		}
		if max > FRAC_PI_2 {
			return Err(InvalidCameraSetting::ElevationOutOfRange);
		}
		(self.elevation_min, self.elevation_max) = (min, max);
		Ok(())
	}

	pub fn set_reset_view_radius(&mut self, radius: f32) -> Result<(), InvalidCameraSetting> {
		self.reset_view_radius = positive(radius)?;
		Ok(())
	}

	pub fn set_orbit_radians_per_pixel(&mut self, value: f32) -> Result<(), InvalidCameraSetting> {
		self.orbit_radians_per_pixel = positive(value)?;
		Ok(())
	}

	pub fn set_pan_focus_distances_per_pixel(
		&mut self,
		value: f32,
	) -> Result<(), InvalidCameraSetting> {
		self.pan_focus_distances_per_pixel = positive(value)?;
		Ok(())
	}

	pub fn set_keyboard_pan_focus_distances_per_second(
		&mut self,
		value: f32,
	) -> Result<(), InvalidCameraSetting> {
		self.keyboard_pan_focus_distances_per_second = positive(value)?;
		Ok(())
	}

	pub fn set_zoom_factor_per_line(&mut self, value: f32) -> Result<(), InvalidCameraSetting> {
		let value = positive(value)?;
		if value <= 1.0 {
			return Err(InvalidCameraSetting::ZoomFactorTooSmall);
		}
		self.zoom_factor_per_line = value;
		Ok(())
	}
}

#[cfg(test)]
#[path = "../tests/config/camera.rs"]
mod tests;
