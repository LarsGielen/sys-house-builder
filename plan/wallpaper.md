# Wallpaper and wall finishes

Status: proposed. Order: 4. Depends on surface ownership and edit correspondence;
[rooms](room-detection.md) enable applying a finish around a complete room.

## Intended behavior

Apply a finish to one side of a wall or all inward-facing sides of a room. Support
textured wallpaper with meaningful scale/offset, including curved walls, while
preserving assignments through splits, merges, and reshaping.

## Current support and gaps

`wall_mesh::SurfaceTag` distinguishes wall sides, opening reveals, junctions,
and other surface roles. `WallMesh` has positions, normals, indices, and sections
but no UVs. `wall_render.rs` currently renders the full committed wall mesh with
one material. Junction-owned surfaces do not always identify the wall side whose
finish should continue across them.

## Proposed architecture

- Store engine-independent finish definitions/references and assignments in the
  document. Bevy resolves references to textures and material assets.
- Assign finishes to directed wall-side spans, not mesh section indices.
- Transfer spans using old/new curve intervals and orientation correspondence.
  Preserve distinct finishes when otherwise compatible walls merge.
- Extend semantic surface ownership where necessary to resolve corner finishes.
- Separate geometric surface coordinates from per-finish texture transforms.

Left/right is relative to a wall's direction. Reversal must swap sides and
transform span/texture coordinates consistently. Room-wide application resolves
the relevant inward-facing side for every physical boundary portion; logical
dividers have no wall surface to paint.

## Implementation steps

1. Add two-sided solid-colour assignments and persistence.
2. Extend picking to return the selected surface side/role, rather than only a
   wall or opening. Give junction surfaces an explicit resolution policy.
3. Render sections grouped by resolved material, keeping useful batching.
4. Generate UV coordinates: horizontal distance along the wall and vertical
   elevation, with an explicit mapping for reveals and caps.
5. Add texture references, repeat scale in metres, offset, and rotation.
6. Preserve texture origin/phase through splitting, reversal, and merging. Do
   not restart a wallpaper pattern merely because a new wall handle was created.
7. Add room-wide application, preview/history, and missing-asset feedback.

Keep appearance edits separate from topology edits so changing wallpaper does
not need to rerun intersection planning. Begin with a simple coherent rebuild;
optimize material-only updates once the ownership model is correct.

## Systems touched

| System | Responsibility |
| --- | --- |
| `house_document` | Finish definitions, assignments, edit transfer, persistence |
| `wall_graph` | Geometry correspondence across wall replacement |
| `wall_mesh` | UVs and sufficiently precise surface ownership |
| `house_editor_core` | Surface-aware picking and application workflow |
| `house_editor` | Asset resolution, material grouping, finish controls |

## Decisions before the relevant implementation

- Default reveal/cap finish: inherit a neighbouring side or use a separate trim
  assignment. Start with a documented default and allow later overrides.
- Whether wallpaper continuity should be manually aligned around an entire room.
  Per-wall phase preservation can ship before a room-perimeter alignment tool.
- Project asset portability: embedded/copied assets versus external references.
  Choose deliberately before enabling arbitrary texture import.

## Validation and completion criteria

- Painting one wall side leaves the opposite side unchanged.
- Curved wallpaper uses continuous distance-based mapping without triangle seams.
- Different finishes survive wall merging; splits preserve texture alignment.
- Reversed wall direction preserves the intended physical finish side.
- Corner and opening reveal assignments follow the documented policy.
- Room-wide application handles physical and logical boundaries correctly.
- Undo/redo and save/load preserve assignments and transforms; missing textures
  produce a visible fallback and useful feedback.
