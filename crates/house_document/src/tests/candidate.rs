use super::*;

#[test]
fn duplicate_wall_proposal_is_rejected() {
	let committed = candidate_with_wall(Vec2::ZERO, Vec2::X);
	let proposal = WallProposal {
		start: Vec2::ZERO,
		end: Vec2::X,
		dimensions: WallDimensions::default(),
	};
	assert!(propose_wall(&committed, &proposal).is_err());
}

#[test]
fn wall_proposal_generates_matching_graph_and_mesh() {
	let committed = EditCandidate::default();
	let proposal = WallProposal {
		start: Vec2::ZERO,
		end: Vec2::X,
		dimensions: WallDimensions::default(),
	};
	let candidate = propose_wall(&committed, &proposal).unwrap();
	assert_eq!(candidate.graph().walls().count(), 1);
	assert_eq!(
		candidate.mesh(),
		&wall_mesh::generate(candidate.graph(), &MeshSettings::default()).unwrap()
	);
}

fn candidate_with_wall(start: Vec2, end: Vec2) -> EditCandidate {
	let proposal = WallProposal {
		start,
		end,
		dimensions: WallDimensions::default(),
	};
	propose_wall(&EditCandidate::default(), &proposal).unwrap()
}

fn door(wall: Wall, center_distance: f64, width: f32) -> OpeningProposal {
	opening(wall, center_distance, width, OpeningCategory::Door)
}

fn opening(
	wall: Wall,
	center_distance: f64,
	width: f32,
	category: OpeningCategory,
) -> OpeningProposal {
	OpeningProposal {
		wall,
		spec: OpeningSpec {
			center_distance,
			width,
			bottom: 0.0,
			height: 2.1,
		},
		category,
	}
}

fn only_wall(candidate: &EditCandidate) -> Wall {
	candidate.graph().walls().next().unwrap()
}

#[test]
fn opening_proposal_generates_matching_geometry_and_category() {
	let committed = candidate_with_wall(Vec2::ZERO, Vec2::new(4.0, 0.0));
	let wall = only_wall(&committed);
	let candidate = propose_opening(
		&committed,
		&opening(wall, 2.0, 0.9, OpeningCategory::Window),
	)
	.unwrap();
	assert_eq!(candidate.graph().openings().count(), 1);
	let (id, opening) = candidate.graph().openings().next().unwrap();
	assert_eq!(opening.spec.center_distance, 2.0);
	assert_eq!(
		candidate.categories().get(id),
		Some(OpeningCategory::Window)
	);
	assert_eq!(
		candidate.mesh(),
		&wall_mesh::generate(candidate.graph(), &MeshSettings::default()).unwrap()
	);
}

#[test]
fn opening_wider_than_its_wall_is_rejected() {
	let committed = candidate_with_wall(Vec2::ZERO, Vec2::new(4.0, 0.0));
	let wall = only_wall(&committed);
	assert_eq!(
		propose_opening(&committed, &door(wall, 2.0, 5.0)).err(),
		Some(EditError::Graph(WallError::InvalidOpening))
	);
}

#[test]
fn identical_openings_keep_their_categories_through_splits() {
	let mut document = candidate_with_wall(Vec2::ZERO, Vec2::new(8.0, 0.0));
	let wall = only_wall(&document);
	let door_proposal = door(wall, 1.5, 0.9);
	document = propose_opening(&document, &door_proposal).unwrap();
	let wall = only_wall(&document);
	let window_proposal = opening(wall, 6.5, 0.9, OpeningCategory::Window);
	document = propose_opening(&document, &window_proposal).unwrap();

	// A crossing wall in the middle splits the host between the two openings.
	let crossing = WallProposal {
		start: Vec2::new(4.0, -2.0),
		end: Vec2::new(4.0, 2.0),
		dimensions: WallDimensions::default(),
	};
	let split = propose_wall(&document, &crossing).unwrap();

	assert_eq!(split.graph().walls().count(), 4);
	let category_at = |x: f32| {
		let (id, _) = split
			.graph()
			.openings()
			.find(|&(id, _)| split.graph().opening_position(id).unwrap().x == x)
			.unwrap();
		split.categories().get(id)
	};
	assert_eq!(category_at(1.5), Some(OpeningCategory::Door));
	assert_eq!(category_at(6.5), Some(OpeningCategory::Window));
}

fn candidate_with_window(length: f32, center_distance: f64) -> (EditCandidate, OpeningId) {
	let committed = candidate_with_wall(Vec2::ZERO, Vec2::new(length, 0.0));
	let wall = only_wall(&committed);
	let candidate = propose_opening(
		&committed,
		&opening(wall, center_distance, 0.9, OpeningCategory::Window),
	)
	.unwrap();
	let (id, _) = candidate.graph().openings().next().unwrap();
	(candidate, id)
}

