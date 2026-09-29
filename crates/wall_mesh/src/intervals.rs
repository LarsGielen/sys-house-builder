//! Sets of solid elevation intervals, in metres above the floor.

/// Elevations closer than this are the same level. Opening tops are sums of `f32` values,
/// so `0.9 + 1.2` must still meet a wall top of `2.1`.
pub(crate) const ELEVATION_TOLERANCE: f64 = 1e-5;

/// Sorted, disjoint, non-empty `(bottom, top)` intervals.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Intervals(Vec<(f64, f64)>);

impl Intervals {
	pub(crate) fn span(bottom: f64, top: f64) -> Self {
		if top > bottom {
			Self(vec![(bottom, top)])
		} else {
			Self::default()
		}
	}

	pub(crate) fn subtract(&self, bottom: f64, top: f64) -> Self {
		let mut result = Vec::with_capacity(self.0.len() + 1);
		for &(low, high) in &self.0 {
			if top <= low || bottom >= high {
				result.push((low, high));
				continue;
			}
			if bottom > low {
				result.push((low, bottom));
			}
			if top < high {
				result.push((top, high));
			}
		}
		Self(result)
	}

	pub(crate) fn difference(&self, other: &Self) -> Self {
		other
			.0
			.iter()
			.fold(self.clone(), |remaining, &(low, high)| {
				remaining.subtract(low, high)
			})
	}

	/// Moves every bound to a canonical level, dropping intervals that collapse and joining
	/// intervals that now touch.
	pub(crate) fn snapped(&self, levels: &Levels) -> Self {
		let mut result: Vec<(f64, f64)> = Vec::with_capacity(self.0.len());
		for &(low, high) in &self.0 {
			let (low, high) = (levels.snap(low), levels.snap(high));
			if high <= low {
				continue;
			}
			match result.last_mut() {
				Some(last) if last.1 >= low => last.1 = last.1.max(high),
				_ => result.push((low, high)),
			}
		}
		Self(result)
	}

	pub(crate) fn iter(&self) -> impl Iterator<Item = (f64, f64)> + '_ {
		self.0.iter().copied()
	}

	pub(crate) fn bounds(&self) -> impl Iterator<Item = f64> + '_ {
		self.0.iter().flat_map(|&(low, high)| [low, high])
	}
}

/// Clusters of nearly equal elevations. Every member maps to its cluster's lowest value, so
/// faces from different cells meet at exactly the same height.
pub(crate) struct Levels {
	clusters: Vec<(f64, f64)>,
}

impl Levels {
	pub(crate) fn new(values: impl Iterator<Item = f64>) -> Self {
		let mut values: Vec<f64> = values.collect();
		values.sort_by(f64::total_cmp);
		let mut clusters: Vec<(f64, f64)> = Vec::new();
		for value in values {
			match clusters.last_mut() {
				Some(cluster) if value - cluster.1 <= ELEVATION_TOLERANCE => cluster.1 = value,
				_ => clusters.push((value, value)),
			}
		}
		Self { clusters }
	}

	/// Only values given to [`Levels::new`] may be snapped.
	pub(crate) fn snap(&self, value: f64) -> f64 {
		let index = self.clusters.partition_point(|cluster| cluster.1 < value);
		self.clusters[index].0
	}
}

#[cfg(test)]
#[path = "tests/intervals.rs"]
mod tests;
