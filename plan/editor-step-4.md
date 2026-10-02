# Editor step 4: openings and basic property edits

This expands step 4 of [the implementation order](editor-implementation-order.md).
Start after [step 3](editor-step-3.md). Reuse the candidate graph, mesh
validation, selection, and preview paths for openings and dimension edits.

## Tasks

### 4.1 Place an opening on a chosen host wall

Use wall picking to choose a host. Derive the opening's center distance along
that wall, then preview width, height, bottom elevation, and wall position in
metres. Apply `add_opening` to a candidate graph and regenerate its mesh before
commit. Show graph and mesh errors at the proposed opening.

Learn here: `OpeningSpec::center_distance` follows the wall path, including an
arc; it is not a world X coordinate. A graph-valid opening can still fail mesh
clearance near a resolved junction.

**Check:** an opening cuts the expected hole, follows its host, and an invalid
size or position never changes the house.

### 4.2 Keep door/window category in the editor document

Offer door and window as categories for the same geometric opening. Store the
category with a stable `OpeningId` in editor-owned, engine-independent data.
Update that metadata when openings are added, removed, or transferred by a
graph edit. Do not add door/window-specific mesh behavior yet.

Learn here: `WallGraph` owns geometry and validates openings; the editor
document can hold information the graph does not model. Keep metadata changes
in the same commit as the graph change so they cannot disagree.

**Check:** two identical holes can retain different categories through further
wall insertions that split or merge their hosts.

### 4.3 Pick and select across the empty hole

Add an editor-only hit area spanning each opening. Mesh tags cover jambs,
heads, and sills but no triangles at the center of the hole. Opening selection
should take precedence over its host wall when the pointer is in the opening
area. Show opening position and size handles, with clear hover and selection.

Learn here: picking geometry can be different from rendered geometry. Query
`opening` and `opening_position` from the current graph to construct its hit
area; do not add invisible physical wall triangles to the house mesh.

**Check:** clicking through the center of a doorway selects the opening, while
clicking adjacent solid wall selects the wall.

### 4.4 Edit, duplicate, and remove openings

Use a validated draft to drag an opening along its host, change elevation and
dimensions, and enter exact values. Add deletion and duplication, with the copy
placed at a valid new position. Snap a moved or new opening to another opening's
center or sides, optionally with an offset. Keep each confirmed operation
atomic, including its category metadata.

Learn here: `set_opening` and `remove_opening` validate graph changes, while
mesh generation validates the resulting solid. A duplication creates a new
identity; a move should retain the existing opening identity.

**Check:** a too-wide, overlapping, or junction-crowding edit shows a reason
and leaves both geometry and category unchanged.

### 4.5 Edit wall dimensions and handle wall deletion

Expose exact wall height and thickness with `set_wall_dimensions`, previewing
the new mesh before commit. Let the user delete a selected wall, but show its
attached openings and the effect on the layout before confirmation. Remove
categories for openings that the graph removes with their host wall. Clear any
selection invalidated by the operation.

Learn here: an operation may succeed in `wall_graph` but produce a `wall_mesh`
error. Keep the entire candidate document until both checks succeed.

**Check:** wall dimensions change the visible mesh. Deleting a wall visibly
removes its openings and categories in one operation.

### Step 4 is complete when

- [ ] Doors and windows can be placed, selected through their holes, and edited.
- [ ] Exact properties, deletion, duplication, and opening snapping use previews.
- [ ] Geometry and category metadata commit or reject together.
- [ ] Wall dimension edits and deletion leave no stale openings or selection.
- [ ] Relevant graph/mesh tests and `cargo check -p house_editor` pass; opening
      placement and property editing are tried manually.

## Useful project references

- [Opening API and stability rules](../crates/wall_graph/README.md).
- [Mesh validation and surface tags](../crates/wall_mesh/README.md).

Next: [step 5, undo and document state](editor-step-5.md).
