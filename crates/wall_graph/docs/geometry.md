# Geometry conventions

All positions use the XY plane in arbitrary coordinate units. The graph stores
wall centerlines and their junctions, not thickness, height, or surfaces.
Public positions and samples use `glam::Vec2` (`f32` coordinates). Curve
calculations use `f64` internally, but that does not restore precision already
lost when positions were supplied as `Vec2`.

## Curves and direction

`add_wall(origin, destination)` creates a straight segment.
`add_arc(origin, destination, signed_sweep)` creates a circular arc
through those positions. Positive sweep is counterclockwise and negative
sweep is clockwise, viewed from above the XY plane. From `(1, 0)` to
`(-1, 0)`, `+PI` takes the upper semicircle and `-PI` the lower one.
Magnitudes above `PI` select a major arc.

An input arc sweep must be finite, have magnitude greater than `0.000001`
radians, and remain below one full revolution. Its endpoints must be farther
apart than the distance tolerance. Full circles require at least two wall
pieces. A valid arc must also remain representable as `Vec2` along its whole
path, even when its endpoints are finite.

Every directed wall uses a parameter `t` in `[0, 1]`:

| Parameter | Position |
| --- | --- |
| `0` | Origin of the `Wall` handle |
| `1` | Destination of the `Wall` handle |

`wall_position(wall, t)` evaluates the path; `wall_tangent(wall, t)`
returns its unit direction. `wall_length(wall)` returns the actual path
length as `f64`. For the current segments and circular arcs, equal
parameter intervals happen to have equal lengths. Callers should still
treat `t` as a path-order parameter: that relationship need not hold for
future Bézier curves.

Splitting an arc across `[a, b]` keeps it circular with sweep
`original_sweep × (b - a)`. The resulting piece uses its junction nodes as
endpoints. These pieces can have smaller sweeps than the minimum permitted for
a newly requested arc.

## Contacts, snapping, and rejection

The geometry module handles segment–segment, segment–arc, and arc–arc
intersections. It filters supporting-line or supporting-circle solutions to
the finite paths. An intersection result can contain multiple contacts,
each with a parameter on both curves, or report an overlapping stretch.
Curves that share an endpoint are still checked for another meeting elsewhere.

| Rule | Current value | Effect |
| --- | ---: | --- |
| Distance tolerance | `0.0001` coordinate units | Groups nearby contacts, finds points on curves, reuses nearby endpoint nodes, and rejects too-short pieces. |
| Departure-angle tolerance | `0.000001` radians | Rejects junctions whose outgoing paths cannot be ordered unambiguously by tangent. |

When a requested endpoint lies within distance tolerance of existing nodes,
the graph chooses the nearest, then the lowest ID on a tie. It retains that
node's position; it does not move the node to the requested coordinate.
Interior contacts similarly prefer an existing node. The planner checks that
each snapped piece stays within the distance tolerance of its original curve
over its **whole** parameter interval. Otherwise insertion returns
`InconsistentJunction` without changing the graph.

Exactly duplicated paths return `Duplicate`; shared stretches return
`Overlapping`. Tangential contacts and departures with indistinguishable
tangents return `TangentialContact`. A smooth end-to-end continuation can be
valid when its two *outgoing* directions point opposite ways. Near-circle
tangencies within distance tolerance are treated as tangential instead of
creating two nearly coincident junctions.

Other errors include `InvalidPosition` for non-finite endpoints,
`InvalidArc` for an invalid sweep or unrepresentable arc, and `ZeroLength`
when requested endpoints or a resulting piece are too close. `UnknownWall`
means a handle has been removed or split; `InvalidParameter` covers invalid
sampling deviation or excessive sample count. `UnknownNode` and `SameNode`
remain in the error type for the internal node-targeted insertion path but
are not produced by the public position-based insertion methods.

## Sampling and limits

`sample_wall(wall, max_deviation)` returns ordered points including both
endpoints. Straight walls need one chord; arcs use enough chords to keep the
maximum geometric deviation within the requested positive, finite distance,
before conversion back to `Vec2`. Requests needing more than 65,536 segments
return `InvalidParameter`. Sampled points belong to a renderer or exporter;
they never become graph nodes.

The graph currently rejects overlapping paths and tangential junctions. It
does not support a single full-circle wall, self-intersecting curve types,
or Bézier walls. It does not compute wall thickness, offset curves, or room
areas. See [Architecture](architecture.md) for how these geometry decisions
feed the topology.
