//! Curve calculations are independent of graph identifiers and rendering.
use std::f64::consts::TAU;

use glam::{DVec2, Vec2};

pub(crate) const DISTANCE_TOLERANCE: f64 = 1e-4;
pub(crate) const ANGLE_TOLERANCE: f64 = 1e-6;

/// Endpoints belong to the graph. Shape data is stored once for each wall.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum CurveShape {
	Straight,
	CircularArc { sweep: f64 },
}

impl CurveShape {
	pub(crate) fn reversed(self) -> Self {
		match self {
			Self::Straight => self,
			Self::CircularArc { sweep } => Self::CircularArc { sweep: -sweep },
		}
	}
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Curve {
	pub(crate) start: DVec2,
	pub(crate) end: DVec2,
	pub(crate) shape: CurveShape,
}

/// Isolated contacts retain parameters on both curves; coincident stretches are rejected by the graph.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CurveContact {
	pub(crate) first_parameter: f64,
	pub(crate) second_parameter: f64,
	pub(crate) tangential: bool,
}

pub(crate) struct Intersections {
	pub(crate) contacts: Vec<CurveContact>,
	pub(crate) overlapping: bool,
}

impl Curve {
	pub(crate) fn new(start: Vec2, end: Vec2, shape: CurveShape) -> Self {
		Self {
			start: start.as_dvec2(),
			end: end.as_dvec2(),
			shape,
		}
	}

	pub(crate) fn same_path(self, other: Self) -> bool {
		(self.start == other.start && self.end == other.end && self.shape == other.shape)
			|| (self.start == other.end
				&& self.end == other.start
				&& self.shape.reversed() == other.shape)
	}

	/// Extracting geometry here keeps the original endpoints available when future variants
	/// need them to subdivide their control points.
	pub(crate) fn subcurve(self, start: f64, end: f64) -> Self {
		let shape = match self.shape {
			CurveShape::Straight => CurveShape::Straight,
			CurveShape::CircularArc { sweep } => CurveShape::CircularArc {
				sweep: sweep * (end - start),
			},
		};
		Self {
			start: self.position(start),
			end: self.position(end),
			shape,
		}
	}

	/// Returns one shape only when replacing both paths stays within the graph's tolerance.
	pub(crate) fn merged_shape(self, next: Self) -> Option<CurveShape> {
		if self.end != next.start
			|| self.tangent(1.0).dot(next.tangent(0.0)) <= 0.0
			|| self.tangent(1.0).perp_dot(next.tangent(0.0)).abs() >= ANGLE_TOLERANCE
		{
			return None;
		}
		let shape = match (self.shape, next.shape) {
			(CurveShape::Straight, CurveShape::Straight) => CurveShape::Straight,
			(
				CurveShape::CircularArc { sweep: first },
				CurveShape::CircularArc { sweep: second },
			) if first.signum() == second.signum() && (first + second).abs() < TAU => {
				CurveShape::CircularArc {
					sweep: first + second,
				}
			}
			_ => return None,
		};
		let joined = Self {
			start: self.start,
			end: next.end,
			shape,
		};
		if !joined.valid() {
			return None;
		}
		let split = match shape {
			CurveShape::Straight => self.length() / (self.length() + next.length()),
			CurveShape::CircularArc { sweep } => {
				let CurveShape::CircularArc { sweep: first } = self.shape else {
					unreachable!()
				};
				first / sweep
			}
		};
		(joined.subcurve(0.0, split).deviation_from(self, 0.0, 1.0) <= DISTANCE_TOLERANCE
			&& joined.subcurve(split, 1.0).deviation_from(next, 0.0, 1.0) <= DISTANCE_TOLERANCE)
			.then_some(shape)
	}

