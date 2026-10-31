# Procedural roof generation

Status: proposed. Order: 9. Depends on building footprint queries and level
ownership. Reuse generation conventions established for
[stairs](multiple-floors-and-stairs.md).

## Intended behavior

Generate roofs from authored definitions and building footprints. Make new roof
types easy to add through a clear validation/generation pipeline. Begin with
limited, explicitly supported footprints rather than a universal roof solver.

## Current support and gaps

There are no roof records, footprint queries for roof purposes, or roof meshes.
Wall geometry supports constant height per piece. A gable roof therefore also
needs a plan for the triangular wall termination below it.

## Proposed architecture

An authored roof has a stable ID, supporting level/elevation, footprint source,
placement, roof type, common settings, and type-specific parameters. A source
may refer to a derived building footprint or an explicit authored override.

Resolve the definition/context, validate it, then generate semantic surfaces and
collision output as one coherent candidate. Meshes remain derived data.

Keep type-specific logic in focused generators with explicit supported-input
checks. Begin with concrete Rust data variants and dispatch. Introduce a trait
only when actual generator implementations establish a useful common contract.

Derive footprints from the exterior building envelope while preserving holes and
disconnected components. Individual room outlines would generate unintended
internal roof boundaries; a convex hull would cover courtyards and recesses.

## Implementation steps

1. Define a roof footprint query and explicit override representation. Keep it
   separate from logical room partitions.
2. Add a flat-roof definition and generator over supported polygonal footprints,
   including thickness, top/bottom, edges, tags, and optional overhang.
3. Add a rectangular gable generator with explicit ridge direction, pitch, and
   overhang. Reject unsupported footprints with a useful error.
4. Add generated gable infill owned by the roof result, or deliberately extend
   the wall-top model. Start with the focused infill approach unless editable
   variable wall tops are a separate requirement.
5. Add another type, such as a hip roof over a supported footprint, to validate
   extension of the pipeline without modifying unrelated generator internals.
6. Add selection, handles/properties, previews, history, and persistence.
7. Extend to concave/compound roofs, valleys, dormers, and curved boundaries as
   separate capabilities, each with explicit supported geometry and tests.

General hip roofs over arbitrary concave polygons require much more topology
work than a rectangular gable. Evaluate suitable algorithms when that milestone
is reached; do not let this complexity block the initial element pipeline.

## Integration rules

- A roof may cover several rooms, a partial level, or an extension. Do not assume
  exactly one roof per room or exactly one roof for the whole building.
- Changes to a footprint or supporting level regenerate affected roofs during
  candidate validation. If regeneration fails, expose the reason and preserve
  the committed document; do not silently retain a stale roof mesh.
- Define overhang offset/join behavior, especially around concave or curved
  boundaries, rather than approximating it with unrelated wall-offset rules.
- Generated infill/caps must avoid duplicate faces against existing walls.
- Roof visibility/cutaway controls are necessary for editing rooms below.

## Systems touched

| System | Responsibility |
| --- | --- |
| Room/building boundary layer | Physical envelope and courtyard/component queries |
| `house_document` | Roof definitions, references, context validation, persistence |
| Roof geometry module | Type-specific surfaces, thickness, infill, collision |
| Wall integration | Wall termination and roof/infill joins |
| Core/editor | Selection, handles, settings, visibility, preview rendering |
| Material rendering | Roof roles, UV coordinates, material resolution |

## Decisions before the relevant implementation

- Initial pitch/ridge controls and whether roofs automatically follow envelope
  changes or use an explicit footprint override.
- How roofs on lower extensions interact with higher-level walls.
- Which curved-wall cases the first pitched roof types support. Unsupported
  cases need clear errors; flat roof support does not imply pitched support.

## Validation and completion criteria

- Flat and rectangular gable roofs generate expected surfaces, winding, normals,
  thickness, and semantic tags.
- Footprint changes regenerate the complete roof/infill result atomically.
- Courtyards are preserved; internal room dividers do not create roof seams.
- Concave overhangs and unsupported pitched footprints either work under their
  declared contract or fail explicitly.
- A second roof type reuses preview, commit, persistence, and rendering plumbing.
- Undo/save/load preserves the definition and rebuilds the intended geometry.
- Inspect headless views of roof coverage and wall termination from several angles.
