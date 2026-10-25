use super::*;
use glam::Vec2;
use wall_graph::OpeningSpec;

#[test]
fn categories_of_removed_openings_are_dropped() {
	let mut graph = WallGraph::new();
	let wall = graph.add_wall(Vec2::ZERO, Vec2::new(6.0, 0.0)).unwrap()[0];
	let spec = |center_distance| OpeningSpec {
		center_distance,
		width: 0.9,
		bottom: 0.0,
		height: 2.1,
	};
	let kept = graph.add_opening(wall, spec(1.5)).unwrap();
	let removed = graph.add_opening(wall, spec(4.0)).unwrap();
	let mut categories = OpeningCategories::default();
	categories.insert(kept, OpeningCategory::Window);
	categories.insert(removed, OpeningCategory::Door);

	graph.remove_opening(removed).unwrap();
	categories.retain_existing(&graph);

	assert_eq!(categories.get(kept), Some(OpeningCategory::Window));
	assert_eq!(categories.get(removed), None);
	let mut expected = OpeningCategories::default();
	expected.insert(kept, OpeningCategory::Window);
	assert_eq!(categories, expected);
}
