# Editor feature list (draft)

This is an editable list of capabilities and open design questions. Sections
group related features; their order does not indicate implementation priority.

## 3D viewport and camera

- [ ] Use an orbiting 3D editor camera.
- [ ] Reserve left click and drag for building tools and object manipulation.
- [ ] Use middle drag to orbit, right drag to pan, and the wheel to zoom.
- [ ] Focus or frame the current house or selection.
- [ ] Show a ground plane and scale cues.
- [ ] Prevent viewport actions when the pointer is over the UI.

## Selection and feedback

- [ ] Highlight the wall or opening under the pointer before selection.
- [ ] Click a wall surface to select that wall piece; click empty space to clear
      selection. Escape clears selection when no edit is in progress.
- [ ] Give openings an editor-only picking area across the empty space so they
      can be selected by clicking inside them.
- [ ] Show the selected object's properties and a clear visual outline.
- [ ] Show draggable edit handles for wall endpoints/junctions and for an
      opening's position and size. Show a bend handle for a free curved wall.
- [ ] Clear selection when entering a wall-building tool. If any edit still
      splits, merges, or removes the selected wall, clear its stale selection.
      A crossing can replace one graph wall with multiple pieces.
- [ ] Consider multi-selection if editing workflows need it.

## Wall creation and editing

- [ ] Draw straight walls in the 3D view by choosing start and end points on
      the ground plane.
- [ ] In a wall draw tool, the first left press starts a preview. A drag sets
      the endpoint and releasing confirms it when the shape is fully specified;
      a click leaves the preview active and a second click confirms it when
      ready. A free arc also needs its bend specified. Use a movement threshold
      so small pointer motion does not turn a click into a drag.
- [ ] Draw curved walls by choosing endpoints and setting the bend with a third
      pointer position or an exact radius/angle in the properties panel. A free
      arc needs this additional bend step after its endpoints are chosen.
- [ ] Allow a new curved wall to be tangent to one connecting wall, or to both
      connecting walls when a valid circular arc exists. In tangent modes,
      derive the curve from the endpoints and connection tangents instead of
      asking for a separate bend control.
- [ ] Preview a wall with its proposed length, height, and thickness before
      committing it.
- [ ] Enter exact wall height and thickness in the properties panel.
- [ ] Delete a selected wall with clear feedback about affected geometry.
- [ ] Define wall move and endpoint drag behavior, including what happens to
      connected walls and openings. Connected geometry should follow the drag
      where possible. Preview the entire result and show invalid geometry in
      red without changing the committed house. The domain model needs an
      atomic operation for these edits.
- [ ] When a junction moves, infer existing tangencies from the connected
      walls' geometry within a consistent tolerance. Try to keep those
      tangencies while adjusting connected arcs; do not store tangent flags.
- [ ] If the new endpoints cannot support the inferred tangencies, preview a
      valid curve that keeps the dragged endpoints fixed and minimizes change
      from the original arc. Make any lost tangency visible before commit.
      Keep the committed house unchanged if the resulting graph or mesh is
      invalid.

## Openings, doors, and windows

- [ ] Place an opening by pointing at a wall in the 3D view.
- [ ] Preview opening width, height, bottom elevation, and position on its host
      wall before committing it.
- [ ] Drag an opening along its wall and change its elevation through a
      validated preview.
- [ ] Enter exact opening dimensions and position in the properties panel.
- [ ] Preserve and revalidate openings when editing their host wall. The graph
      already transfers openings on splits and merges and keeps their world
      positions; new wall/junction move operations must define their own
      opening behavior.
- [ ] Delete and duplicate openings.
- [ ] Offer door and window categories for openings in the UI and saved
      project. Both use the same geometric opening model for mesh generation;
      the category preserves the user's choice without adding door- or
      window-specific behavior yet.
- [ ] Add visible door and window objects when their appearance and behavior
      are defined.

## Snapping and precision

- [ ] Snap placement to wall corners/junctions, wall geometry, and a grid, in
      that priority order when targets are within a useful pointer distance.
- [ ] Let the user configure grid spacing in the UI.
- [ ] Show the active snap target and allow snapping to be temporarily disabled.
- [ ] Show useful lengths, offsets, and elevations during placement and edits.
- [ ] Use metres consistently in the model, viewport labels, and properties.
- [ ] Snap openings to the centers and sides of other openings, with an offset
      option when aligning beside an opening.

## Preview and validation

- [ ] Keep proposed changes separate from the committed house until accepted.
- [ ] Validate both the proposed graph operation and mesh generation before
      commit; a graph edit can succeed while meshing fails.
- [ ] Show why a proposed edit is invalid, preferably near the affected
      geometry.
- [ ] Let Escape cancel an in-progress placement or manipulation.
- [ ] Rebuild displayed geometry and picking data after a committed edit.

## Editing history and document state

- [ ] Undo and redo committed placements, moves, dimension changes, and
      deletions.
- [ ] Show whether the current project has unsaved changes.
- [ ] Handle closing or opening another project when changes are unsaved.

## Project files

- [ ] Choose a versioned, human-inspectable format with documented units and
      coordinates.
- [ ] Save/export the complete editable house model to a data file.
- [ ] Open/import a file only after validating it fully; report actionable
      errors for bad data or unsupported versions.
- [ ] Ensure a failed save does not damage the previous file.
- [ ] Verify that walls, arcs, dimensions, and openings survive a save/reopen
      round trip.
- [ ] Save the resolved wall geometry and openings needed to reconstruct the
      current graph, along with editor data such as door/window categories.
      Drawing history and tangent flags are not required for the current house
      state; tangency can be inferred from the restored geometry.

## Extensibility and performance

- [ ] Keep house data and validation independent of Bevy. The renderer adapts
      domain data rather than owning it.
- [ ] Make building tools and the properties UI easy to extend with new object
      types, without introducing a large framework prematurely.
- [ ] Support one floor for now; define how elevations work when adding stairs
      or upper levels later.
- [ ] Add furniture placement and editing when the document model can represent
      independent objects.
- [ ] Measure mesh regeneration and picking on realistic projects; optimize
      only where interaction becomes too slow.
- [ ] Later, add a way to hide or cut away obscuring walls so interior objects
      can be edited. Choose the visual technique when implementing it.
