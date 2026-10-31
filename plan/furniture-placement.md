# Furniture placement

Status: proposed. Order: 6. Can start before stairs once level ownership and
[floor coverage](floor-mesh-generation.md) exist.

## Intended behavior

Place, move, rotate, duplicate, and delete furniture. Instances remain editable
and persistent, and expose obstacle information for character navigation. Future
usable objects should offer explicit interaction locations.

Furniture content will also supply automatic room tags, such as bathroom or
kitchen. Room detection initially leaves classification unknown; this feature
introduces classification from contents rather than inheriting old room types.

## Current support and gaps

The dock already contains a furniture catalog and icons, but all entries are
placeholders. There is no furniture domain model, placement proposal, collision
representation, or selection target.

## Proposed architecture

Separate reusable definitions from placed instances:

- Definition: stable asset reference, visual resource, placement rules, collision
  geometry, navigation obstruction, and optional interaction anchors.
- Instance: stable ID, definition reference, level, position, orientation, and
  any supported instance overrides.

Keep Bevy handles/entities out of the saved model. The editor resolves visual
assets; the domain works with dimensions, shapes, and references.

Visual mesh, placement collision, and navigation obstruction have different
purposes. A table's overhang and the area a standing character cannot occupy
need not match. Keep enough vertical information to avoid assuming every
overlapping 2D footprint is a forbidden 3D collision.

Start with floor-standing instances and simple engine-independent collision
shapes. Wall-mounted objects, stacking/support relationships, and arbitrary
scaling are separate extensions with additional invariants.

## Implementation steps

1. Define a minimal furniture definition/instance model and stable IDs.
2. Add a small built-in catalog with simple visuals so imported asset plumbing
   does not block validating placement behavior.
3. Add placement proposals and a ghost preview on the active level.
4. Validate finite transforms, support, voids, and relevant solid collisions.
   A door opening can permit placement across its plane; a blanket wall-centerline
   collision rule would reject valid geometry.
5. Add move/rotate handles, typed rotation, grid/wall snapping, and selection.
6. Add duplication/deletion, history, and persistence.
7. Export static navigation obstruction data. Mark relevant changes so navigation
   can update without depending on editor rendering entities.
8. Add interaction anchors when character/object use is introduced, including
   local position, facing, and required standing clearance.
9. Add room tagging from furniture content as a separate classification pass.
   Keep tagging rules independent of placement collision and navigation. Resolve
   evidence from furniture definitions and spatial room membership, and update
   affected tags when contents move, change, or disappear. Decide the supported
   room tags, mixed-use rules, and confidence thresholds before implementing it.

Room membership can be derived from placement rather than becoming an ownership
rule. A chair may straddle a logical room divider. Keep level ownership explicit.

Keep classification derived from current contents, with an unknown fallback when
evidence is insufficient. A topology split/merge resets its resulting rooms to
unknown first; any subsequent content-based classification is an explicit later
stage, not inheritance from the replaced rooms. Decide when that pass runs before
integrating it, and ensure undo/load gives a coherent classification result.

## Interaction and validity policy

- Distinguish a physically invalid placement from an accessibility warning.
  Blocking a route does not automatically make a document mutation invalid.
- Decide whether room translation offers to include furniture. Resizing a room
  should not silently move its contents by default.
- Missing visual assets should produce a visible placeholder and useful feedback
  while preserving the saved instance/reference. Collision metadata must remain
  well-defined rather than silently vanishing.

## Systems touched

| System | Responsibility |
| --- | --- |
| `house_document` | Definitions, instances, placement validation, history, persistence |
| Geometry/collision modules | Support and overlap queries |
| `house_editor_core` | Furniture gestures, picking, snapping, selection |
| `house_editor` | Catalog activation, visuals, ghost/handle rendering, asset resolution |
| Navigation adapter | Obstruction and interaction-target inputs |

## Validation and completion criteria

- Place/rotate/move an instance with a faithful preview and stable identity.
- Reject invalid support, void overlap, and prohibited collisions atomically.
- Preserve valid allowed vertical overlaps under the documented placement rules.
- Duplicate creates a distinct ID; deletion and undo restore the intended state.
- Logical room dividers do not act as furniture collision walls.
- Save/load retains definition references and transforms, including missing assets.
- Navigation consumes definition/instance geometry without inspecting Bevy meshes.
- Classification responds to room contents under the chosen tagging rules;
  insufficient evidence produces unknown, and split/merge never copies old tags.
