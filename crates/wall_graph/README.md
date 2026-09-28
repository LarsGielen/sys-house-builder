# Wall graph

An engine-independent planar half-edge graph with straight segments and circular
arcs. Adding a wall splits both the new path and existing walls at their contacts.
Returned handles follow the new path in insertion order.

```rust
use glam::Vec2;
use wall_graph::WallGraph;

let mut graph = WallGraph::new();
let pieces = graph.add_arc(
    Vec2::new(1.0, 0.0),
    Vec2::new(-1.0, 0.0),
    std::f32::consts::PI,
)?;
let points = graph.sample_wall(pieces[0], 0.01)?;
# Ok::<(), wall_graph::WallError>(())
```

## Geometry conventions

- `add_wall` creates a straight segment. `add_arc` uses a signed sweep in radians:
  positive counterclockwise, negative clockwise, viewed in the XY plane.
- Arc sweep magnitude must exceed `0.000001` and be less than one revolution.
  Major arcs are supported. A full circle can be built from two semicircles.
- Wall insertion takes positions and creates or reuses endpoint nodes atomically.
  Existing nodes within `0.0001` coordinate units are reused, preferring the
  closest node and then the oldest identifier. Every stored node belongs to a wall.
- Node positions own the endpoints; each wall stores its shape once, shared by
  its two half-edges. Different geometric paths may connect the same nodes.
- `wall_position` and `wall_tangent` use parameters from zero at the handle's
  origin to one at its destination. Parameters describe path order, not a general
  distance fraction. `wall_length` returns the path length as `f64`.
- Geometry calculations use `f64` internally; node and sampled positions remain
  `Vec2`. Arcs extending outside finite `Vec2` coordinates are rejected.
- Contact and snapping decisions use `0.0001` coordinate units. Snapping preserves
  each piece within that distance over its whole parameter interval; otherwise
  insertion returns `InconsistentJunction`.
- Overlapping paths are rejected. Tangential junctions and contacts within the
  distance tolerance of circle tangency are rejected with `TangentialContact`.
  Node ordering uses departure tangents with a `0.000001` radian tolerance.
- Interior contacts prefer an existing node deterministically. Node IDs remain
  stable while the node has an incident wall.

Insertion validates a temporary graph and commits it only on success, so errors
change neither nodes, walls, nor identifier counters. Removal and splitting
invalidate the original wall handle. Queries on stale handles return `None` or
`UnknownWall`, depending on the query.

## Rendering and future curves

`sample_wall` produces points with a requested maximum chord deviation, before
coordinate rounding. It introduces no graph nodes and limits requests to 65,536
segments. The Bevy demo renders these samples; the core has no Bevy dependency.

The private `geometry` module contains curve evaluation, tangents, point location,
length, reversal, subcurve shape extraction, deviation bounds, and curve-pair
intersections. Insertion planning consumes parameters and subcurves; half-edge
linkage does not depend on curve-specific intersection algorithms.

Adding a Bézier variant will require implementing those geometry operations and
its curve-pair intersections. Shape extraction must also subdivide its control
points. Contacts already retain parameters on both curves and tangency information.
Closed or self-intersecting single curves would need additional graph policies.
