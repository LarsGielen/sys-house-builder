use super::*;

fn spec(center_distance: f64) -> OpeningSpec {
	OpeningSpec {
		center_distance,
		width: 1.0,
		bottom: 0.5,
		height: 2.0,
	}
}

fn graph_with_wall() -> (WallGraph, Wall) {
	let mut graph = WallGraph::new();
	let wall = graph.add_wall(Vec2::ZERO, Vec2::new(4.0, 0.0)).unwrap()[0];
	(graph, wall)
}

#[test]
fn area_spans_the_opening_along_the_wall_and_up() {
	let (graph, wall) = graph_with_wall();

	let area = opening_area(&graph, wall, &spec(2.0)).unwrap();

	assert_eq!(
		area.corners(),
		[
			Vec3::new(1.5, 0.0, 0.5),
			Vec3::new(2.5, 0.0, 0.5),
			Vec3::new(2.5, 0.0, 2.5),
			Vec3::new(1.5, 0.0, 2.5),
		]
	);
	assert_eq!(area.center(), Vec3::new(2.0, 0.0, 1.5));
	assert_eq!(area.tangent, Vec2::X);
}

#[test]
fn edges_beyond_the_wall_are_clamped_onto_it() {
	let (graph, wall) = graph_with_wall();

	let area = opening_area(&graph, wall, &spec(0.2)).unwrap();

	assert_eq!(area.start, Vec2::ZERO);
}

#[test]
fn a_stale_wall_has_no_area() {
	let (mut graph, wall) = graph_with_wall();
	graph.remove_wall(wall).unwrap();

	assert_eq!(opening_area(&graph, wall, &spec(2.0)), None);
}
