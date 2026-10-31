# Room detection and creation

Status: proposed. Order: 1. See the [roadmap](README.md) for shared foundations.

## Intended behavior

Closing a boundary automatically discovers a room; no separate create-room
confirmation is needed. Rooms have stable identity and initially have an unknown
classification. Open-plan rooms provide the same selection, editing, properties,
and destination behavior without requiring physical walls around their entire
boundary.

## Confirmed behavior

- Invisible dividers are available as a wall type in the editor. They participate
  in room detection but produce no physical wall or movement obstruction.
- Deleting a separating wall merges its adjoining rooms when they become one
  region. Do not retain an invisible divider or ask whether to preserve the split.
- To retain the room division while opening up the space, the user changes the
  physical wall's type to an invisible divider instead of deleting it.
- Converting a physical wall to an invisible divider automatically removes its
  hosted doors/windows and their opening metadata in the same undoable edit.
- Dividing a room resets the classification of both resulting rooms to unknown.
  Merging rooms likewise produces an unknown room, even if both had the same type.
- Furniture-based room tagging is later work. The initial detector must not infer
  bathroom/kitchen/etc. from shape or inherit those classifications across a
  split or merge.

Whole-room drawing and manipulation are covered in
[room editing](room-editing.md); this feature supplies the spatial model they use.

## Current support and gaps

`wall_graph` already stores ordered half-edge links following the boundary to
the left of each directed edge. Tests exercise interior/exterior loops and
dangling walls, but there is no public room-query API or stored face/room object.
Straight and circular wall geometry is available through `WallCurve`.

A room cannot simply be a set of wall handles: handles change during edits,
open-plan boundaries may have no wall, and one wall can border two rooms.

## Proposed architecture

- Keep physical walls in `wall_graph`.
- Add a room boundary layer in the engine-independent domain. Physical boundary
  portions refer to wall geometry; explicit logical dividers supply invisible
  boundary portions. Present physical/invisible as wall types through a common
  editing interaction; that does not require giving an invisible divider fake
  thickness or adding it to the physical wall mesh.
- Derive regions from the combined boundary arrangement. A divider crossing a
  physical wall may split the derived arrangement without splitting the physical
  wall itself.
- Store stable room records and metadata in `house_document`, separate from the
  transient region traversal results.
- Keep room identity separate from classification. One-to-one room reshapes can
  preserve identity; splits/merges have explicit correspondence and reset all
  resulting classifications to unknown. Do not use a room's type as its identity.
- Preserve directed boundary portions so consumers can resolve which wall side
  faces each room.

Use existing wall half-edge traversal for the enclosed-wall milestone. Extend
the boundary layer to a combined arrangement for logical dividers, reusing
focused traversal/geometry utilities where appropriate. Do not maintain two
unrelated room detectors or insert zero-thickness walls into `wall_graph`.

Changing a physical wall to an invisible divider is one atomic boundary-type
operation. Preserve its curve in the logical arrangement while removing physical
geometry. Do not commit an intermediate deletion/merge, since the room division
is supposed to survive this conversion. Remove hosted openings and their
metadata in the same candidate; clear selections that refer to removed openings.
Undo restores the physical wall, openings, and metadata together. A divider
crossing an opening must not create a physical junction clearance requirement or
an extra wall pier.

Distinguish geometric enclosure from physical floor support. A logical region
does not automatically establish a supported floor outside the building.

## Implementation steps

1. Add a focused boundary query that returns directed wall/curve portions while
   keeping private half-edge storage private.
2. Classify bounded regions, exterior components, holes, and degenerate walks.
   Account for disconnected/nested loops and dangling walls. A closed walk or
   area sign alone is not a complete room classifier.
3. Compute region area and point containment, including arcs. Keep topology
   independent of rendering samples and use consistent geometric tolerances.
4. Add stable room records, level ownership, names, unknown classification, and region
   association. Anchor points can help association but need edit correspondence
   and explicit handling when an edit moves, splits, or merges a room. Apply the
   confirmed classification reset policy within the same candidate.
5. Add logical dividers and derive regions from physical and logical boundaries.
   Add physical/invisible wall-type conversion and deletion semantics. Define
   attachment behavior when a host wall moves or disappears.
6. Add room picking, highlighting, properties, history, and persistence. Display
   unknown classification; automatic furniture-based tags arrive with furniture.
7. Emit enough room correspondence information for selection, floors, finishes,
   and destination references to follow a committed edit.

## Systems touched

| System | Responsibility |
| --- | --- |
| `wall_graph` | Focused boundary/curve queries and edit correspondence |
| `house_document` | Room records, dividers, derived regions, proposals, persistence |
| `house_editor_core` | Region picking, room selection, boundary-type workflow |
| `house_editor` | Region overlays, labels, room properties, wall-type control |

## Decisions before the relevant implementation

- Identity correspondence after splits/merges, independently of the confirmed
  classification reset. Recommend retiring replaced room IDs and reporting their
  successors, while preserving IDs for one-to-one reshapes. Stale destination
  references must be detected rather than silently retargeted.
- Name handling if custom room names are supported. Classification resets are
  confirmed; they do not by themselves specify a custom-name retention policy.
- Whether disconnected pieces may share one room identity. Start with one
  connected region per room unless there is a concrete need otherwise.

## Validation and completion criteria

- Detect a rectangle, concave room, curved enclosure, adjacent rooms, and nested
  components correctly, independent of wall insertion direction/order.
- Handle dangling partitions without manufacturing extra rooms.
- Closing a valid boundary creates a selectable unknown room automatically.
- An invisible divider creates distinct spaces without physical collision.
- Deleting a separating wall merges its rooms into one unknown room. If removal
  opens a room to the unbounded exterior, it ceases to be an enclosed room.
- Changing the wall to an invisible divider preserves the existing room division
  without an intermediate merge or unintended classification reset.
- Conversion removes all hosted openings and metadata without leaving dangling
  references. One undo restores the wall and openings; redo removes them again.
- Splitting any classified room makes both results unknown; merging rooms makes
  the result unknown even when their original classifications matched.
- Moving/splitting/removing walls updates room associations coherently.
- Invalid divider edits leave state unchanged; undo and save/load preserve room
  identity, classification/reset results, and intended region associations. Undo
  restores earlier classifications as part of the restored document snapshot.

This feature unblocks floor generation, whole-room editing, room-wide finishes,
and semantic character destinations.
