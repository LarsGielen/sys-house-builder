use std::{collections::HashMap, ops::RangeInclusive, sync::Mutex};

use wall_graph::{Wall, WallGraph};
use wall_mesh::WallMesh;

use crate::candidate::{self, EditCandidate};

/// A document state that history can return to, with the identity of its content.
struct Snapshot {
	candidate: EditCandidate,
	content_id: u64,
}

/// The committed graph and its generated mesh are replaced as one validated candidate.
///
/// Every replacement keeps the previous document for undo. Restoring one is a replacement too,
/// so the scene refreshes from `revision` the same way after a commit, an undo, and a redo.
#[derive(Default)]
pub struct House {
	committed: EditCandidate,
	revision: u64,
	undo_stack: Vec<Snapshot>,
	redo_stack: Vec<Snapshot>,
	/// Names the committed content. Every commit takes a fresh one and a restore brings back
	/// the old one, so equal IDs mean equal documents even after the history has branched.
	content_id: u64,
	next_content_id: u64,
	saved_content_id: u64,
	/// Computing a wall's bounds meshes the junctions of the whole house, and opening placement
	/// asks for them every frame, so they are kept until the next commit.
	opening_bounds: Mutex<HashMap<Wall, Option<RangeInclusive<f64>>>>,
}

impl House {
	pub fn committed(&self) -> &EditCandidate {
		&self.committed
	}

	pub fn graph(&self) -> &WallGraph {
		self.committed.graph()
	}

	pub fn mesh(&self) -> &WallMesh {
		self.committed.mesh()
	}

	pub fn revision(&self) -> u64 {
		self.revision
	}

	/// See [`candidate::opening_bounds`]. Cached, because the committed house cannot change between
	/// commits.
	pub fn opening_bounds(&self, wall: Wall) -> Option<RangeInclusive<f64>> {
		self.opening_bounds
			.lock()
			.expect("the cache holds no invariants a panic could break")
			.entry(wall)
			.or_insert_with(|| candidate::opening_bounds(self.graph(), wall))
			.clone()
	}

	/// Commits one edit as one undo step and discards any redo branch.
	pub fn commit(&mut self, candidate: EditCandidate) {
		self.next_content_id += 1;
		let previous = self.replace(candidate, self.next_content_id);
		self.undo_stack.push(previous);
		self.redo_stack.clear();
	}

	pub fn can_undo(&self) -> bool {
		!self.undo_stack.is_empty()
	}

	pub fn can_redo(&self) -> bool {
		!self.redo_stack.is_empty()
	}

	/// Returns whether there was anything to undo.
	pub fn undo(&mut self) -> bool {
		let Some(restored) = self.undo_stack.pop() else {
			return false;
		};
		let undone = self.replace(restored.candidate, restored.content_id);
		self.redo_stack.push(undone);
		true
	}

	/// Returns whether there was anything to redo.
	pub fn redo(&mut self) -> bool {
		let Some(restored) = self.redo_stack.pop() else {
			return false;
		};
		let redone = self.replace(restored.candidate, restored.content_id);
		self.undo_stack.push(redone);
		true
	}

	/// Whether the document differs from the last saved or opened one. A new house counts as
	/// saved, so only edits make it modified.
	pub fn is_modified(&self) -> bool {
		self.content_id != self.saved_content_id
	}

	pub fn mark_saved(&mut self) {
		self.saved_content_id = self.content_id;
	}

	/// Replaces the whole document, as when a project is opened or started. History belongs to
	/// the document it was recorded on, so it is dropped, and the new document counts as saved.
	pub fn replace_document(&mut self, candidate: EditCandidate) {
		self.next_content_id += 1;
		self.replace(candidate, self.next_content_id);
		self.undo_stack.clear();
		self.redo_stack.clear();
		self.mark_saved();
	}

	/// The one place the committed document changes, so the scene refresh and the cache
	/// invalidation cannot be missed by a new way of replacing it.
	fn replace(&mut self, candidate: EditCandidate, content_id: u64) -> Snapshot {
		self.revision += 1;
		self.opening_bounds
			.get_mut()
			.expect("the cache holds no invariants a panic could break")
			.clear();
		Snapshot {
			candidate: std::mem::replace(&mut self.committed, candidate),
			content_id: std::mem::replace(&mut self.content_id, content_id),
		}
	}
}

#[cfg(test)]
#[path = "tests/document.rs"]
mod tests;
