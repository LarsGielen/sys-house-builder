# Character pathfinding

Status: proposed. Order: 7. Single-level work can start earlier; full integration
uses [rooms](room-detection.md), [floors](floor-mesh-generation.md),
[stairs](multiple-floors-and-stairs.md), and
[furniture obstacles](furniture-placement.md).

## Intended behavior

A character instructed to go to a bathroom finds a walkable route to a suitable
location in that room. Paths account for walls, openings, furniture, floor voids,
and eventually inter-level stairs and changing door states.

## Proposed architecture

Separate semantic destination resolution, navigation construction/search, and
runtime movement:

1. Resolve a room ID or semantic request such as bathroom to eligible rooms.
2. Find reachable standing locations inside those rooms for this character.
3. Search the navigation representation and return a route with any traversal
   actions, such as using a stair or interacting with a door.
4. Let a runtime movement system follow the route and request replanning when
   its assumptions change.

A room centroid is not a reliable target: it can lie outside a concave room or
inside furniture. Room identity and current classification come from the room
model; furniture-based tagging supplies semantic classifications in its own
pass. Rooms start unknown, and splitting/merging resets the results to unknown.
Semantic requests must not use stale classifications while awaiting retagging.
Reachable goal regions come from walkable geometry. Multiple bathrooms should resolve using
reachability/path cost rather than only straight-line distance.

Keep navigation engine-independent and consume authored/domain geometry through
an adapter. Do not infer walkability from render triangle tags alone or require
an editor session to construct navigation data.

## Navigation inputs

- Supported floor coverage, excluding voids.
- Physical wall solids and actual traversable opening clearance.
- Furniture obstruction at relevant heights.
- Stair paths/endpoints and traversal capabilities.
- Character radius, height, step/slope limits, and permitted traversal actions.
- Runtime door state and other navigation-relevant dynamic state.

Logical room dividers identify destinations but do not obstruct movement. Door
or window category alone is not a complete passability rule. Custom opening
profiles must eventually supply clearance at the required body heights.

## Implementation steps

1. Define focused route queries/results, character capabilities, and explicit
   failures such as unsupported position, no eligible destination, or unreachable.
2. Build a conservative single-level navigation grid. Keep its implementation
   behind the query boundary so a later navigation mesh can replace it.
3. Account for character clearance during occupancy and edge checks. Thin walls
   must not vanish between samples; diagonal moves must not cut corners.
4. Implement A* and debug overlays for occupied cells, search results, goals,
   and route segments.
5. Add room destination lookup and reachable goal-region search.
6. Add furniture obstacles, then stair connections across levels. Route results
   should preserve connector identity and traversal information.
7. Add safe simplification using clearance-aware line-of-sight checks.
8. Add revision-based invalidation for building/obstacle/door changes and integrate
   a simple character that follows paths and requests replanning.

Grid resolution is an explicit trade-off: conservative occupancy can reject
narrow passages that continuous geometry permits. Document the supported
resolution/agent scale and test it before considering a navigation mesh.

## Runtime boundary

Keep character positions, current routes, and changing door state outside
building edit history. Static building edits and runtime state changes both
invalidate affected navigation assumptions, but need not share one revision.

If a closed door is actionable, the route may include an interaction step; if
the character cannot open it, treat it as blocked. Local avoidance between moving
characters is later work and should not be confused with static route search.

## Systems touched

| System | Responsibility |
| --- | --- |
| Room/document model | Stable destinations, types, geometry references |
| Navigation module/crate | Occupancy, clearance, search, routes, invalidation |
| Domain adapter | Floors, walls/openings, furniture, and stair inputs |
| Runtime integration | Movement, door actions, replanning |
| Editor debug UI | Navigation/goal/route inspection |

## Validation and completion criteria

- Reach a bathroom around walls and furniture, including a concave goal room.
- Distinguish unreachable, absent, and reachable destinations.
- Choose a reachable bathroom when a closer one is blocked.
- Respect character width/height, thin walls, voids, and diagonal corners.
- Logical dividers never block a path.
- Cross levels through valid stairs, with appropriate connector traversal data.
- Door/furniture changes invalidate routes; smoothing cannot introduce collisions.
- Core queries work without Bevy and have deterministic test fixtures.
