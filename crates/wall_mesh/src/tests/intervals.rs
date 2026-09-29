use super::*;

#[test]
fn subtracting_openings_leaves_the_solid_parts() {
	let wall = Intervals::span(0.0, 2.5);
	let window = wall.subtract(0.9, 2.1);
	assert_eq!(window, Intervals(vec![(0.0, 0.9), (2.1, 2.5)]));
	assert_eq!(wall.subtract(0.0, 2.1), Intervals(vec![(2.1, 2.5)]));
	assert_eq!(wall.subtract(3.0, 4.0), wall);
	assert_eq!(wall.difference(&window), Intervals(vec![(0.9, 2.1)]));
	assert_eq!(Intervals::span(1.0, 1.0), Intervals::default());
}

#[test]
fn snapping_merges_nearly_equal_levels_and_drops_slivers() {
	let top = 0.9f32 as f64 + 1.2f32 as f64;
	let wall_top = 2.1f32 as f64;
	let solid = Intervals::span(0.0, wall_top).subtract(0.9f32 as f64, top);
	let levels = Levels::new([0.0, 0.9f32 as f64, top, wall_top].into_iter());
	assert_eq!(levels.snap(top), levels.snap(wall_top));
	assert_eq!(
		solid.snapped(&levels),
		Intervals(vec![(0.0, 0.9f32 as f64)])
	);
}