	pub(crate) fn valid(self) -> bool {
		if !self.start.is_finite()
			|| !self.end.is_finite()
			|| self.start.distance(self.end) <= DISTANCE_TOLERANCE
		{
			return false;
		}
		match self.shape {
			CurveShape::Straight => true,
			CurveShape::CircularArc { sweep } => {
				if !sweep.is_finite()
					|| sweep == 0.0 || sweep.abs() >= TAU
					|| !self
						.circle()
						.is_some_and(|(center, radius)| center.is_finite() && radius.is_finite())
				{
					return false;
				}
				// Check the coordinate extrema on the actual arc, since a major arc may extend
				// far beyond its finite endpoints. The public coordinate type remains Vec2.
				let initial_angle = self.tangent(0.0).to_angle();
				for axis in 0..4 {
					let angle = axis as f64 * std::f64::consts::FRAC_PI_2;
					let parameter =
						((angle - initial_angle) * sweep.signum()).rem_euclid(TAU) / sweep.abs();
					if parameter <= 1.0 && !self.position(parameter).as_vec2().is_finite() {
						return false;
					}
				}
				true
			}
		}
	}

	pub(crate) fn circle(self) -> Option<(DVec2, f64)> {
		let CurveShape::CircularArc { sweep } = self.shape else {
			return None;
		};
		let chord = self.end - self.start;
		let center = (self.start + self.end) * 0.5 + chord.perp() / (2.0 * (sweep * 0.5).tan());
		Some((center, chord.length() / (2.0 * (sweep * 0.5).sin().abs())))
	}

	pub(crate) fn position(self, parameter: f64) -> DVec2 {
		if parameter == 0.0 {
			return self.start;
		}
		if parameter == 1.0 {
			return self.end;
		}
		match self.shape {
			CurveShape::Straight => self.start.lerp(self.end, parameter),
			CurveShape::CircularArc { sweep } => {
				// Chord coordinates avoid subtracting a distant center for shallow arcs.
				let angle = sweep * parameter;
				let cotangent = 1.0 / (sweep * 0.5).tan();
				let one_minus_cos = 2.0 * (angle * 0.5).sin().powi(2);
				let chord = self.end - self.start;
				self.start
					+ chord * (one_minus_cos + cotangent * angle.sin()) * 0.5
					+ chord.perp() * (cotangent * one_minus_cos - angle.sin()) * 0.5
			}
		}
	}

	pub(crate) fn tangent(self, parameter: f64) -> DVec2 {
		let chord = self.end - self.start;
		match self.shape {
			CurveShape::Straight => chord.normalize(),
			CurveShape::CircularArc { sweep } => {
				let angle = sweep * (parameter - 0.5);
				DVec2::from_angle(angle).rotate(chord.normalize())
			}
		}
	}

	pub(crate) fn length(self) -> f64 {
		match self.shape {
			CurveShape::Straight => self.start.distance(self.end),
			CurveShape::CircularArc { sweep } => self.circle().unwrap().1 * sweep.abs(),
		}
	}

	pub(crate) fn closest_parameter(self, point: DVec2) -> f64 {
		match self.shape {
			CurveShape::Straight => {
				let chord = self.end - self.start;
				((point - self.start).dot(chord) / chord.length_squared()).clamp(0.0, 1.0)
			}
			CurveShape::CircularArc { .. } => {
				let (center, radius) = self.circle().expect("valid arc");
				let radial = point - center;
				let mut candidates = vec![0.0, 1.0];
				if radial.length_squared() > 0.0 {
					let projected = center + radial.normalize() * radius;
					if let Some(parameter) = self.parameter_of(projected) {
						candidates.push(parameter);
					}
				}
				candidates
					.into_iter()
					.min_by(|&left, &right| {
						self.position(left)
							.distance_squared(point)
							.total_cmp(&self.position(right).distance_squared(point))
					})
					.expect("arc has endpoints")
			}
		}
	}

