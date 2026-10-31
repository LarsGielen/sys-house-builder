//! Solves for the signed sweep of a circular arc from the constraints a drawing tool collects.
//!
//! The graph's arcs are defined by two endpoints and a signed sweep, so each solver answers
//! one question: which sweep satisfies these constraints? An arc leaves its start at
//! `-sweep / 2` from the chord and arrives at `+sweep / 2`, which makes every tangent
//! constraint a plain angle comparison. A returned sweep of exactly `0.0` means the
//! constraints describe a straight wall.
use std::f64::consts::{PI, TAU};
use std::fmt;

use glam::{DVec2, Vec2};

use crate::geometry::{ANGLE_TOLERANCE, DISTANCE_TOLERANCE};

/// Below this the bend or tangent lies too near the chord to tell an arc from a line.
const STRAIGHT_ANGLE_TOLERANCE: f64 = ANGLE_TOLERANCE;
/// How far apart the two tangent constraints may disagree and still describe one circle.
const TANGENT_CONSISTENCY_TOLERANCE: f64 = 1e-4;

/// Why no arc satisfies the requested constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArcError {
	/// A position is not finite, or the endpoints are within the distance tolerance.
	InvalidPosition,
	/// A tangent direction is not finite or has zero length.
	InvalidDirection,
	/// The bend lies on the line through the endpoints, so it fixes no circle.
	BendOnChord,
	/// The endpoint lies behind the tangent, which would need a full revolution or more.
	EndpointBehindTangent,
	/// The two tangents do not belong to one circle through both endpoints.
	InconsistentTangents,
	/// The radius cannot span the chord.
	RadiusTooSmall,
	/// The radius is not finite and positive.
	InvalidRadius,
	/// A tangent mode needs a wall end to continue from.
	MissingConnection,
}

impl fmt::Display for ArcError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(match self {
			Self::InvalidPosition => "arc endpoints must be finite and apart",
			Self::InvalidDirection => "a tangent direction must be finite and nonzero",
			Self::BendOnChord => "the bend is on the line between the endpoints",
			Self::EndpointBehindTangent => {
				"the end lies directly behind the connecting wall, so no arc reaches it"
			}
			Self::InconsistentTangents => {
				"the two connecting walls do not meet on one circular arc"
			}
			Self::RadiusTooSmall => "the radius is too small to span the endpoints",
			Self::InvalidRadius => "the radius must be finite and positive",
			Self::MissingConnection => "a tangent arc must start and end on a free wall end",
		})
	}
}

impl std::error::Error for ArcError {}

fn chord(start: Vec2, end: Vec2) -> Result<DVec2, ArcError> {
	if !start.is_finite() || !end.is_finite() {
		return Err(ArcError::InvalidPosition);
	}
	let chord = end.as_dvec2() - start.as_dvec2();
	if chord.length() <= DISTANCE_TOLERANCE {
		return Err(ArcError::InvalidPosition);
	}
	Ok(chord)
}

fn direction(direction: Vec2) -> Result<DVec2, ArcError> {
	if !direction.is_finite() || direction.length_squared() == 0.0 {
		return Err(ArcError::InvalidDirection);
	}
	Ok(direction.as_dvec2().normalize())
}

/// The signed angle from `from` to `to`, in `(-PI, PI]`.
fn signed_angle(from: DVec2, to: DVec2) -> f64 {
	from.perp_dot(to).atan2(from.dot(to))
}

fn finished_sweep(sweep: f64) -> Result<f32, ArcError> {
	// A sweep this close to a full revolution is the endpoint directly behind the tangent.
	if sweep.abs() >= TAU - 1e-3 {
		return Err(ArcError::EndpointBehindTangent);
	}
	if sweep.abs() <= STRAIGHT_ANGLE_TOLERANCE {
		return Ok(0.0);
	}
	Ok(sweep as f32)
}

/// The arc from `start` to `end` that passes through `bend`.
pub fn arc_through_bend(start: Vec2, bend: Vec2, end: Vec2) -> Result<f32, ArcError> {
	let chord = chord(start, end)?;
	if !bend.is_finite() {
		return Err(ArcError::InvalidPosition);
	}
	let to_bend = bend.as_dvec2() - start.as_dvec2();
	let distance_from_chord = chord.normalize().perp_dot(to_bend).abs();
	if distance_from_chord <= DISTANCE_TOLERANCE {
		return Err(ArcError::BendOnChord);
	}

	// Inscribed angle theorem: the chord seen from the bend spans half of the arc that does
	// not contain the bend.
	let seen = signed_angle(
		start.as_dvec2() - bend.as_dvec2(),
		end.as_dvec2() - bend.as_dvec2(),
	);
	// A bend left of the chord means the arc bulges left, which is clockwise.
	let magnitude = TAU - 2.0 * seen.abs();
	let sweep = if chord.perp_dot(to_bend) > 0.0 {
		-magnitude
	} else {
		magnitude
	};
	finished_sweep(sweep)
}

