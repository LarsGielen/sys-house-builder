use bevy::prelude::{Deref, Resource};
use house_document::EditCandidate;

/// The editor's committed document, shared with systems as a Bevy resource. The document logic
/// lives in `house_document`; this wrapper only lets it be a resource.
///
/// Reads go through `Deref`. Mutations are forwarded explicitly, so every way of changing the
/// document stays visible here and an edit can borrow the committed document while it commits.
#[derive(Resource, Default, Deref)]
pub struct House(house_document::House);

impl House {
	pub fn commit(&mut self, candidate: EditCandidate) {
		self.0.commit(candidate);
	}

	pub fn undo(&mut self) -> bool {
		self.0.undo()
	}

	pub fn redo(&mut self) -> bool {
		self.0.redo()
	}

	pub fn mark_saved(&mut self) {
		self.0.mark_saved();
	}

	pub fn replace_document(&mut self, candidate: EditCandidate) {
		self.0.replace_document(candidate);
	}
}
