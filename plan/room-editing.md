# Whole-room creation and editing

Status: proposed. Order: 3. Depends on [rooms](room-detection.md),
[floors](floor-mesh-generation.md) for integrated feedback, and batch candidates.

## Intended behavior

Create and manipulate a complete room instead of placing every wall separately.
Moving/resizing a shared boundary reshapes neighbouring rooms. Open-plan rooms
use the same interaction model, with logical boundary portions where needed.

## Current support and gaps

The editor already has wall/arc drawing, wall/junction dragging, snapping, and
validated previews. `WallGraph::move_nodes` accepts multiple node moves, but the
document's current `MoveProposal` carries only one or two nodes.

Wall insertion rejects duplicate/overlapping paths. A room tool cannot simply
insert all its boundary walls over an existing layout. Current reshaping also
rejects new crossings, overlaps, or landing on another node.

## Proposed architecture

Room tools produce higher-level proposals against the shared boundary model.
They do not own an independent set of walls or a second authoritative polygon.
One candidate includes all affected walls, logical dividers, room associations,
floor output, and metadata transfers.

New rooms are discovered automatically with unknown classification. Splitting a
room makes both results unknown; merging rooms makes the result unknown. Moving
or resizing a room without a split/merge preserves its identity/classification.
Deleting a separator merges rooms; converting its wall type to invisible keeps
the division and automatically removes hosted doors/windows and their metadata.
Follow the same rules in single-wall and whole-room tools, with conversion and
opening removal in one undo step.

Reuse the existing gesture/preview architecture. Keep gesture state in
`house_editor_core` and domain validity in `house_document`/geometry modules.
Commit the complete operation as one undo step.

## Implementation steps

1. Add rectangular room creation with typed dimensions and a drag preview.
2. Plan boundary reuse before insertion: identify existing compatible stretches,
   split where necessary, and insert only missing physical portions. Explicitly
   decide how requested dimensions interact with existing shared walls.
3. Add closed polygon room drawing, initially using straight segments.
4. Add room selection and boundary handles, including logical divider handles.
5. Generalize movement proposals to carry all affected junctions. Moving a
   boundary moves its junctions; incident neighbouring walls follow.
6. Add complete-room translation using the same atomic mechanism. Preview all
   affected neighbours, not only the selected room.
7. Update room identity anchors and logical-boundary attachments from edit intent
   and correspondence. Recompute derived floors and preserve finishes/openings.
8. Add curved boundaries and richer dimension controls once simple edits work.

## Geometry and interaction policy

- Initially retain the no-new-crossings restriction during drags. Show rejected
  layouts clearly. Topology-changing drags are a later capability, not an
  automatic consequence of adding room handles.
- Avoid scaling an arbitrary polygon and treating it as a valid room edit.
  Non-uniform scaling turns circular arcs into ellipses and can break shared
  geometry. Move selected boundaries and re-solve supported curves instead.
- Preserve wall thickness. Define whether displayed room dimensions measure
  clear interior space or wall centerlines; make the UI explicit.
- Reuse the current opening placement policy during wall stretching initially:
  positions follow the same fraction along the host wall, and invalid fits
  reject the candidate.
- Do not automatically move furniture when resizing a room. Decide whole-room
  translation/group behavior when furniture integration is implemented.

## Systems touched

| System | Responsibility |
| --- | --- |
| `wall_graph` | Batch reshaping and insertion planning/correspondence |
| `house_document` | Compound proposals, metadata transfer, history |
| Room boundary layer | Shared physical/logical boundaries and room association |
| `house_editor_core` | Room drawing/drag gestures, handles, snapping, picking |
| `house_editor` | Tools, dimensions, neighbour highlights, previews |

## Validation and completion criteria

- Create an isolated room and one adjoining an existing room without duplicate
  walls or unintended dimension changes.
- Move a shared boundary and verify both rooms, openings, finishes, and floors.
- Moving a complete room updates shared neighbours as required.
- Open-plan rooms expose equivalent selection and boundary-edit behavior.
- Reject a crossing or an opening that no longer fits without partial changes.
- One undo restores the entire operation, including identities and metadata.
- Save/load retains the resulting rooms and geometry; inspect headless previews.