/// The arc that leaves `start` along `start_direction` and reaches `end`.
pub fn arc_tangent_at_start(
	start: Vec2,
	start_direction: Vec2,
	end: Vec2,
) -> Result<f32, ArcError> {
	let chord = chord(start, end)?;
	let direction = direction(start_direction)?;
	finished_sweep(2.0 * signed_angle(direction, chord))
}

/// The arc that leaves `start` along `start_direction` and arrives at `end` travelling along
/// `end_direction`. Both directions point the way the new wall travels.
pub fn arc_tangent_at_both(
	start: Vec2,
	start_direction: Vec2,
	end: Vec2,
	end_direction: Vec2,
) -> Result<f32, ArcError> {
	let chord = chord(start, end)?;
	let start_direction = direction(start_direction)?;
	let end_direction = direction(end_direction)?;
	let leaving = signed_angle(start_direction, chord);
	let arriving = signed_angle(chord, end_direction);
	if (leaving - arriving).abs() > TANGENT_CONSISTENCY_TOLERANCE
		// `PI` and `-PI` are the same direction.
		&& (leaving.abs() - PI).abs() + (arriving.abs() - PI).abs() > TANGENT_CONSISTENCY_TOLERANCE
	{
		return Err(ArcError::InconsistentTangents);
	}
	finished_sweep(leaving + arriving)
}

/// The arc from `start` to `end` with the given radius. A chord has two minor and two major
/// arcs; `counterclockwise` and `major` choose one.
pub fn arc_sweep_for_radius(
	start: Vec2,
	end: Vec2,
	radius: f32,
	counterclockwise: bool,
	major: bool,
) -> Result<f32, ArcError> {
	let length = chord(start, end)?.length();
	if !radius.is_finite() || radius <= 0.0 {
		return Err(ArcError::InvalidRadius);
	}
	let ratio = length / (2.0 * f64::from(radius));
	if ratio > 1.0 {
		return Err(ArcError::RadiusTooSmall);
	}
	let minor = 2.0 * ratio.asin();
	let magnitude = if major { TAU - minor } else { minor };
	let sweep = if counterclockwise {
		magnitude
	} else {
		-magnitude
	};
	finished_sweep(sweep)
}

/// The radius of the arc from `start` to `end` with `sweep`, or `None` for a straight wall or
/// invalid input.
pub fn arc_radius(start: Vec2, end: Vec2, sweep: f32) -> Option<f32> {
	let length = chord(start, end).ok()?.length();
	let half_sweep = f64::from(sweep).abs() / 2.0;
	(sweep.is_finite() && half_sweep.sin() > 0.0)
		.then(|| (length / (2.0 * half_sweep.sin())) as f32)
}

/// The point halfway along the arc, which a bend handle sits on.
pub fn arc_midpoint(start: Vec2, end: Vec2, sweep: f32) -> Vec2 {
	let midpoint = (start + end) / 2.0;
	if sweep == 0.0 {
		return midpoint;
	}
	// The sagitta lies on the side the arc bulges toward: right of the chord for a
	// counterclockwise sweep.
	let chord = (end - start).as_dvec2();
	let half_sweep = f64::from(sweep) / 2.0;
	let radius = chord.length() / (2.0 * half_sweep.sin().abs());
	let center_offset = radius * half_sweep.cos();
	let normal = -chord.perp().normalize() * f64::from(sweep.signum());
	midpoint + (normal * (radius - center_offset)).as_vec2()
}

/// Points along the arc from `start` to `end` with `sweep`, including both endpoints, close
/// enough to the true curve that no chord deviates more than `max_deviation` metres. A zero
/// sweep gives the straight chord.
///
/// For drawing proposals that are not in the graph, such as a preview that is still invalid.
pub fn arc_points(start: Vec2, end: Vec2, sweep: f32, max_deviation: f32) -> Vec<Vec2> {
	use crate::geometry::{Curve, CurveShape};

	let shape = if sweep == 0.0 {
		CurveShape::Straight
	} else {
		CurveShape::CircularArc {
			sweep: f64::from(sweep),
		}
	};
	let curve = Curve::new(start, end, shape);
	if !curve.valid() || !max_deviation.is_finite() || max_deviation <= 0.0 {
		return vec![start, end];
	}
	let count = curve.sample_count(f64::from(max_deviation)).clamp(1, 4096);
	(0..=count)
		.map(|index| curve.position(index as f64 / count as f64).as_vec2())
		.collect()
}
