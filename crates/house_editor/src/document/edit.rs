use bevy::prelude::{Deref, DerefMut, Mut, Resource};
pub use house_document::{
	ArcDefinition, ArcProposal, BendProposal, EditCandidate, EditError, MoveProposal,
	OpeningEditProposal, OpeningProposal, Proposal, WallDimensionsProposal, WallProposal,
	propose_opening_duplicate, propose_opening_removal,
};
// Production code only requests proposals through `EditPreview`; tests build documents directly.
#[cfg(test)]
pub use house_document::{propose_opening, propose_wall, propose_wall_removal};
pub use house_editor_core::{Preview, PreviewState};

/// Why the last edit that had no preview was refused, such as a duplicate with no room.
/// The next successful edit or a new selection clears it.
#[derive(Resource, Default)]
pub struct EditNotice {
	pub message: Option<String>,
}

/// The edit being proposed and what it would produce, shared between the tool that
/// proposes it and the views that display it. Committing takes exactly this candidate,
/// so the preview and the committed house cannot disagree.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct EditPreview(Preview);

impl EditPreview {
	/// Refreshes only when the proposal or its base document changed. Avoiding redundant
	/// writes also avoids rebuilding the rendered preview through Bevy's change tracking.
	pub fn update(
		mut preview: Mut<Self>,
		proposal: Proposal,
		house_revision: u64,
		committed: &EditCandidate,
	) {
		if preview.matches(proposal, house_revision) {
			return;
		}
		preview.refresh(proposal, house_revision, committed);
	}
}
