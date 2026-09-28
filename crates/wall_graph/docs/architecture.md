# Architecture

`WallGraph` connects wall junctions with straight segments and circular arcs.
It uses a half-edge structure for connectivity and a separate geometry module
for paths and intersections. The graph has no dependency on Bevy or another
engine; the demo renders sampled paths outside the core crate.

Use these terms consistently:

| Term | Meaning |
| --- | --- |
| Node or junction | A position shared by incident wall pieces. Public insertion leaves no standalone nodes. |
| Wall | One current piece between two nodes, with one geometric shape. |
| Wall path | The shape requested by one insertion; crossings may turn it into several walls. The graph does not retain a separate identity for the original path. |
| Half-edge | One direction along one side of a wall. A wall has two twins. |
| Face boundary | A cycle obtained by following `next` links. The graph does not store room or face objects. |

## A wall and its two sides

Every wall has two half-edges, one in each direction:

```text
                 forward
             A ---------> B
             A <--------- B
                 backward
```

Each half-edge stores its origin, twin, next, and previous IDs. Its destination
is the origin of its twin, so it is not stored twice. `next` and `previous`
follow the boundary on the half-edge's left. When a traversal reaches a node,
`next` selects the first departure clockwise from the arriving edge's twin.
Conversely, from an outgoing edge `e`, `next(twin(e))` gives the next outgoing
edge clockwise around its origin.

A node stores its `Vec2` position and one outgoing half-edge as an entry into
that ring. The geometry of a wall is stored once for the pair. Its endpoints
come from the nodes, and the backward half-edge uses the reversed shape.
Departure **tangents**, rather than endpoint chords, determine ring order; an
arc can leave a node in a direction quite different from the line to its other
endpoint. Boundary traversal is available through the links, but the crate
does not yet expose a room-query API.

## Worked example: one line crossing an arc twice

First insert a counterclockwise semicircle from `S = (1, 0)` to
`E = (-1, 0)`. Then insert a horizontal line from `L = (-2, 0.5)` to
`R = (2, 0.5)`.

```text
                       upper arc
                       .------.
                    .-'        '-.
  L *-------- X *----------------* Y --------* R     y = 0.5
                 /                \
              E *                  * S                y = 0

              arc direction:  S -> Y -> X -> E
             line direction:  L -> X -> Y -> R
```

`X` and `Y` are the two intersections, at `x = -sqrt(3)/2` and
`x = sqrt(3)/2`. The diagram is schematic: the arc stays curved between
the crossings; only the new line follows the horizontal segment.

Before the second insertion there is one wall and two nodes. Afterwards:

| Original path | Current pieces in its direction |
| --- | --- |
| Semicircle | `S -> Y`, `Y -> X`, `X -> E` (three arcs) |
| New line | `L -> X`, `X -> Y`, `Y -> R` (three segments) |

There are six walls and six nodes. The returned vector contains only the three
new **line** pieces, in `L -> R` order. The old arc handle is invalid because
its wall was replaced. Its three replacement handles can be found through
`walls()`; the API does not return them directly. `X` and `Y` are shared
by both paths, allowing boundary walks through the lens-shaped region between
them.

## How insertion works

1. The public method validates its positions and shape. It creates a temporary
   copy of the graph, then finds or inserts endpoint nodes there. An existing
   node within the distance tolerance is reused. The proposed path is checked
   against any change caused by snapping to those node positions.
2. The planner scans existing nodes on the new path and intersects the new
   curve with every existing wall. Contacts carry parameters on **both**
   curves, so one pair of curves may produce multiple junctions. Duplicate and
   overlapping paths, as well as unsupported tangential contacts, are rejected.
3. Contacts near one another are grouped into junctions, preferring an
   existing node when possible. The planner orders cuts on the new path and on
   each affected old wall, then constructs all replacement pieces. Each old
   wall is replaced once, even if it has several cuts.
4. Before changing topology, the planner checks each piece's length and
   geometric deviation after snapping, and checks departure tangents against
   surviving and planned walls.
5. On success, the temporary graph gains junction nodes, old walls are
   detached, and all replacement and new pieces are linked into their node
   rings. Adjacent pieces of the same shape then merge through degree-two
   nodes when one path represents them within tolerance. Only then does it
   replace the caller's graph. On error, the caller's graph and identifier
   counters remain unchanged.

For example, adding `A -> B` and then `B -> C` on the same line leaves one
`A -> C` wall. Bridging two collinear walls also leaves one wall. The returned
handle covers the requested path but may now reach beyond it; handles for the
absorbed pieces become invalid. A branch or crossing preserves its junction,
so walls do not merge through it. Circular arcs merge only when their direction
and curvature agree and their combined sweep remains below one revolution.

The temporary copy makes endpoint creation part of the same atomic operation.
It adds work and memory proportional to graph size for each insertion. The
planner also scans the graph; it has no spatial index at present.

## Removal, handles, and invariants

`remove_wall` validates the handle, detaches both half-edges, and reconnects
their neighbors. It deletes either endpoint only if no incident wall remains.
It does not merge neighboring wall pieces automatically.

Node IDs and half-edge IDs are not reused during normal editing. A `Wall` is a
copyable handle to one *current* piece, not a persistent identity for an
original drawing operation. Its cached endpoint IDs remain readable after
removal, splitting, or merging, but geometry queries then return `None` and removal
returns `UnknownWall`. IDs and handles are local to one graph instance.

`optimize()` rebuilds the graph in old-ID order, assigning contiguous node IDs
and half-edge IDs from zero. It preserves each surviving wall's shape,
direction, endpoints, and topology. It returns `WallGraphIdMap`, with `node(old)`
and `wall(old)` lookups for external references. A lookup returns `None` for an
ID or handle absent at optimization time. Callers must replace stored IDs and
handles after optimization; old numeric values may now identify different
objects. The operation scans and rebuilds the whole graph, so it suits a load,
save, or explicit maintenance step rather than every edit.

In debug builds, mutations check link reciprocity, endpoints, geometry,
minimum piece length, and departure ordering. See
[Geometry conventions](geometry.md) for the numeric rules behind those checks.

To support another curve type, extend the private `CurveShape` operations
(evaluation, tangent, reversal, subcurve extraction, length, point location,
deviation, and sampling) and its pairwise intersection cases. The insertion
planner consumes curve parameters and subcurves; half-edge linkage does not
need a curve-specific path representation. Renderers can sample a curve but
must not insert those display points as graph nodes.
