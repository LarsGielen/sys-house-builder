# Wall graph

`wall_graph` is an engine-independent, two-dimensional graph of straight and
circular wall paths. Adding a path creates or reuses wall junctions and splits
both the new path and any walls it crosses. The graph stores connectivity and
centerlines and wall dimensions; rendering belongs to its consumers.

```rust
use glam::Vec2;
use wall_graph::WallGraph;

let mut graph = WallGraph::new();
let arc = graph.add_arc(
    Vec2::new(1.0, 0.0),
    Vec2::new(-1.0, 0.0),
    std::f32::consts::PI,
)?;
assert_eq!(arc.len(), 1);

// The line meets the upper semicircle twice, so both paths become three pieces.
let line = graph.add_wall(Vec2::new(-2.0, 0.5), Vec2::new(2.0, 0.5))?;
assert_eq!(line.len(), 3);
assert_eq!(graph.walls().count(), 6);

let saved_wall = line[0];
let saved_node = saved_wall.origin();
let ids = graph.optimize();
assert!(graph.wall_length(ids.wall(saved_wall).unwrap()).is_some());
assert_eq!(graph.node_position(ids.node(saved_node).unwrap()), Some(Vec2::new(-2.0, 0.5)));
# Ok::<(), wall_graph::WallError>(())
```

`add_wall` and `add_arc` take endpoint positions and return the current walls
covering the new path in path order. Compatible straight pieces and circular arcs
merge through junctions with exactly two incident walls when their thickness and
height match. Default dimensions are 0.20 m thick and 2.50 m high.
`add_wall_with_dimensions` and `add_arc_with_dimensions` accept explicit dimensions;
`wall_dimensions` and `set_wall_dimensions` query and edit a live piece.
For consumers that build surfaces, `node_walls` lists a junction's walls in
counterclockwise departure order and `wall_curve` returns a piece's exact line or
circle in double precision.
A returned wall may therefore extend beyond the requested path. A failed insertion leaves nodes,
walls, and identifiers unchanged. Removing, splitting, or merging a wall
invalidates its handle; node identifiers stay stable through ordinary edits
while their junctions remain connected. The public API does not create
standalone nodes.

An opening belongs to one current wall piece. `add_opening` takes an `OpeningSpec`
with its center distance along the wall path, width, bottom elevation, and opening
height, all in metres. `opening`, `openings`, and `opening_position` query them;
`set_opening` and `remove_opening` edit them independently of the wall. Opening
IDs stay stable through ordinary edits and `optimize`. A wall removal removes
its openings. Splits and merges keep an opening's world position; an insertion
that would cut through or crowd an opening is rejected atomically.

After editing, `graph.optimize()` rebuilds the same geometry and connectivity
with contiguous node and internal half-edge IDs. It returns a `WallGraphIdMap`
that translates old node IDs and wall handles for objects that refer to the
graph. Apply that mapping to external references before using them again. If an
insertion runs out of IDs, it returns `WallError::IdExhausted` without changing
the graph.

The guides cover different questions:

- [Architecture](https://github.com/LarsGielen/sys-house-builder/blob/main/crates/wall_graph/docs/architecture.md): how nodes, half-edges, geometry, insertion, and removal fit together.
- [Geometry conventions](https://github.com/LarsGielen/sys-house-builder/blob/main/crates/wall_graph/docs/geometry.md): sweep direction, parameters, tolerances, errors, and limits.
- API reference: generate it with `cargo doc -p wall_graph --no-deps`.

The example above is also a rustdoc test. Run `cargo test -p wall_graph` to check
the crate's examples and behavior.
