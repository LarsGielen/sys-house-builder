use super::*;
use crate::candidate::{OpeningProposal, WallProposal, propose_opening, propose_wall};
use wall_graph::{WallError, arc_tangent_at_start};

const SAMPLE: &str = "\
house-editor-project 1
# Units are metres.
wall w1: from 0 0 to 4 0 height 2.5 thickness 0.2
wall w2: from 4 0 to 4 3 sweep 1.5707964 height 3 thickness 0.3

opening o1: wall w1 center 2 width 0.9 bottom 0 height 2.1 category door
opening o2: wall w1 center 3.2 width 0.6 bottom 0.9 height 1.2 category window
";

fn add_wall(
	candidate: &EditCandidate,
	start: Vec2,
	end: Vec2,
	sweep: Option<f32>,
	dimensions: WallDimensions,
) -> EditCandidate {
	match sweep {
		None => propose_wall(
			candidate,
			&WallProposal {
				start,
				end,
				dimensions,
			},
		)
		.unwrap(),
		Some(sweep) => {
			let mut graph = candidate.graph().clone();
			graph
				.add_arc_with_dimensions(start, end, sweep, dimensions)
				.unwrap();
			EditCandidate::build(graph, candidate.categories().clone()).unwrap()
		}
	}
}

fn add_opening(
	candidate: &EditCandidate,
	wall_start: Vec2,
	center_distance: f64,
	category: OpeningCategory,
) -> EditCandidate {
	let graph = candidate.graph();
	let wall = graph
		.walls()
		.find(|wall| graph.node_position(wall.origin()) == Some(wall_start))
		.unwrap();
	propose_opening(
		candidate,
		&OpeningProposal {
			wall,
			spec: OpeningSpec {
				center_distance,
				width: 0.9,
				bottom: 0.0,
				height: 2.1,
			},
			category,
		},
	)
	.unwrap()
}

/// Straight walls, a tangent arc, different dimensions, a crossing, and both categories.
fn full_house() -> EditCandidate {
	let thick = WallDimensions {
		thickness: 0.3,
		height: 3.0,
	};
	let mut house = EditCandidate::default();
	house = add_wall(
		&house,
		Vec2::ZERO,
		Vec2::new(6.0, 0.0),
		None,
		Default::default(),
	);
	house = add_wall(
		&house,
		Vec2::new(0.0, -2.0),
		Vec2::new(0.0, 4.0),
		None,
		thick,
	);
	let end = Vec2::new(9.0, 3.0);
	let sweep = arc_tangent_at_start(Vec2::new(6.0, 0.0), Vec2::X, end).unwrap();
	house = add_wall(&house, Vec2::new(6.0, 0.0), end, Some(sweep), thick);
	house = add_wall(
		&house,
		Vec2::new(0.0, 4.0),
		Vec2::new(-3.0, 6.0),
		Some(-1.2),
		Default::default(),
	);
	house = add_opening(&house, Vec2::new(0.0, 0.0), 2.0, OpeningCategory::Door);
	add_opening(&house, Vec2::new(6.0, 0.0), 2.5, OpeningCategory::Window)
}

#[test]
fn a_sample_file_states_version_units_shapes_sizes_and_categories_in_plain_text() {
	let file = ProjectFile::parse(SAMPLE).unwrap();
	assert_eq!(file.walls.len(), 2);
	assert_eq!(file.walls[1].sweep, Some(1.5707964));
	assert_eq!(file.walls[1].dimensions.thickness, 0.3);
	assert_eq!(file.openings[1].category, OpeningCategory::Window);
	assert_eq!(file.openings[1].spec.width, 0.6);
	assert!(file.to_candidate().is_ok());
}

#[test]
fn a_saved_file_documents_its_units_and_coordinates() {
	let text = ProjectFile::from_candidate(&full_house()).to_text();
	assert!(text.starts_with("house-editor-project 1\n"));
	assert!(text.contains("metres"));
	assert!(text.contains("floor-plan"));
	assert!(text.contains("elevation"));
}

#[test]
fn round_trip_keeps_walls_arcs_dimensions_openings_and_categories() {
	let original = full_house();
	let text = ProjectFile::from_candidate(&original).to_text();
	let reopened = ProjectFile::parse(&text).unwrap().to_candidate().unwrap();

	assert_eq!(
		reopened.graph().walls().count(),
		original.graph().walls().count()
	);
	assert_eq!(reopened.graph().openings().count(), 2);
	// Meaning, not private storage order: describing both documents gives the same file.
	assert_eq!(ProjectFile::from_candidate(&reopened).to_text(), text);
	// Saving again loses nothing, and the mesh is the same house.
	assert_eq!(
		reopened.mesh().indices().len(),
		original.mesh().indices().len()
	);
}

#[test]
fn round_trip_keeps_each_openings_category_on_its_own_opening() {
	let reopened = ProjectFile::parse(&ProjectFile::from_candidate(&full_house()).to_text())
		.unwrap()
		.to_candidate()
		.unwrap();
	let mut categories: Vec<_> = reopened
		.graph()
		.openings()
		.map(|(id, opening)| {
			(
				opening.spec.center_distance,
				reopened.categories().get(id).unwrap(),
			)
		})
		.collect();
	categories.sort_by(|a, b| a.0.total_cmp(&b.0));
	assert_eq!(
		categories.iter().map(|entry| entry.1).collect::<Vec<_>>(),
		[OpeningCategory::Door, OpeningCategory::Window]
	);
}

