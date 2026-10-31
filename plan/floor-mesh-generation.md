# Floor mesh generation

Status: proposed. Order: 2. Depends on
[room regions](room-detection.md) and the [document foundations](README.md).

## Intended behavior

Generate physical floor geometry from the building's supported regions. Follow
straight and curved walls, support concave boundaries and holes, and provide
separate finish regions for rooms sharing a continuous floor.

## Current support and gaps

The editor's `scene.rs` spawns a fixed 4-by-4 rectangle; it is not part of the
document or generated from walls. `wall_mesh` already resolves wall offsets and
junctions, but keeps that footprint machinery private. Its ear-clipping
triangulator handles simple polygons, not arbitrary polygons with holes.

## Proposed architecture

Separate three concepts:

- Floor coverage: where physical support exists on a level.
- Structural slab: thickness, underside, and exposed edges.
- Floor finishes: visible regions/materials, which may follow room divisions.

Logical room boundaries should not duplicate the slab or introduce cracks.
Courtyards, stair openings, and atriums are explicit coverage exclusions, not
accidental gaps between room meshes.

Keep floor definitions in `house_document` and generation engine-independent.
Initially use a focused module; decide whether a separate geometry crate is
useful once the interface is understood. Share the wall mesher's resolved
junction/offset information through a narrow API rather than independently
approximating the same corners.

Visible finish boundaries generally meet wall faces. Slabs may extend beneath
walls. Preserve that distinction instead of applying one inward offset to every
kind of floor output.

## Implementation steps

1. Establish floor coverage semantics for a simple enclosed, single-level room.
2. Generate a flat top mesh and tag it with the relevant level/region identity.
3. Resolve boundaries against wall thickness and actual junction geometry,
   including unequal thicknesses, miters, and bevels.
4. Add curved sampling with a controlled error budget, concave polygons, and
   triangulation supporting holes. Keep shared boundaries consistent between
   neighbouring regions and wall/floor output.
5. Add slab thickness, underside, and exposed boundary faces with consistent
   winding and normals.
6. Add authored void records and validate their relationship to coverage.
7. Add material/UV support, picking, previews, history, and persistence. Regenerate
   meshes from definitions when loading.

Stair-owned voids will be introduced by the
[stairs pipeline](multiple-floors-and-stairs.md), using this same coverage model.

## Systems touched

| System | Responsibility |
| --- | --- |
| Room boundary layer | Supported region and finish-region inputs |
| `wall_mesh` | Reusable resolved footprint information |
| `house_document` | Coverage, slab/finish settings, voids, validation, persistence |
| Geometry generation | Triangulation, slab surfaces, UVs, semantic tags |
| Core/editor | Floor picking, display, properties, coherent preview |

## Scope and edge cases

Start with horizontal slabs and an explicit thickness. Sloped floors, terrain,
and structural analysis are later extensions. Do not infer every possible floor
solely from room closure: terraces or an open exterior edge may eventually need
explicit coverage boundaries.

Define whether invalid or collapsed regions reject the proposed operation or
require an explicit coverage adjustment. Do not silently generate partial floors.

## Validation and completion criteria

- A rectangle, concave region, curved boundary, and polygon with a hole generate
  correct visible surfaces and expected area.
- No triangles span a void; no inverted/degenerate triangles are emitted.
- Mixed wall thickness and bevelled corners leave no visible gaps.
- Two open-plan rooms share continuous physical support while retaining distinct
  finishes.
- A solid slab has valid top/bottom/edge connectivity where closure is intended.
- Floor changes participate in atomic previews, undo/redo, and project round trips.
- Inspect representative headless screenshots with captures outside the repo.