	/// Endpoint proximity is handled separately by the graph to preserve existing node identities.
	pub(crate) fn parameter_of(self, point: DVec2) -> Option<f64> {
		match self.shape {
			CurveShape::Straight => {
				let chord = self.end - self.start;
				let parameter = (point - self.start).dot(chord) / chord.length_squared();
				((0.0..=1.0).contains(&parameter)
					&& self.position(parameter).distance(point) <= DISTANCE_TOLERANCE)
					.then_some(parameter)
			}
			CurveShape::CircularArc { sweep } => {
				if point.distance(self.start) <= DISTANCE_TOLERANCE {
					return Some(0.0);
				}
				if point.distance(self.end) <= DISTANCE_TOLERANCE {
					return Some(1.0);
				}
				let (center, radius) = self.circle().unwrap();
				if (point.distance(center) - radius).abs() > DISTANCE_TOLERANCE {
					return None;
				}
				let from = self.start - center;
				let to = point - center;
				let angle = from.perp_dot(to).atan2(from.dot(to));
				let directed_angle = (angle * sweep.signum()).rem_euclid(TAU);
				let parameter = directed_angle / sweep.abs();
				(parameter <= 1.0).then_some(parameter)
			}
		}
	}

	pub(crate) fn interior_parameter(self, point: DVec2) -> Option<f64> {
		if point.distance(self.start) <= DISTANCE_TOLERANCE
			|| point.distance(self.end) <= DISTANCE_TOLERANCE
		{
			return None;
		}
		self.parameter_of(point)
	}

	/// Maximum distance from the corresponding interval of a curve with the same shape.
	pub(crate) fn deviation_from(self, original: Curve, start: f64, end: f64) -> f64 {
		let error = Curve {
			start: self.start - original.position(start),
			end: self.end - original.position(end),
			shape: self.shape,
		};
		let mut maximum = error.start.length().max(error.end.length());
		if let CurveShape::CircularArc { sweep } = self.shape {
			// The error itself follows a circular arc: its norm is largest where its rotating
			// radius aligns with its center, if that angle lies in the swept interval.
			let chord = error.end - error.start;
			let center =
				(error.start + error.end) * 0.5 + chord.perp() / (2.0 * (sweep * 0.5).tan());
			let radial = error.start - center;
			let angle = (radial.perp_dot(center).atan2(radial.dot(center)) * sweep.signum())
				.rem_euclid(TAU);
			if angle <= sweep.abs() {
				maximum = maximum.max(error.position(angle / sweep.abs()).length());
			}
		}
		maximum
	}

	/// Number of chords needed to stay within the requested sagitta, before coordinate rounding.
	pub(crate) fn sample_count(self, deviation: f64) -> usize {
		match self.shape {
			CurveShape::Straight => 1,
			CurveShape::CircularArc { sweep } => {
				let radius = self.circle().unwrap().1;
				// asin is stable for small error/radius ratios, unlike acos(1 - error/radius).
				let step = 4.0 * (deviation / (2.0 * radius)).min(0.5).sqrt().asin();
				(sweep.abs() / step).ceil().max(1.0) as usize
			}
		}
	}
}

