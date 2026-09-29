# Wall mesh

`wall_mesh` turns a `wall_graph::WallGraph` into a closed, indexed triangle mesh.
It is engine-independent: it returns positions, normals, triangle indices, and
tagged sections, and a renderer or exporter converts those into its own types.

```rust
use glam::Vec2;
use wall_graph::{OpeningSpec, WallGraph};
use wall_mesh::{MeshSettings, SurfaceKind, generate};

let mut graph = WallGraph::new();
let wall = graph.add_wall(Vec2::new(0.0, 0.0), Vec2::new(4.0, 0.0))?[0];
graph.add_wall(Vec2::new(0.0, 0.0), Vec2::new(0.0, 3.0))?;
graph.add_opening(
    wall,
    OpeningSpec { center_distance: 2.0, width: 0.9, bottom: 0.0, height: 2.1 },
)?;

let mesh = generate(&graph, &MeshSettings::default())?;
assert!(mesh.sections().iter().any(|section| section.tag.kind == SurfaceKind::Jamb));
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Output

Coordinates keep the graph's floor-plan X and Y and add elevation as Z, in
metres. Triangles wind counterclockwise when seen from outside. The mesh is a
closed solid: after welding identical positions, every edge borders exactly two
triangles in opposite directions. Faces meeting at a hard edge have separate
vertices; curved wall faces use radial normals so they shade smoothly.

Each `MeshSection` tags a range of indices with a `SurfaceKind` (wall side, top,
bottom, end cap, jamb, head, sill, junction step, or bevel) and the wall,
opening, or junction it belongs to. Tags are meant for later material work; they
refer to the meshed graph state, so regenerate the mesh after editing the graph.

## How walls are joined

The footprint is split into planar cells that share exact vertices. Each wall
contributes body cells between its junction trims and opening edges. Each
junction of two or more walls contributes one core cell:

- At each gap between neighbouring walls, the facing wall sides meet in a miter.
  An outside miter longer than `miter_limit` times the thicker half-thickness
  becomes a bevel. Walls of different thickness that meet nearly straight
  become a step.
- A junction core takes the height of its tallest wall, so a taller wall wraps
  the corner and a shorter wall meets it with a step face.
- A single wall end gets a flat cap.

Every cell is solid over some elevation intervals: a body cell is solid over its
wall height minus any openings spanning it. Vertical faces appear where a cell is
solid and its neighbour is not, and horizontal faces at the ends of each solid
interval. This produces jambs, heads, sills, and exposed steps without buried
internal faces.

## Tolerances and limits

| Value | Meaning |
| --- | --- |
| `max_deviation` (setting, 0.005 m) | Largest gap between a curved face and its chords, measured on the outer face |
| `miter_limit` (setting, 4) | Longest outside miter, relative to the thicker half-thickness |
| `min_pier_width` (setting, 0.05 m) | Least wall length between an opening and a resolved junction |
| `max_triangles` (setting, 2,000,000) | Output size limit |
| 0.0001 m | The graph's distance tolerance, used for its path distances |
| 1e-7 m | Footprint vertices closer than this are merged at a junction |
| 1e-5 m | Elevations closer than this are the same level |

Generation fails as a whole, returning a `MeshError` that names the offending
wall, node, or opening:

- an arc too tight for its thickness;
- junction trims that consume a wall;
- an opening within `min_pier_width` of a resolved junction;
- walls that overlap away from a junction;
- limits that would be exceeded.

The graph's own clearance rules measure opening distances along wall
centerlines. A resolved miter can reach further, so an opening the graph accepts
may still be rejected here.

Run `WALL_MESH_SVG_DIR=/some/absolute/dir cargo test -p wall_mesh` to write an
SVG of each footprint test fixture for visual inspection.

## Benchmarks

Run `cargo bench -p wall_mesh --bench generation` to measure mesh generation with
an optimized build. This is separate from the normal test suite. The benchmark
builds each graph before timing, warms up generation, and reports the median of
five samples. It does not include graph insertion time.

The cases compare spread-out walls, parallel walls with overlapping X ranges,
zigzag junctions, and one arc at several sampling tolerances. Wall counts double
to make changes in scaling visible. Compare results from the same machine and
build profile; elapsed times are measurements, not pass/fail limits. The cases
exercise the complete `generate` call, so they identify workloads worth profiling
but do not attribute time to a particular internal stage.
