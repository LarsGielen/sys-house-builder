//! Development diagnostics: draws a resolved footprint as SVG.
//!
//! Set `WALL_MESH_SVG_DIR` when running tests to write one file per footprint fixture.

use std::fmt::Write;

use glam::DVec2;

use crate::Side;
use crate::footprint::{CellKind, EdgeKind, Footprint, polygon_area};

pub(crate) fn write_if_requested(name: &str, footprint: &Footprint) {
	let Ok(directory) = std::env::var("WALL_MESH_SVG_DIR") else {
		return;
	};
	std::fs::create_dir_all(&directory).expect("create SVG directory");
	std::fs::write(
		std::path::Path::new(&directory).join(format!("{name}.svg")),
		footprint_svg(footprint),
	)
	.expect("write SVG");
}

/// Labels show snapshot indices: `w` for walls, `n` for junction nodes.
fn footprint_svg(footprint: &Footprint) -> String {
	let (mut low, mut high) = (DVec2::splat(f64::INFINITY), DVec2::splat(f64::NEG_INFINITY));
	for &vertex in &footprint.vertices {
		low = low.min(vertex);
		high = high.max(vertex);
	}
	let margin = 0.2;
	low -= DVec2::splat(margin);
	high += DVec2::splat(margin);
	let size = high - low;
	let scale = 800.0 / size.x.max(size.y);
	// SVG's Y axis points down; the floor plan's points up.
	let map = |point: DVec2| DVec2::new(point.x - low.x, high.y - point.y) * scale;

	let mut svg = String::new();
	writeln!(
		svg,
		r#"<svg xmlns="http://www.w3.org/2000/svg" width="{:.0}" height="{:.0}" font-family="monospace" font-size="12">"#,
		size.x * scale,
		size.y * scale
	)
	.unwrap();
	writeln!(svg, r##"<rect width="100%" height="100%" fill="#fff"/>"##).unwrap();
	for cell in &footprint.cells {
		let fill = match cell.kind {
			CellKind::Body { ref openings, .. } if !openings.is_empty() => "#d7e8f7",
			CellKind::Body { .. } => "#e6e6e6",
			CellKind::Core { .. } => "#f7d9b0",
		};
		let points: Vec<String> = cell
			.boundary
			.iter()
			.map(|&vertex| {
				let point = map(footprint.vertices[vertex]);
				format!("{:.2},{:.2}", point.x, point.y)
			})
			.collect();
		writeln!(
			svg,
			r#"<polygon points="{}" fill="{fill}" stroke="none"/>"#,
			points.join(" ")
		)
		.unwrap();
		for (k, (from, to)) in cell.edge_vertices().enumerate() {
			let color = match cell.edges[k] {
				EdgeKind::Side {
					side: Side::Left, ..
				} => "#1f5fbf",
				EdgeKind::Side {
					side: Side::Right, ..
				} => "#c0392b",
				EdgeKind::CrossSection { .. } => "#888",
				EdgeKind::Bevel { .. } => "#1e8449",
			};
			let (a, b) = (map(footprint.vertices[from]), map(footprint.vertices[to]));
			writeln!(
				svg,
				r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{color}" stroke-width="1.5"/>"#,
				a.x, a.y, b.x, b.y
			)
			.unwrap();
		}
		let area = polygon_area(&footprint.vertices, &cell.boundary);
		let centroid = cell
			.boundary
			.iter()
			.map(|&vertex| footprint.vertices[vertex])
			.sum::<DVec2>()
			/ cell.boundary.len() as f64;
		let label = match cell.kind {
			CellKind::Body { wall, .. } => format!("w{wall}"),
			CellKind::Core { node } => format!("n{node}"),
		};
		let at = map(centroid);
		writeln!(
			svg,
			r#"<text x="{:.2}" y="{:.2}" text-anchor="middle"><title>area {area:.6}</title>{label}</text>"#,
			at.x, at.y
		)
		.unwrap();
	}
	for vertex in &footprint.vertices {
		let point = map(*vertex);
		writeln!(
			svg,
			r##"<circle cx="{:.2}" cy="{:.2}" r="1.5" fill="#333"/>"##,
			point.x, point.y
		)
		.unwrap();
	}
	svg.push_str("</svg>\n");
	svg
}
