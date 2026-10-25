use super::*;
use glam::Vec2;
use house_document::{EditCandidate, WallProposal, propose_wall};
use wall_graph::WallDimensions;

fn straight_wall() -> EditCandidate {
	propose_wall(
		&EditCandidate::default(),
		&WallProposal {
			start: Vec2::ZERO,
			end: Vec2::new(6.0, 0.0),
			dimensions: WallDimensions::default(),
		},
	)
	.unwrap()
}

#[test]
fn a_ray_onto_a_wall_proposes_an_opening_centred_there_with_the_defaults() {
	let candidate = straight_wall();
	let mut defaults = OpeningDefaults::default();
	defaults.set_category(OpeningCategory::Window);
	let proposal = propose_opening_at_ray(
		candidate.graph(),
		candidate.mesh(),
		&defaults,
		Vec3::new(2.5, 0.0, 5.0),
		-Vec3::Z,
	)
	.unwrap();
	assert!((proposal.spec.center_distance - 2.5).abs() < 1e-3);
	assert_eq!(proposal.category, OpeningCategory::Window);
	assert!((proposal.spec.width - 0.9).abs() < 1e-6);
}

#[test]
fn a_ray_that_misses_every_wall_proposes_nothing() {
	let candidate = straight_wall();
	assert!(
		propose_opening_at_ray(
			candidate.graph(),
			candidate.mesh(),
			&OpeningDefaults::default(),
			Vec3::new(2.5, 5.0, 5.0),
			-Vec3::Z,
		)
		.is_none()
	);
}
