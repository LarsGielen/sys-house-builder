//! Double-precision wall centerlines and their parallel offsets.
//!
//! Distances are measured along the centerline from the path start. Offsets are measured to the
//! left of the direction of travel, so a wall's faces lie at `+half_thickness` and
//! `-half_thickness`.
use std::f64::consts::{PI, TAU};

use glam::DVec2;
use wall_graph::WallCurve;

use crate::MeshError;

/// Most chords one curved run may use, matching the graph's own sampling limit.
const MAX_ARC_SEGMENTS: f64 = 65_536.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Path {
	Line {
		start: DVec2,
		end: DVec2,
		direction: DVec2,
		length: f64,
	},
	Arc {
		center: DVec2,
		radius: f64,
		start_angle: f64,
		sweep: f64,
	},
}

/// The complete line or circle that contains one offset of a path.
#[derive(Debug, Clone, Copy)]
pub(crate) enum OffsetCurve {
	Line { point: DVec2, direction: DVec2 },
	Circle { center: DVec2, radius: f64 },
}

impl Path {
	pub(crate) fn new(curve: WallCurve) -> Self {
		match curve {
			WallCurve::Straight { start, end } => Self::Line {
				start,
				end,
				direction: (end - start).normalize(),
				length: start.distance(end),
			},
			WallCurve::CircularArc {
				center,
				radius,
				start_angle,
				sweep,
			} => Self::Arc {
				center,
				radius,
				start_angle,
				sweep,
			},
		}
	}

	pub(crate) fn length(self) -> f64 {
		match self {
			Self::Line { length, .. } => length,
			Self::Arc { radius, sweep, .. } => radius * sweep.abs(),
		}
	}

	pub(crate) fn reversed(self) -> Self {
		match self {
			Self::Line {
				start,
				end,
				direction,
				length,
			} => Self::Line {
				start: end,
				end: start,
				direction: -direction,
				length,
			},
			Self::Arc {
				center,
				radius,
				start_angle,
				sweep,
			} => Self::Arc {
				center,
				radius,
				start_angle: start_angle + sweep,
				sweep: -sweep,
			},
		}
	}

	pub(crate) fn center(self) -> Option<DVec2> {
		match self {
			Self::Line { .. } => None,
			Self::Arc { center, .. } => Some(center),
		}
	}

	pub(crate) fn point(self, distance: f64, offset: f64) -> DVec2 {
		match self {
			Self::Line {
				start, direction, ..
			} => start + direction * distance + direction.perp() * offset,
			Self::Arc { center, .. } => {
				center
					+ DVec2::from_angle(self.angle_at(distance))
						* self.offset_radius(offset).expect("arc has a radius")
			}
		}
	}

	pub(crate) fn tangent(self, distance: f64) -> DVec2 {
		match self {
			Self::Line { direction, .. } => direction,
			Self::Arc { sweep, .. } => {
				DVec2::from_angle(self.angle_at(distance)).perp() * sweep.signum()
			}
		}
	}

	/// The radius of an arc's offset. The left side of a counterclockwise arc faces its center.
	pub(crate) fn offset_radius(self, offset: f64) -> Option<f64> {
		match self {
			Self::Line { .. } => None,
			Self::Arc { radius, sweep, .. } => Some(radius - sweep.signum() * offset),
		}
	}

	/// The offset of whichever face is farther from an arc's center; chords deviate most there.
	pub(crate) fn outer_offset(self, half_thickness: f64) -> f64 {
		match self {
			Self::Line { .. } => 0.0,
			Self::Arc { sweep, .. } => -sweep.signum() * half_thickness,
		}
	}

	pub(crate) fn offset_curve(self, offset: f64) -> OffsetCurve {
		match self {
			Self::Line { direction, .. } => OffsetCurve::Line {
				point: self.point(0.0, offset),
				direction,
			},
			Self::Arc { center, .. } => OffsetCurve::Circle {
				center,
				radius: self.offset_radius(offset).expect("arc has a radius"),
			},
		}
	}

