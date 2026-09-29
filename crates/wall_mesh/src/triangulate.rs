//! Ear clipping for the simple polygons of footprint cells.

use glam::DVec2;

/// Twice the area, in square metres, below which a triangle counts as degenerate.
const MIN_DOUBLE_AREA: f64 = 1e-14;
/// Twice the area within which a vertex counts as lying on a triangle's edge. Junction splits
/// place vertices on edges only to within rounding error.
const ON_EDGE_DOUBLE_AREA: f64 = 1e-12;

/// Triangulates a simple counterclockwise polygon, returning indices into `points`.
///
/// Collinear vertices are kept: they are shared with neighbouring faces, so dropping them would
/// leave cracks. No zero-area triangle is produced; `None` means no valid ear was found.
pub(crate) fn triangulate(points: &[DVec2]) -> Option<Vec<[usize; 3]>> {
	if points.len() < 3 {
		return None;
	}
	let mut remaining: Vec<usize> = (0..points.len()).collect();
	let mut triangles = Vec::with_capacity(points.len() - 2);
	let mut cursor = 0;
	let mut failures = 0;
	while remaining.len() > 3 {
		let count = remaining.len();
		let previous = remaining[(cursor + count - 1) % count];
		let current = remaining[cursor];
		let next = remaining[(cursor + 1) % count];
		if is_ear(points, &remaining, previous, current, next) {
			triangles.push([previous, current, next]);
			remaining.remove(cursor);
			cursor %= remaining.len();
			failures = 0;
		} else {
			cursor = (cursor + 1) % count;
			failures += 1;
			if failures > count {
				return None;
			}
		}
	}
	let [a, b, c] = [remaining[0], remaining[1], remaining[2]];
	if double_area(points[a], points[b], points[c]) <= MIN_DOUBLE_AREA {
		return None;
	}
	triangles.push([a, b, c]);
	Some(triangles)
}

fn double_area(a: DVec2, b: DVec2, c: DVec2) -> f64 {
	(b - a).perp_dot(c - a)
}

fn is_ear(
	points: &[DVec2],
	remaining: &[usize],
	previous: usize,
	current: usize,
	next: usize,
) -> bool {
	let (a, b, c) = (points[previous], points[current], points[next]);
	if double_area(a, b, c) <= MIN_DOUBLE_AREA {
		return false;
	}
	// A vertex on the ear's boundary would become a T-junction, so it also blocks the ear.
	!remaining.iter().any(|&index| {
		if index == previous || index == current || index == next {
			return false;
		}
		let p = points[index];
		double_area(a, b, p) >= -ON_EDGE_DOUBLE_AREA
			&& double_area(b, c, p) >= -ON_EDGE_DOUBLE_AREA
			&& double_area(c, a, p) >= -ON_EDGE_DOUBLE_AREA
	})
}

#[cfg(test)]
#[path = "tests/triangulate.rs"]
mod tests;
