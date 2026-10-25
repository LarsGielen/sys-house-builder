use house_document::{
	EditCandidate, EditError, Proposal, propose_arc, propose_bend, propose_move, propose_opening,
	propose_opening_edit, propose_opening_removal, propose_wall, propose_wall_dimensions,
	propose_wall_removal,
};

pub struct PreviewState {
	pub house_revision: u64,
	pub proposal: Proposal,
	pub outcome: Result<EditCandidate, EditError>,
}

/// The edit being proposed and what it would produce, shared between the tool that
/// proposes it and the views that display it. Committing takes exactly this candidate,
/// so the preview and the committed house cannot disagree.
#[derive(Default)]
pub struct Preview {
	pub state: Option<PreviewState>,
}

impl Preview {
	pub fn matches(&self, proposal: Proposal, house_revision: u64) -> bool {
		self.state.as_ref().is_some_and(|state| {
			state.proposal == proposal && state.house_revision == house_revision
		})
	}

	/// Evaluates `proposal` against `committed` unless the preview already holds it for this
	/// revision. Returns whether the preview was replaced, so a Bevy wrapper can avoid marking
	/// the resource changed (and rebuilding the rendered preview) when it was not.
	pub fn refresh(&mut self, proposal: Proposal, house_revision: u64, committed: &EditCandidate) {
		let outcome = match proposal {
			Proposal::Wall(wall) => propose_wall(committed, &wall),
			Proposal::Arc(arc) => propose_arc(committed, &arc),
			Proposal::Move(edit) => propose_move(committed, &edit),
			Proposal::Bend(edit) => propose_bend(committed, &edit),
			Proposal::WallDimensions(edit) => propose_wall_dimensions(committed, &edit),
			Proposal::WallRemoval(wall) => propose_wall_removal(committed, wall),
			Proposal::OpeningRemoval(opening) => propose_opening_removal(committed, opening),
			Proposal::Opening(opening) => propose_opening(committed, &opening),
			Proposal::OpeningEdit(edit) => propose_opening_edit(committed, &edit),
		};
		self.state = Some(PreviewState {
			house_revision,
			proposal,
			outcome,
		});
	}

	/// Takes only the expected edit, validated against the current document. Invalid or
	/// stale candidates remain available to display their reason or to be refreshed.
	pub fn take_valid(&mut self, proposal: Proposal, house_revision: u64) -> Option<EditCandidate> {
		if !self.matches(proposal, house_revision) || self.valid_candidate().is_none() {
			return None;
		}
		self.state.take()?.outcome.ok()
	}

	/// Tools use these to clear only their own proposal, so that leaving one tool does not
	/// erase the preview of the tool that was just entered.
	pub fn holds_wall_proposal(&self) -> bool {
		matches!(
			self.state,
			Some(PreviewState {
				proposal: Proposal::Wall(_),
				..
			})
		)
	}

	pub fn holds_arc_proposal(&self) -> bool {
		matches!(
			self.state,
			Some(PreviewState {
				proposal: Proposal::Arc(_),
				..
			})
		)
	}

	/// A wall or junction drag from the selection tool's handles.
	pub fn holds_handle_drag(&self) -> bool {
		matches!(
			self.state,
			Some(PreviewState {
				proposal: Proposal::Move(_) | Proposal::Bend(_),
				..
			})
		)
	}

	/// A draft built by typing into a field, which Escape in the field discards.
	pub fn holds_field_draft(&self) -> bool {
		self.holds_opening_edit() || self.holds_wall_dimensions()
	}

	/// A deletion the delete tool previews under the pointer.
	pub fn holds_removal(&self) -> bool {
		matches!(
			self.state,
			Some(PreviewState {
				proposal: Proposal::WallRemoval(_) | Proposal::OpeningRemoval(_),
				..
			})
		)
	}

	pub fn holds_wall_removal(&self) -> bool {
		matches!(
			self.state,
			Some(PreviewState {
				proposal: Proposal::WallRemoval(_),
				..
			})
		)
	}

	pub fn holds_wall_dimensions(&self) -> bool {
		matches!(
			self.state,
			Some(PreviewState {
				proposal: Proposal::WallDimensions(_),
				..
			})
		)
	}

	pub fn holds_opening_edit(&self) -> bool {
		matches!(
			self.state,
			Some(PreviewState {
				proposal: Proposal::OpeningEdit(_),
				..
			})
		)
	}

	pub fn holds_opening_proposal(&self) -> bool {
		matches!(
			self.state,
			Some(PreviewState {
				proposal: Proposal::Opening(_),
				..
			})
		)
	}

	pub fn valid_candidate(&self) -> Option<&EditCandidate> {
		self.state.as_ref()?.outcome.as_ref().ok()
	}

	/// The candidate whose mesh replaces the committed walls on screen. A deletion is shown
	/// by tinting what would go instead, so the house around it stays in view.
	pub fn candidate_to_display(&self) -> Option<&EditCandidate> {
		if self.holds_removal() {
			None
		} else {
			self.valid_candidate()
		}
	}
}

#[cfg(test)]
#[path = "../tests/workflow/preview.rs"]
mod tests;
