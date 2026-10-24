use std::ops::RangeInclusive;

use wall_graph::{MIN_OPENING_GAP, OpeningId, OpeningSpec, Wall, WallGraph};

// How close along the wall a snap candidate must be to take the opening's centre.
const SNAP_RADIUS_METRES: f64 = 0.15;

/// Configuration for snapping an opening along its host wall.
#[derive(Clone, Copy)]
pub struct OpeningSnapOptions {
	pub grid_spacing: f32,
	/// Gap kept beside a neighbour, raised to the graph's minimum when smaller.
	pub gap: f32,
	/// An opening being moved, which must not snap to its own old position.
	pub moving: Option<OpeningId>,
}

/// A place the opening's centre can snap to. Lower priorities win when several are in range.
struct SnapCandidate {
	center_distance: f64,
	priority: u8,
}

const STRUCTURAL_PRIORITY: u8 = 0;
const GRID_PRIORITY: u8 = 1;

/// Snaps to nearby wall ends, opening centres or sides, and free-span centres before
/// considering the grid. Structural candidates share a priority; the closest one wins. Everything is
/// measured along the host wall's path and stays within the stretch where junctions allow an
/// opening. `bounds` is [`crate::edit::opening_bounds`] for `wall`, which the caller supplies
/// because it is costly to compute.
pub fn snap_opening(
	graph: &WallGraph,
	wall: Wall,
	bounds: Option<RangeInclusive<f64>>,
	spec: OpeningSpec,
	options: OpeningSnapOptions,
) -> OpeningSpec {
	let (Some(length), Some(bounds)) = (graph.wall_length(wall), bounds) else {
		return spec;
	};
	let width = f64::from(spec.width);
	let raw = spec.center_distance;
	let in_range = |center_distance: f64| (center_distance - raw).abs() <= SNAP_RADIUS_METRES;
	let centers = (bounds.start() + width / 2.0)..=(bounds.end() - width / 2.0);
	if centers.is_empty() {
		return spec;
	}

	// Each neighbour as the stretch of wall it occupies, nearest the wall's start first.
	let mut neighbours: Vec<(f64, f64)> = graph
		.openings()
		.filter(|&(id, opening)| opening.wall == wall && Some(id) != options.moving)
		.map(|(_, opening)| {
			let half = f64::from(opening.spec.width) / 2.0;
			(
				opening.spec.center_distance - half,
				opening.spec.center_distance + half,
			)
		})
		.collect();
	neighbours.sort_by(|a, b| a.0.total_cmp(&b.0));

	// Both ends of the allowed stretch count as obstacles, so an empty wall has one span and
	// snaps to the middle of what is allowed.
	let mut spans = Vec::new();
	let mut free_from = *bounds.start();
	for &(start, end) in &neighbours {
		spans.push((free_from, start));
		free_from = free_from.max(end);
	}
	spans.push((free_from, *bounds.end()));

	// The pointer is attracted by the wall end itself, but a junction keeps the opening off
	// the end, so the snap lands on the nearest position the junction allows.
	let edges = [
		(width / 2.0, *centers.start()),
		(length - width / 2.0, *centers.end()),
	]
	.into_iter()
	.filter(|&(flush, _)| in_range(flush))
	.map(|(_, nearest_allowed)| nearest_allowed);
	let span_centers = spans
		.into_iter()
		// A span narrower than the opening cannot hold it, so centring in it only
		// leads to a rejected proposal.
		.filter(|(start, end)| end - start >= width)
		.map(|(start, end)| (start + end) / 2.0);
	let gap = f64::from(options.gap).max(MIN_OPENING_GAP);
	let beside_neighbours = neighbours.iter().flat_map(|&(start, end)| {
		[
			(start + end) / 2.0,
			end + gap + width / 2.0,
			start - gap - width / 2.0,
		]
	});
	let structural = edges
		.chain(
			span_centers
				.chain(beside_neighbours)
				.filter(|center_distance| centers.contains(center_distance))
				.filter(|&center_distance| in_range(center_distance)),
		)
		.map(|center_distance| SnapCandidate {
			center_distance,
			priority: STRUCTURAL_PRIORITY,
		});
	// Distances from the wall's start line up with the grid when its ends are on the grid.
	let spacing = f64::from(options.grid_spacing);
	let grid_distance = (raw / spacing).round() * spacing;
	let grid =
		(in_range(grid_distance) && centers.contains(&grid_distance)).then_some(SnapCandidate {
			center_distance: grid_distance,
			priority: GRID_PRIORITY,
		});

	let snapped = structural.chain(grid).min_by(|a, b| {
		a.priority.cmp(&b.priority).then(
			(a.center_distance - raw)
				.abs()
				.total_cmp(&(b.center_distance - raw).abs()),
		)
	});

	OpeningSpec {
		center_distance: snapped.map_or(raw, |candidate| candidate.center_distance),
		..spec
	}
}

#[cfg(test)]
#[path = "tests/opening_snap.rs"]
mod tests;