#[test]
fn tangency_is_inferred_from_the_restored_geometry() {
	let original = full_house();
	assert!(!original.graph().smooth_joins().is_empty());
	let text = ProjectFile::from_candidate(&original).to_text();
	assert!(!text.contains("tangent "), "no tangent flag is written");
	let reopened = ProjectFile::parse(&text).unwrap().to_candidate().unwrap();
	assert_eq!(
		reopened.graph().smooth_joins().len(),
		original.graph().smooth_joins().len()
	);
}

#[test]
fn an_empty_house_round_trips() {
	let text = ProjectFile::from_candidate(&EditCandidate::default()).to_text();
	let reopened = ProjectFile::parse(&text).unwrap().to_candidate().unwrap();
	assert_eq!(reopened.graph().walls().count(), 0);
}

#[test]
fn opening_hosts_follow_walls_whatever_order_and_direction_the_file_lists_them() {
	// The wall is written backwards, so the opening's center is measured from its far end.
	let file = ProjectFile::parse(
		"house-editor-project 1\n\
		 wall a: from 4 0 to 0 0 height 2.5 thickness 0.2\n\
		 opening d: wall a center 1 width 0.9 bottom 0 height 2.1 category door\n",
	)
	.unwrap();
	let candidate = file.to_candidate().unwrap();
	let (id, _) = candidate.graph().openings().next().unwrap();
	let position = candidate.graph().opening_position(id).unwrap();
	assert!(position.distance(Vec2::new(3.0, 0.0)) < 1e-4);
}

#[test]
fn a_malformed_number_names_its_line_and_rejects_the_import() {
	let error = ProjectFile::parse(
		"house-editor-project 1\n\
		 wall a: from 0 0 to 4 0 height 2.5 thickness 0.2\n\
		 wall b: from 4 0 to four 3 height 2.5 thickness 0.2\n",
	)
	.unwrap_err();
	assert!(
		matches!(error, ImportError::Syntax { line: 3, .. }),
		"{error:?}"
	);
	assert!(error.to_string().contains("'four'"));
	assert!(
		ProjectFile::parse(
			"house-editor-project 1\nwall a: from 0 0 to NaN 0 height 2 thickness 1\n"
		)
		.is_err()
	);
}

#[test]
fn syntax_problems_are_reported_where_they_are() {
	for (text, line) in [
		("house-editor-project 1\nwall a from 0 0\n", 2),
		("house-editor-project 1\nroof a: pitched\n", 2),
		(
			"house-editor-project 1\nwall a: from 0 0 to 1 1 height 2 thickness 1 extra\n",
			2,
		),
		(
			"house-editor-project 1\nopening a: wall w center 1 width 1 bottom 0 height 1 category gate\n",
			2,
		),
	] {
		let error = ProjectFile::parse(text).unwrap_err();
		assert!(
			matches!(error, ImportError::Syntax { line: l, .. } if l == line),
			"{error:?}"
		);
	}
}

#[test]
fn a_missing_header_or_unsupported_version_is_rejected() {
	assert_eq!(ProjectFile::parse(""), Err(ImportError::MissingHeader));
	assert_eq!(
		ProjectFile::parse("wall a: from 0 0 to 1 0 height 2 thickness 1\n"),
		Err(ImportError::MissingHeader)
	);
	let error = ProjectFile::parse("house-editor-project 2\n").unwrap_err();
	assert_eq!(
		error,
		ImportError::UnsupportedVersion {
			found: "2".to_string()
		}
	);
	assert!(error.to_string().contains("version 1"));
}

#[test]
fn a_missing_host_or_duplicate_id_rejects_the_whole_file() {
	let missing_host = ProjectFile::parse(
		"house-editor-project 1\n\
		 wall a: from 0 0 to 4 0 height 2.5 thickness 0.2\n\
		 opening d: wall zzz center 2 width 0.9 bottom 0 height 2.1 category door\n",
	)
	.unwrap();
	assert_eq!(
		missing_host.to_candidate().err(),
		Some(ImportError::UnknownWall {
			opening: "d".to_string(),
			wall: "zzz".to_string()
		})
	);
	assert!(matches!(
		ProjectFile::parse(
			"house-editor-project 1\n\
			 wall a: from 0 0 to 4 0 height 2.5 thickness 0.2\n\
			 wall a: from 0 1 to 4 1 height 2.5 thickness 0.2\n"
		),
		Err(ImportError::DuplicateId { line: 3, .. })
	));
}

#[test]
fn data_the_graph_rejects_is_reported_against_its_object() {
	for (text, expected) in [
		(
			"house-editor-project 1\nwall a: from 0 0 to 0 0 height 2.5 thickness 0.2\n",
			"wall a",
		),
		(
			"house-editor-project 1\nwall a: from 0 0 to 4 0 height -1 thickness 0.2\n",
			"wall a",
		),
		(
			"house-editor-project 1\n\
			 wall a: from 0 0 to 4 0 height 2.5 thickness 0.2\n\
			 wall b: from 1 0 to 3 0 height 2.5 thickness 0.2\n",
			"wall b",
		),
		(
			"house-editor-project 1\n\
			 wall a: from 0 0 to 4 0 height 2.5 thickness 0.2\n\
			 opening d: wall a center 9 width 0.9 bottom 0 height 2.1 category door\n",
			"opening d",
		),
	] {
		let error = ProjectFile::parse(text)
			.unwrap()
			.to_candidate()
			.err()
			.unwrap();
		assert!(error.to_string().starts_with(expected), "{error}");
	}
	assert!(matches!(
		ProjectFile::parse(
			"house-editor-project 1\nwall a: from 0 0 to 4 0 height 2.5 thickness 0.2\n\
			 wall b: from 1 0 to 3 0 height 2.5 thickness 0.2\n"
		)
		.unwrap()
		.to_candidate(),
		Err(ImportError::Wall {
			error: WallError::Overlapping,
			..
		})
	));
}