#[test]
fn editing_an_opening_keeps_its_identity_and_category() {
	let (committed, id) = candidate_with_window(6.0, 3.0);
	let spec = OpeningSpec {
		center_distance: 4.0,
		width: 1.2,
		bottom: 0.9,
		height: 1.0,
	};

	let edited =
		propose_opening_edit(&committed, &OpeningEditProposal { opening: id, spec }).unwrap();

	assert_eq!(edited.graph().opening(id).unwrap().spec, spec);
	assert_eq!(edited.categories().get(id), Some(OpeningCategory::Window));
	assert_eq!(
		edited.mesh(),
		&wall_mesh::generate(edited.graph(), &MeshSettings::default()).unwrap()
	);
}

#[test]
fn moving_an_opening_into_a_junction_is_rejected() {
	let committed = candidate_with_wall(Vec2::ZERO, Vec2::new(6.0, 0.0));
	let wall = only_wall(&committed);
	// A second wall joins the first at its start, so an opening there crowds the junction.
	let mut document = propose_wall(
		&committed,
		&WallProposal {
			start: Vec2::ZERO,
			end: Vec2::new(0.0, 3.0),
			dimensions: WallDimensions::default(),
		},
	)
	.unwrap();
	document =
		propose_opening(&document, &opening(wall, 3.0, 0.9, OpeningCategory::Window)).unwrap();
	let (id, original) = document.graph().openings().next().unwrap();

	let crowded = OpeningSpec {
		center_distance: 0.5,
		..original.spec
	};
	let error = propose_opening_edit(
		&document,
		&OpeningEditProposal {
			opening: id,
			spec: crowded,
		},
	)
	.err();

	assert_eq!(
		error,
		Some(EditError::Graph(WallError::OpeningTooCloseToJunction))
	);
}

#[test]
fn removing_an_opening_removes_its_category_in_the_same_candidate() {
	let (committed, id) = candidate_with_window(6.0, 3.0);

	let removed = propose_opening_removal(&committed, id).unwrap();

	assert!(removed.graph().opening(id).is_none());
	assert_eq!(removed.categories(), &OpeningCategories::default());
}

#[test]
fn a_copy_is_a_new_opening_with_the_same_category_beside_the_original() {
	let (committed, id) = candidate_with_window(8.0, 3.0);

	let (copied, copy) = propose_opening_duplicate(&committed, id, 0.0).unwrap();

	assert_ne!(copy, id);
	assert_eq!(copied.categories().get(copy), Some(OpeningCategory::Window));
	assert_eq!(copied.categories().get(id), Some(OpeningCategory::Window));
	let copy_spec = copied.graph().opening(copy).unwrap().spec;
	assert_eq!(copy_spec.width, 0.9);
	assert!((copy_spec.center_distance - 3.0).abs() >= 0.9 + wall_graph::MIN_OPENING_GAP - 1e-6);
}

#[test]
fn a_copy_that_does_not_fit_is_refused() {
	let (committed, id) = candidate_with_window(1.4, 0.7);

	let error = propose_opening_duplicate(&committed, id, 0.0).err();

	assert!(error.is_some());
}

#[test]
fn changing_wall_dimensions_updates_the_graph_and_mesh() {
	let committed = candidate_with_wall(Vec2::ZERO, Vec2::new(4.0, 0.0));
	let wall = only_wall(&committed);
	let dimensions = WallDimensions {
		thickness: 0.3,
		height: 3.0,
	};

	let changed =
		propose_wall_dimensions(&committed, &WallDimensionsProposal { wall, dimensions }).unwrap();

	assert_eq!(changed.graph().wall_dimensions(wall), Some(dimensions));
	assert_ne!(changed.mesh(), committed.mesh());
}

#[test]
fn zero_wall_height_is_rejected() {
	let committed = candidate_with_wall(Vec2::ZERO, Vec2::new(4.0, 0.0));
	let wall = only_wall(&committed);
	let flat = WallDimensions {
		thickness: 0.2,
		height: 0.0,
	};

	let error = propose_wall_dimensions(
		&committed,
		&WallDimensionsProposal {
			wall,
			dimensions: flat,
		},
	)
	.err();

	assert_eq!(error, Some(EditError::Graph(WallError::InvalidDimensions)));
}