pub(crate) fn intersections(a: Curve, b: Curve) -> Intersections {
	let mut result = Intersections {
		contacts: Vec::new(),
		overlapping: false,
	};
	let mut points = Vec::new();
	match (a.shape, b.shape) {
		(CurveShape::Straight, CurveShape::Straight) => {
			let first = a.end - a.start;
			let second = b.end - b.start;
			// The longer segment provides a more stable reference line for near-collinear walls.
			let (long, short) = if first.length() >= second.length() {
				(a, b)
			} else {
				(b, a)
			};
			let direction = long.end - long.start;
			let project =
				|point: DVec2| (point - long.start).dot(direction) / direction.length_squared();
			let low = project(short.start).min(project(short.end)).max(0.0);
			let high = project(short.start).max(project(short.end)).min(1.0);
			let on_line = |point: DVec2| {
				(point - long.start).perp_dot(direction).abs() / direction.length()
					<= DISTANCE_TOLERANCE
			};
			result.overlapping = on_line(short.start)
				&& on_line(short.end)
				&& (high - low) * direction.length() > DISTANCE_TOLERANCE;
			let denominator = first.perp_dot(second);
			if denominator.abs() > f64::EPSILON * first.length() * second.length() {
				let parameter = (b.start - a.start).perp_dot(second) / denominator;
				points.push((a.position(parameter), false));
			}
		}
		(CurveShape::Straight, CurveShape::CircularArc { .. }) => {
			line_circle_points(a, b.circle().unwrap(), &mut points)
		}
		(CurveShape::CircularArc { .. }, CurveShape::Straight) => {
			line_circle_points(b, a.circle().unwrap(), &mut points)
		}
		(CurveShape::CircularArc { .. }, CurveShape::CircularArc { .. }) => {
			let (first_center, first_radius) = a.circle().unwrap();
			let (second_center, second_radius) = b.circle().unwrap();
			let delta = second_center - first_center;
			let distance = delta.length();
			if distance <= DISTANCE_TOLERANCE
				&& (first_radius - second_radius).abs() <= DISTANCE_TOLERANCE
			{
				let intervals = |curve: Curve, center: DVec2| {
					let CurveShape::CircularArc { sweep } = curve.shape else {
						unreachable!()
					};
					let start =
						((curve.start - center).to_angle() + sweep.min(0.0)).rem_euclid(TAU);
					[
						(start, (start + sweep.abs()).min(TAU)),
						(0.0, (start + sweep.abs() - TAU).max(0.0)),
					]
				};
				for first in intervals(a, first_center) {
					for second in intervals(b, first_center) {
						if (first.1.min(second.1) - first.0.max(second.0)) * first_radius
							> DISTANCE_TOLERANCE
						{
							result.overlapping = true;
						}
					}
				}
				points.extend([a.start, a.end, b.start, b.end].map(|point| (point, false)));
			} else if distance > 0.0 {
				let along = (first_radius.powi(2) - second_radius.powi(2) + distance.powi(2))
					/ (2.0 * distance);
				let height_squared = first_radius.powi(2) - along.powi(2);
				let roundoff = f64::EPSILON * 32.0 * first_radius.powi(2).max(along.powi(2));
				let near_tangent = (distance - first_radius - second_radius).abs()
					<= DISTANCE_TOLERANCE
					|| (distance - (first_radius - second_radius).abs()).abs()
						<= DISTANCE_TOLERANCE;
				if height_squared >= -roundoff || near_tangent {
					let axis = delta / distance;
					let base = first_center + axis * along;
					let offset = axis.perp() * height_squared.max(0.0).sqrt();
					if near_tangent {
						points.push((base, true));
					} else {
						points.extend([(base + offset, false), (base - offset, false)]);
					}
				}
			}
		}
	}
	for (point, near_tangent) in points {
		let (Some(first), Some(second)) = (a.parameter_of(point), b.parameter_of(point)) else {
			continue;
		};
		if result.contacts.iter().any(|contact: &CurveContact| {
			a.position(contact.first_parameter).distance(point) <= DISTANCE_TOLERANCE
		}) {
			continue;
		}
		result.contacts.push(CurveContact {
			first_parameter: first,
			second_parameter: second,
			tangential: near_tangent
				|| a.tangent(first).perp_dot(b.tangent(second)).abs() < ANGLE_TOLERANCE,
		});
	}
	result
		.contacts
		.sort_by(|a, b| a.first_parameter.total_cmp(&b.first_parameter));
	result
}

fn line_circle_points(
	line: Curve,
	(center, radius): (DVec2, f64),
	points: &mut Vec<(DVec2, bool)>,
) {
	let direction = (line.end - line.start).normalize();
	let along = (center - line.start).dot(direction);
	let closest = line.start + direction * along;
	let height_squared = radius.powi(2) - closest.distance_squared(center);
	let roundoff = f64::EPSILON * 32.0 * radius.powi(2);
	if height_squared >= -roundoff
		|| (radius - closest.distance(center)).abs() <= DISTANCE_TOLERANCE
	{
		if (radius - closest.distance(center)).abs() <= DISTANCE_TOLERANCE {
			// Resolve near-tangency in world-space units, not a scale-dependent discriminant.
			points.push((closest, true));
		} else {
			let offset = direction * height_squared.max(0.0).sqrt();
			points.extend([(closest + offset, false), (closest - offset, false)]);
		}
	}
}

#[cfg(test)]
#[path = "tests/curve.rs"]
mod tests;
