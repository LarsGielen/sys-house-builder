use std::hint::black_box;
use std::time::{Duration, Instant};

use glam::Vec2;
use wall_graph::WallGraph;
use wall_mesh::{MeshSettings, generate};

const SAMPLE_COUNT: usize = 5;
const TARGET_SAMPLE_TIME: Duration = Duration::from_millis(50);
const MAX_ITERATIONS: usize = 10_000;

fn main() {
	println!("wall_mesh::generate (median of {SAMPLE_COUNT} samples; graph construction excluded)");
	println!("case                         walls    iterations     median / generation");

	for wall_count in [16, 32, 64, 128] {
		measure(
			"spread walls",
			wall_count,
			spread_walls(wall_count),
			MeshSettings::default(),
		);
		measure(
			"parallel walls",
			wall_count,
			parallel_walls(wall_count),
			MeshSettings::default(),
		);
		measure(
			"zigzag junctions",
			wall_count,
			zigzag(wall_count),
			MeshSettings::default(),
		);
	}

	for max_deviation in [0.05, 0.01, 0.002] {
		let settings = MeshSettings {
			max_deviation,
			..MeshSettings::default()
		};
		measure(
			&format!("arc (deviation {max_deviation} m)"),
			1,
			semicircle(),
			settings,
		);
	}
}

fn measure(name: &str, wall_count: usize, graph: WallGraph, settings: MeshSettings) {
	let start = Instant::now();
	black_box(
		generate(black_box(&graph), black_box(&settings)).expect("benchmark graph must mesh"),
	);
	let warmup_time = start.elapsed();
	let iterations = (TARGET_SAMPLE_TIME.as_nanos() / warmup_time.as_nanos().max(1))
		.clamp(1, MAX_ITERATIONS as u128) as usize;

	let mut samples = [Duration::ZERO; SAMPLE_COUNT];
	for sample in &mut samples {
		let start = Instant::now();
		for _ in 0..iterations {
			black_box(
				generate(black_box(&graph), black_box(&settings))
					.expect("benchmark graph must mesh"),
			);
		}
		*sample = start.elapsed() / iterations as u32;
	}
	samples.sort_unstable();
	println!(
		"{name:28} {wall_count:>5} {iterations:>13} {median:>20?}",
		median = samples[SAMPLE_COUNT / 2]
	);
}

fn spread_walls(wall_count: usize) -> WallGraph {
	let mut graph = WallGraph::new();
	for index in 0..wall_count {
		let x = index as f32 * 4.0;
		graph
			.add_wall(Vec2::new(x, 0.0), Vec2::new(x + 2.0, 0.0))
			.unwrap();
	}
	graph
}

fn parallel_walls(wall_count: usize) -> WallGraph {
	let mut graph = WallGraph::new();
	for index in 0..wall_count {
		let y = index as f32;
		graph
			.add_wall(Vec2::new(0.0, y), Vec2::new(10.0, y))
			.unwrap();
	}
	graph
}

fn zigzag(wall_count: usize) -> WallGraph {
	let mut graph = WallGraph::new();
	for index in 0..wall_count {
		let x = index as f32 * 2.0;
		let from = Vec2::new(x, if index % 2 == 0 { 0.0 } else { 2.0 });
		let to = Vec2::new(x + 2.0, if index % 2 == 0 { 2.0 } else { 0.0 });
		graph.add_wall(from, to).unwrap();
	}
	graph
}

fn semicircle() -> WallGraph {
	let mut graph = WallGraph::new();
	graph
		.add_arc(
			Vec2::new(10.0, 0.0),
			Vec2::new(-10.0, 0.0),
			std::f32::consts::PI,
		)
		.unwrap();
	graph
}
