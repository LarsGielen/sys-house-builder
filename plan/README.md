# Feature implementation roadmap

Status: planning only. Based on the project inspected on 2026-10-30. These
documents propose changes; none of the features below have been implemented as
part of this planning task.

## Requirements established so far

- Pathfinding is for characters, including requests such as "go to the bathroom."
- Moving or resizing a room moves shared walls and reshapes neighbouring rooms.
- Open-plan rooms behave like normal rooms, including selection, editing,
  properties, and destination lookup.
- Rooms are detected automatically. Invisible dividers are an editor wall type
  and separate rooms without creating physical barriers.
- Deleting a separating wall merges the adjoining rooms; changing it to an
  invisible divider preserves their separation.
- Changing a physical wall to an invisible divider automatically removes its
  hosted openings and their metadata as part of the same undoable edit.
- New rooms have unknown classification. Splitting resets both resulting rooms
  to unknown, and merging resets the resulting room to unknown. Furniture-based
  tagging is a later feature.
- Roofs, stairs, and similar generated elements need a clear pipeline that can
  accommodate additional types. The first implementation need not support every
  type.
- Domain logic remains independent of Bevy. Prefer focused modules and explicit
  data over a general plugin framework until that framework is needed.

Open-plan rooms are spaces bounded partly by logical, invisible dividers. Those
dividers participate in room detection but do not obstruct movement. Physical
wall-to-divider conversion is one atomic edit that preserves the room division.

## Recommended order

| Order | Feature | Main prerequisite |
| --- | --- | --- |
| 0 | Document foundations below | Existing proposal/commit pipeline |
| 1 | [Room detection and creation](room-detection.md) | Reliable boundaries and room identity |
| 2 | [Floor mesh generation](floor-mesh-generation.md) | Room regions and wall footprints |
| 3 | [Whole-room editing](room-editing.md) | Room identity and batch edits |
| 4 | [Wallpaper and wall finishes](wallpaper.md) | Surface ownership and edit correspondence |
| 5 | [Multiple floors and stairs](multiple-floors-and-stairs.md) | Level ownership and floor voids |
| 6 | [Furniture placement](furniture-placement.md) | Level ownership and floor coverage |
| 7 | [Character pathfinding](pathfinding.md) | Rooms, walkable floors, openings, stairs, obstacles |
| 8 | [Parametric opening shapes](opening-shape-editor.md) | Profile geometry and generalized cutout meshing |
| 9 | [Procedural roofs](procedural-roof-generation.md) | Building footprints and element generation |

This is a useful integration order, not a strict dependency chain. Furniture can
start on one level before stairs. Single-level navigation can start before
stairs, with cross-level routing added later. Opening profiles and roofs can
move earlier if priorities change; the other systems must not depend on their
full editors being complete.

## Current architecture to preserve

- `wall_graph`: physical wall topology, exact straight/circular geometry,
  dimensions, openings, and geometric mutation invariants.
- `wall_mesh`: resolved wall footprint, junctions, and tagged solid geometry.
- `house_document`: authored building state, validated candidates, undo/redo,
  dirty tracking, and project persistence.
- `house_editor_core`: engine-independent gestures, snapping, picking, and
  workflow.
- `house_editor`: Bevy input, UI, scene resources, asset resolution, and rendering.

Every document edit continues through proposal, complete validation, preview,
and commit. A failed operation leaves the document unchanged. A compound edit
is one undo step, and the committed candidate is the one shown in the preview.

## Shared foundations

### Level ownership

Introduce a level record containing the existing wall graph, initially with only
one level. Give it a stable identifier and an explicit elevation. Keep wall
geometry in the level's local XY plane, with local Z elevation for meshes.

Wall and opening handles are graph-local. References and selection targets must
identify their level as well; equal handles in different graphs are not the same
object. Do not use a level's position in a vector as its persistent identity.

### Edit correspondence and persistent identity

Wall handles can become invalid after splitting, merging, or reshaping.
`ReshapeReport` currently reports rebuilt walls for reshape operations; it does
not provide all the correspondence needed by future boundary/finish metadata.

Describe old-to-new wall portions, parameter ranges, and direction changes for
topology edits. Use that information to transfer wall-side metadata and boundary
attachments. Do not save mesh section indices or temporary face indices as
object identities.

Give rooms, furniture instances, stairs, and roofs their own stable identities.
Keep identity separate from classification. Splits/merges reset all resulting
room classifications to unknown. Define identifier correspondence separately,
and include identity changes and classification resets in the same candidate
and history entry as the geometry change.

### Authored state and derived results

Persist definitions, placements, room metadata, logical dividers, finishes, and
asset references. Regenerate meshes and navigation data; they are derived
results rather than independent editable sources of truth.

Expand `EditCandidate` incrementally to hold coherent authored state and the
derived results required to validate/display it. Begin with full regeneration
where it is simple. Add revision-based caches and rebuild only affected levels
or elements when measured costs justify them. Appearance-only edits should not
eventually require rebuilding wall topology.

### Generated-element pipeline

Use a consistent sequence for stairs, roofs, and opening profiles:

1. Read an authored definition and its placement/context.
2. Resolve dimensions and references.
3. Validate the complete proposed result.
4. Generate geometry and the element-specific outputs needed by consumers.
5. Assemble a candidate, preview it, and commit it atomically.

For stairs, outputs include required floor voids and navigation links; roofs have
different outputs. Share useful conventions without forcing all elements into
one oversized result type. Prefer explicit type variants and focused generator
functions initially. Introduce traits when multiple implementations actually
benefit from a shared contract.

### Persistence and compatibility

The current version-1 format stores walls and rectangular openings using
file-local IDs. New records need deliberate versioning, reference validation,
and import migration. Map a version-1 project to one default level. Keep old
projects readable as the format evolves, and load/validate a complete candidate
before replacing the open document.

Do not serialize incidental runtime wall IDs as if they were durable identities.
Explicitly reconstruct/remap references when loading and test that metadata
returns to the intended geometry.

## Delivery and validation

Deliver each feature vertically: domain rules, candidate/preview support,
minimal editor interaction, history, and persistence. Do not postpone save/load
or identity handling until after several features depend on them.

Tests belong under each crate's `src/tests/`, using the repository's existing
test-module convention. Cover geometry, rejected operations leaving state and
IDs unchanged, undo/redo, stale references, and project round trips. Use the
headless screenshot tool for visual checks; write captures outside the repo.

Run targeted tests for affected crates and `cargo fmt --all` after Rust changes.
Broaden checks when integration changes justify it. These are future validation
requirements; this planning task has not run them.

Do not add dependencies without approval. Start with modules in the current
crates; introduce a new crate or shared abstraction when its responsibility is
clear and the dependency direction remains intentional.

## First integrated milestone

Create two adjacent rooms and an open-plan division, generate their floors, and
resize a shared boundary. The complete result should preview, undo, save, and
reload correctly. This exercises the central model before character simulation
and complex generated elements depend on it.
