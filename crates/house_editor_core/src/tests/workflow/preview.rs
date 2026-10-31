use super::*;
use glam::Vec2;
use house_document::{WallProposal, propose_wall};
use wall_graph::WallDimensions;

#[test]
fn taking_a_preview_requires_its_proposal_and_current_revision() {
	let proposal = Proposal::Wall(WallProposal {
		start: Vec2::ZERO,
		end: Vec2::X,
		dimensions: WallDimensions::default(),
	});
	let committed = EditCandidate::default();
	let mut preview = Preview {
		state: Some(PreviewState {
			house_revision: 2,
			proposal,
			outcome: propose_wall(
				&committed,
				&WallProposal {
					start: Vec2::ZERO,
					end: Vec2::X,
					dimensions: WallDimensions::default(),
				},
			),
		}),
	};
	assert!(preview.take_valid(proposal, 3).is_none());
	assert!(preview.state.is_some());
	let different = Proposal::Wall(WallProposal {
		start: Vec2::ZERO,
		end: Vec2::Y,
		dimensions: WallDimensions::default(),
	});
	assert!(preview.take_valid(different, 2).is_none());
	assert!(preview.take_valid(proposal, 2).is_some());
	assert!(preview.state.is_none());
}
