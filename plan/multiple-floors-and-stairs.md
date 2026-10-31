# Multiple floors and stairs

Status: proposed. Order: 5. Builds on level ownership from the
[roadmap](README.md) and [floor coverage/voids](floor-mesh-generation.md).

## Intended behavior

Edit several building levels and connect them with stairs. Make additional stair
types easy to add through a consistent generation pipeline. Start with straight
stairs to validate the complete integration.

## Current support and gaps

The document currently contains one wall graph and mesh. Geometry is local XY
with Z elevation; the Bevy adapter rotates this to Y-up. The pointer intersects
the world plane at elevation zero. Selection and handles currently assume one
graph. There are no floor voids, stair definitions, or vertical navigation links.

## Level model

Each level has a stable ID, name, explicit base elevation, wall graph, room
information, and floor data. Keep local geometry independent of its world
elevation. Floor-to-floor spacing and individual wall heights are separate
quantities; changing one must not silently redefine the other.

Use level-qualified references for graph-local wall/opening handles. Active level
and visibility are editor state, not substitutes for ownership in the document.
An elevation change is a document edit with validation of affected connectors.

## Stair pipeline

An authored stair references source/destination levels and describes placement,
orientation, type, and type-specific parameters. Resolve its total rise from
the level/floor definitions under an explicit convention.

Generation produces the relevant outputs together:

- Render geometry with semantic surface tags.
- Collision and clearance geometry.
- Required slab voids and landing requirements.
- Navigation endpoints and a traversable connection/path description.

Derived stair-owned voids should retain their ownership relationship rather than
becoming unrelated holes the user must keep synchronized manually. If manual
overrides are allowed later, define how they replace the derived result.

Begin with explicit stair variants and focused generator functions. Keep common
placement/context validation outside individual generators, and type-specific
rules inside them. Add L/U/spiral variants without changing the document commit,
asset, or navigation integration model. Do not add a plugin registry prematurely.

## Implementation steps

1. Expose level creation, naming, activation, ordering, and elevation edits.
2. Update rendering transforms, pointer planes, snapping, picking, and selections
   to use explicit levels. Add visibility controls so upper floors do not obscure
   the active work area.
3. Support projected reference snapping between levels without accidentally
   creating shared graph nodes across them.
4. Add a straight stair definition, generator, and clear preview handles.
5. Resolve rise, step count, tread/riser layout, landings, and headroom; report
   invalid combinations explicitly.
6. Create the stair and required floor voids in one validated candidate. Include
   impacted levels when either level elevation changes.
7. Add save/load migration from a version-1 project to one default level, and
   persist authored stair definitions and references.
8. Add another stair type to exercise the extension boundary before generalizing
   the generator API further.

## Systems touched

| System | Responsibility |
| --- | --- |
| `house_document` | Levels, scoped references, stairs, compound validation, persistence |
| Floor generation | Stair-owned voids and landing/support checks |
| Stair geometry module | Mesh, collision, clearance, navigation connection outputs |
| `house_editor_core` | Level-aware pointing, picking, snapping, stair gestures |
| `house_editor` | Level UI, visibility, transforms, stair rendering |
| Navigation adapter | Inter-level connector input, when navigation is added |

## Decisions before the relevant implementation

- Define base elevation relative to finished floor and slab thickness.
- Define step-count rounding and which dimensions are user-controlled versus
  derived. Do not imply compliance with building regulations without separately
  establishing the intended rules.
- Decide deletion behavior for levels with stairs or other dependent objects.
  Reject or explicitly include dependent deletion; never leave dangling links.

## Validation and completion criteria

- Equal numeric wall handles on two levels remain distinct selection targets.
- Each level's pointer and snapping operate at the right elevation.
- A stair and its void preview/commit/undo as one operation.
- Changing an endpoint level updates the complete stair result or rejects the
  change atomically when clearance/landing rules fail.
- New stair types reuse the integration pipeline without engine-specific domain
  logic or changes to unrelated stair definitions.
- Cross-level references and old single-level projects load correctly.
