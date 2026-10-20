use super::*;

#[test]
fn failed_proposal_leaves_committed_graph_unchanged() {
	let mut graph = WallGraph::new();
	graph.add_wall(Vec2::ZERO, Vec2::X).unwrap();
	let nodes: Vec<_> = graph.nodes().collect();
	let walls: Vec<_> = graph.walls().collect();
	let proposal = WallProposal {
		start: Vec2::ZERO,
		end: Vec2::X,
		dimensions: WallDimensions::default(),
	};
	assert!(propose_wall(&graph, &proposal).is_err());
	assert_eq!(graph.nodes().collect::<Vec<_>>(), nodes);
	assert_eq!(graph.walls().collect::<Vec<_>>(), walls);
}

#[test]
fn successful_proposal_preserves_committed_graph_and_generates_matching_mesh() {
	let graph = WallGraph::new();
	let proposal = WallProposal {
		start: Vec2::ZERO,
		end: Vec2::X,
		dimensions: WallDimensions::default(),
	};
	let candidate = propose_wall(&graph, &proposal).unwrap();
	assert_eq!(graph.walls().count(), 0);
	assert_eq!(candidate.graph().walls().count(), 1);
	assert_eq!(
		candidate.mesh(),
		&wall_mesh::generate(candidate.graph(), &MeshSettings::default()).unwrap()
	);
}

#[test]
fn edit_errors_expose_their_cause() {
	use std::error::Error;
	let error = EditError::Graph(WallError::InvalidDimensions);
	assert!(error.source().unwrap().is::<WallError>());
}
