use std::collections::HashMap;

use wall_graph::{OpeningId, WallGraph};

/// What an opening is used for. Doors and windows share one geometry, so the wall graph
/// does not model the difference and the editor remembers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OpeningCategory {
	#[default]
	Door,
	Window,
}

/// The category of every opening in the graph it was built with.
///
/// Opening IDs stay stable while an opening's wall is split or merged, so the category
/// follows the opening through such edits without being touched. It must still leave with
/// an opening that the graph removes, which [`Self::retain_existing`] enforces.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OpeningCategories {
	by_opening: HashMap<OpeningId, OpeningCategory>,
}

impl OpeningCategories {
	pub fn get(&self, opening: OpeningId) -> Option<OpeningCategory> {
		self.by_opening.get(&opening).copied()
	}

	pub fn insert(&mut self, opening: OpeningId, category: OpeningCategory) {
		self.by_opening.insert(opening, category);
	}

	/// Drops categories of openings that `graph` no longer contains.
	pub fn retain_existing(&mut self, graph: &WallGraph) {
		self.by_opening
			.retain(|&opening, _| graph.opening(opening).is_some());
	}
}

#[cfg(test)]
#[path = "tests/opening_category.rs"]
mod tests;