#[test]
fn a_thicker_neighbour_that_crowds_an_opening_is_rejected() {
	let mut document = candidate_with_wall(Vec2::ZERO, Vec2::new(3.0, 0.0));
	let host = only_wall(&document);
	document = propose_wall(
		&document,
		&WallProposal {
			start: Vec2::ZERO,
			end: Vec2::new(0.0, 2.0),
			dimensions: WallDimensions::default(),
		},
	)
	.unwrap();
	document =
		propose_opening(&document, &opening(host, 0.7, 1.0, OpeningCategory::Window)).unwrap();
	let neighbour = document.graph().walls().find(|&wall| wall != host).unwrap();

	let error = propose_wall_dimensions(
		&document,
		&WallDimensionsProposal {
			wall: neighbour,
			dimensions: WallDimensions {
				thickness: 0.4,
				height: 2.5,
			},
		},
	)
	.err();

	assert_eq!(
		error,
		Some(EditError::Graph(WallError::OpeningTooCloseToJunction))
	);
}

#[test]
fn removing_a_wall_removes_its_openings_and_their_categories_together() {
	let mut document = candidate_with_wall(Vec2::ZERO, Vec2::new(6.0, 0.0));
	let host = only_wall(&document);
	document = propose_opening(&document, &opening(host, 1.5, 0.9, OpeningCategory::Door)).unwrap();
	document =
		propose_opening(&document, &opening(host, 4.5, 0.9, OpeningCategory::Window)).unwrap();
	assert_eq!(document.graph().openings().count(), 2);

	let removed = propose_wall_removal(&document, host).unwrap();

	assert_eq!(removed.graph().walls().count(), 0);
	assert_eq!(removed.graph().openings().count(), 0);
	assert_eq!(removed.categories(), &OpeningCategories::default());
}

#[test]
fn removing_a_stale_wall_is_rejected() {
	let committed = candidate_with_wall(Vec2::ZERO, Vec2::new(4.0, 0.0));
	let wall = only_wall(&committed);
	let removed = propose_wall_removal(&committed, wall).unwrap();

	assert_eq!(
		propose_wall_removal(&removed, wall).err(),
		Some(EditError::Graph(WallError::UnknownWall))
	);
}

fn arc_proposal(start: Vec2, end: Vec2, definition: ArcDefinition) -> ArcProposal {
	ArcProposal {
		start,
		end,
		definition,
		dimensions: WallDimensions::default(),
	}
}

#[test]
fn a_free_arc_through_a_bend_commits_a_valid_curved_wall() {
	let proposal = arc_proposal(
		Vec2::ZERO,
		Vec2::new(4.0, 0.0),
		ArcDefinition::Bend(Vec2::new(2.0, 1.0)),
	);
	let candidate = propose_arc(&EditCandidate::default(), &proposal).unwrap();
	let wall = candidate.graph().walls().next().unwrap();
	assert!(matches!(
		candidate.graph().wall_curve(wall),
		Some(wall_graph::WallCurve::CircularArc { .. })
	));
	assert_eq!(
		candidate.mesh(),
		&wall_mesh::generate(candidate.graph(), &MeshSettings::default()).unwrap()
	);
}

#[test]
fn a_bend_on_the_chord_is_an_arc_error_and_leaves_the_house_alone() {
	let committed = candidate_with_wall(Vec2::ZERO, Vec2::new(0.0, 5.0));
	let proposal = arc_proposal(
		Vec2::new(2.0, 0.0),
		Vec2::new(4.0, 0.0),
		ArcDefinition::Bend(Vec2::new(3.0, 0.0)),
	);
	assert_eq!(
		propose_arc(&committed, &proposal).err(),
		Some(EditError::Arc(wall_graph::ArcError::BendOnChord))
	);
	assert_eq!(committed.graph().walls().count(), 1);
}

#[test]
fn a_tangent_arc_meets_the_wall_it_continues_smoothly() {
	let committed = candidate_with_wall(Vec2::ZERO, Vec2::new(2.0, 0.0));
	let proposal = arc_proposal(
		Vec2::new(2.0, 0.0),
		Vec2::new(4.0, 2.0),
		ArcDefinition::OneTangent {
			start_direction: Vec2::X,
		},
	);
	let candidate = propose_arc(&committed, &proposal).unwrap();
	assert_eq!(candidate.graph().walls().count(), 2);
	assert_eq!(candidate.graph().smooth_joins().len(), 1);
}

#[test]
fn a_tangent_arc_straight_ahead_becomes_a_straight_wall() {
	let committed = candidate_with_wall(Vec2::ZERO, Vec2::new(2.0, 0.0));
	let proposal = arc_proposal(
		Vec2::new(2.0, 0.0),
		Vec2::new(4.0, 0.0),
		ArcDefinition::OneTangent {
			start_direction: Vec2::X,
		},
	);
	// Collinear and compatible, so the new piece merges into one wall.
	let candidate = propose_arc(&committed, &proposal).unwrap();
	assert_eq!(candidate.graph().walls().count(), 1);
}

