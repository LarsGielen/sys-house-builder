# Geometry conventions

All positions use the XY plane in metres. The graph stores wall centerlines,
their junctions, thickness, and height, but not surfaces.
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

After insertion, two pieces can merge through a node with exactly two incident
walls if their directed tangents align and one shape represents both within the
distance tolerance. Straight pieces become one segment; circular pieces must
turn the same way and combine to a sweep below one revolution. Different curve
types, bends, branch junctions, and full circles stay separate. A successful
merge removes its middle node and invalidates both old wall handles.
The pieces must also have equal thickness and height. Splitting inherits the
original piece's dimensions.

## Openings and clearances

An opening's center and width are measured in metres along its wall centerline.
Its bottom and height are measured above the wall's floor. Openings on the same
wall may be vertically stacked; their path and vertical intervals must have at
least 0.02 m separation in one direction. Width must exceed the graph's distance
tolerance. Each opening must fit within its wall's path length and height.

At a junction, every connected wall is treated as a constant-width swept
centerline, with radius equal to half its thickness. An opening must remain at
least 0.05 m beyond that footprint. The clearance check covers the complete
opening interval on straight and curved walls. Wall insertion and dimension
edits validate the resulting openings before committing.

## Contacts, snapping, and rejection

The geometry module handles segment–segment, segment–arc, and arc–arc
intersections. It filters supporting-line or supporting-circle solutions to
the finite paths. An intersection result can contain multiple contacts,
each with a parameter on both curves, or report an overlapping stretch.
Curves that share an endpoint are still checked for another meeting elsewhere.

| Rule | Current value | Effect |
| --- | ---: | --- |
| Distance tolerance | `0.0001` m | Groups nearby contacts, finds points on curves, reuses nearby endpoint nodes, and rejects too-short pieces. |
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
`InvalidDimensions` for non-finite or nonpositive wall thickness or height,
`InvalidArc` for an invalid sweep or unrepresentable arc, and `ZeroLength`
when requested endpoints or a resulting piece are too close. `UnknownWall`
means a handle has been removed, split, or merged; `InvalidParameter` covers
invalid sampling deviation or excessive sample count. `IdExhausted` means
insertion needs more node or half-edge IDs than the counters can represent;
the graph is unchanged. `UnknownNode` and `SameNode`
remain in the error type for the internal node-targeted insertion path but
are not produced by the public position-based insertion methods.

## Arc solvers

`arc_through_bend`, `arc_tangent_at_start`, `arc_tangent_at_both`, and
`arc_sweep_for_radius` turn drawing constraints into the signed sweep that
`add_arc` takes. An arc leaves its start `sweep / 2` clockwise of the chord and
arrives `sweep / 2` counterclockwise of it, so a tangent constraint is an angle
comparison: one tangent gives `sweep = 2 * angle(direction, chord)`, and two
tangents must agree on that angle or the request is `ArcError::InconsistentTangents`.
A returned sweep of exactly `0.0` means the constraints describe a straight wall.
A start tangent whose end lies directly behind it would need a full revolution and
is rejected. `arc_points` samples a proposal that is not in the graph yet.

## Smooth joins

`smooth_joins` reports every pair of walls whose departures at a node are opposite
within `1e-4` (sine of the angle). It is computed from geometry on each call; the graph
stores no tangent flags, so a join cannot go stale.

## Moving junctions and bending walls

`move_nodes` and `bend_wall` change geometry in place and rebuild only the walls whose
endpoints moved (or the bent wall) as one atomic operation: nodes, opening IDs, and
the handles of every other wall are kept, and a failed call changes nothing. A node is
shared topology, so moving it stretches every wall that ends there. A whole-wall drag
moves its two nodes by the same offset; an endpoint or junction drag moves one.

For rebuilt walls:

| Wall | Result |
| --- | --- |
| Straight | Stays straight, keeping its other endpoint. |
| Arc with a decided neighbour direction at an end | Re-solved to leave that end along the join's tangent. The neighbour is decided when its node did not move, or when it is a straight wall (its new direction is known). With two such ends the arc must satisfy both; when they disagree it keeps the end at an unmoved node. |
| Arc with no decided neighbour, or no solution | Keeps its sweep, the smallest change to the original curve. |

The `ReshapeReport` lists old to new wall handles and the nodes where a smooth join
existed before and does not now, for display before the change is committed.
Openings stay on their wall at the same fraction of its length, so stretching a wall
stretches the gaps around them; the move is rejected if an opening then no longer fits.
A move is also rejected (`CrossesWall`, `Overlapping`, or `TangentialContact`) if a
rebuilt wall would cross, touch, or overlap another wall, or land on another node,
because those layouts need an insertion that splits walls.

## Sampling and limits

`sample_wall(wall, max_deviation)` returns ordered points including both
endpoints. Straight walls need one chord; arcs use enough chords to keep the
maximum geometric deviation within the requested positive, finite distance,
before conversion back to `Vec2`. Requests needing more than 65,536 segments
return `InvalidParameter`. Sampled points belong to a renderer or exporter;
they never become graph nodes.

Consumers that offset or intersect wall geometry should not infer it from
samples. `wall_curve(wall)` returns the exact path in the handle's direction:
a `Straight` segment, or a `CircularArc` with its center, radius, start angle,
and signed sweep in `f64`. `node_walls(node)` lists incident walls in
counterclockwise order of their departure tangents, starting at the smallest
angle in `(-PI, PI]`.

The graph currently rejects overlapping paths and tangential junctions. It
does not support a single full-circle wall, self-intersecting curve types,
or Bézier walls. It stores wall thickness but does not compute offset curves or room
areas. See [Architecture](architecture.md) for how these geometry decisions
feed the topology.