	/// The path distance of a point on one of the offsets. For an arc, the result is the
	/// revolution nearest `reference`, so points just behind the start get negative distances.
	pub(crate) fn distance_near(self, point: DVec2, reference: f64) -> f64 {
		match self {
			Self::Line {
				start, direction, ..
			} => (point - start).dot(direction),
			Self::Arc {
				center,
				radius,
				sweep,
				..
			} => {
				let turn = (point - center).to_angle() - self.angle_at(reference);
				let wrapped = (turn + PI).rem_euclid(TAU) - PI;
				reference + wrapped * sweep.signum() * radius
			}
		}
	}

	/// Distances from `from` to `to`, inclusive, whose chords along `offset` stay within
	/// `deviation` of the true curve. Straight paths need only their ends.
	pub(crate) fn samples(
		self,
		from: f64,
		to: f64,
		offset: f64,
		deviation: f64,
	) -> Result<Vec<f64>, MeshError> {
		let segments = match self {
			Self::Line { .. } => 1,
			Self::Arc { radius, .. } => {
				let chord_radius = self.offset_radius(offset).expect("arc has a radius").abs();
				// asin is stable for small error/radius ratios, unlike acos(1 - error/radius).
				let step = 4.0 * (deviation / (2.0 * chord_radius)).min(0.5).sqrt().asin();
				let count = ((to - from).abs() / radius / step).ceil();
				if !count.is_finite() || count > MAX_ARC_SEGMENTS {
					return Err(MeshError::OutputLimitExceeded);
				}
				(count as usize).max(1)
			}
		};
		Ok((0..=segments)
			.map(|index| {
				if index == segments {
					to
				} else {
					from + (to - from) * index as f64 / segments as f64
				}
			})
			.collect())
	}

	fn angle_at(self, distance: f64) -> f64 {
		match self {
			Self::Line { .. } => unreachable!("lines have no angle"),
			Self::Arc {
				radius,
				start_angle,
				sweep,
				..
			} => start_angle + sweep.signum() * distance / radius,
		}
	}
}

/// Every point where two complete offset curves meet. Tangent contacts yield one point.
pub(crate) fn intersections(first: OffsetCurve, second: OffsetCurve) -> Vec<DVec2> {
	match (first, second) {
		(
			OffsetCurve::Line {
				point: first_point,
				direction: first_direction,
			},
			OffsetCurve::Line {
				point: second_point,
				direction: second_direction,
			},
		) => {
			let denominator = first_direction.perp_dot(second_direction);
			if denominator.abs() < 1e-12 {
				return Vec::new();
			}
			let along = (second_point - first_point).perp_dot(second_direction) / denominator;
			vec![first_point + first_direction * along]
		}
		(OffsetCurve::Line { point, direction }, OffsetCurve::Circle { center, radius })
		| (OffsetCurve::Circle { center, radius }, OffsetCurve::Line { point, direction }) => {
			let closest = point + direction * (center - point).dot(direction);
			let half_chord_squared = radius * radius - closest.distance_squared(center);
			if half_chord_squared < -1e-12 * radius * radius {
				return Vec::new();
			}
			let half_chord = direction * half_chord_squared.max(0.0).sqrt();
			vec![closest + half_chord, closest - half_chord]
		}
		(
			OffsetCurve::Circle {
				center: first_center,
				radius: first_radius,
			},
			OffsetCurve::Circle {
				center: second_center,
				radius: second_radius,
			},
		) => {
			let delta = second_center - first_center;
			let distance = delta.length();
			if distance < 1e-12 {
				return Vec::new();
			}
			let along = (first_radius * first_radius - second_radius * second_radius
				+ distance * distance)
				/ (2.0 * distance);
			let height_squared = first_radius * first_radius - along * along;
			if height_squared < -1e-12 * first_radius * first_radius {
				return Vec::new();
			}
			let axis = delta / distance;
			let base = first_center + axis * along;
			let height = axis.perp() * height_squared.max(0.0).sqrt();
			vec![base + height, base - height]
		}
	}
}

#[cfg(test)]
#[path = "tests/path.rs"]
mod tests;