#[test]
fn two_tangent_requests_that_cannot_be_met_are_errors_not_free_arcs() {
	let committed = candidate_with_wall(Vec2::ZERO, Vec2::new(2.0, 0.0));
	let inconsistent = arc_proposal(
		Vec2::new(2.0, 0.0),
		Vec2::new(6.0, 0.0),
		ArcDefinition::TwoTangents {
			start_direction: Vec2::X,
			end_direction: Some(Vec2::new(1.0, 1.0)),
		},
	);
	assert_eq!(
		propose_arc(&committed, &inconsistent).err(),
		Some(EditError::Arc(wall_graph::ArcError::InconsistentTangents))
	);
	let missing = arc_proposal(
		Vec2::new(2.0, 0.0),
		Vec2::new(6.0, 0.0),
		ArcDefinition::TwoTangents {
			start_direction: Vec2::X,
			end_direction: None,
		},
	);
	assert_eq!(
		propose_arc(&committed, &missing).err(),
		Some(EditError::Arc(wall_graph::ArcError::MissingConnection))
	);
}

#[test]
fn a_typed_radius_keeps_the_side_and_size_of_the_arc() {
	let proposal = arc_proposal(
		Vec2::ZERO,
		Vec2::new(4.0, 0.0),
		ArcDefinition::Bend(Vec2::new(2.0, 1.0)),
	);
	let current = proposal.sweep().unwrap();
	let retyped = proposal.sweep_for_radius(2.5).unwrap();
	assert_eq!(current.signum(), retyped.signum());
	assert!(retyped.abs() < std::f32::consts::PI);
	assert!(proposal.sweep_for_radius(1.0).is_err());
}

#[test]
fn moving_a_junction_keeps_openings_and_categories_and_reports_the_rebuilt_walls() {
	let committed = candidate_with_wall(Vec2::ZERO, Vec2::new(6.0, 0.0));
	let wall = committed.graph().walls().next().unwrap();
	let committed = propose_opening(
		&committed,
		&opening(wall, 3.0, 0.9, OpeningCategory::Window),
	)
	.unwrap();
	let opening_id = committed.graph().openings().next().unwrap().0;
	let end = wall.destination();

	let candidate = propose_move(
		&committed,
		&MoveProposal {
			first: (end, Vec2::new(8.0, 0.0)),
			second: None,
			grabbed: Some(wall),
		},
	)
	.unwrap();

	assert!(candidate.graph().opening(opening_id).is_some());
	assert_eq!(
		candidate.categories().get(opening_id),
		Some(OpeningCategory::Window)
	);
	assert_eq!(candidate.report().rebuilt_walls.len(), 1);
	assert_eq!(
		candidate.mesh(),
		&wall_mesh::generate(candidate.graph(), &MeshSettings::default()).unwrap()
	);
}

#[test]
fn a_rejected_move_is_an_error_and_the_committed_candidate_is_unchanged() {
	let committed = propose_wall(
		&candidate_with_wall(Vec2::ZERO, Vec2::new(4.0, 0.0)),
		&WallProposal {
			start: Vec2::new(0.0, 2.0),
			end: Vec2::new(4.0, 2.0),
			dimensions: WallDimensions::default(),
		},
	)
	.unwrap();
	let node = committed
		.graph()
		.nodes()
		.find(|(_, position)| *position == Vec2::new(4.0, 2.0))
		.unwrap()
		.0;
	let result = propose_move(
		&committed,
		&MoveProposal {
			first: (node, Vec2::new(4.0, -2.0)),
			second: None,
			grabbed: None,
		},
	);
	assert_eq!(
		result.err(),
		Some(EditError::Graph(wall_graph::WallError::CrossesWall))
	);
	assert_eq!(
		committed.graph().node_position(node),
		Some(Vec2::new(4.0, 2.0))
	);
}

#[test]
fn bending_a_wall_changes_its_curve_and_straightening_restores_it() {
	let committed = candidate_with_wall(Vec2::ZERO, Vec2::new(4.0, 0.0));
	let wall = committed.graph().walls().next().unwrap();
	let bent = propose_bend(&committed, &BendProposal { wall, sweep: 1.0 }).unwrap();
	let bent_wall = bent.report().rebuilt_walls[0].1;
	assert!(matches!(
		bent.graph().wall_curve(bent_wall),
		Some(wall_graph::WallCurve::CircularArc { .. })
	));
	let straight = propose_bend(
		&bent,
		&BendProposal {
			wall: bent_wall,
			sweep: 0.0,
		},
	)
	.unwrap();
	let straight_wall = straight.graph().walls().next().unwrap();
	assert!(matches!(
		straight.graph().wall_curve(straight_wall),
		Some(wall_graph::WallCurve::Straight { .. })
	));
}
