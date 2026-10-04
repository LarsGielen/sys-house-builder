# Editor step 2: shared edit path and straight walls

This expands step 2 of [the implementation order](editor-implementation-order.md).
Start after [step 1](editor-step-1.md). Work through one checkpoint at a time;
the first wall should use the same draft, validation, and commit path that later
tools will use. Keep the house graph independent of Bevy.

## Tasks

### 2.1 Establish a draft and commit boundary (Done)

Keep pointer-driven draft state separate from the committed `WallGraph`. Give a
proposed edit one place to report either a valid result or a graph/mesh error.
On confirmation, validate the whole proposed graph and mesh before replacing
the committed house. Escape discards the draft. A clone of the small current
graph is a reasonable starting point; measure before optimizing this path.

Learn here: Bevy resources can hold editor state, while `WallGraph` is the
domain value. Rust ownership makes the commit boundary explicit: build and
validate a candidate, then replace the committed value only on success.

**Check:** cancellation and failed validation leave the graph and displayed
house unchanged. The error is visible and clears when the draft becomes valid.

### 2.2 Convert graph coordinates to a Bevy mesh (Done)

Adapt `wall_mesh::generate` output into Bevy mesh assets. Map graph `(x, y, z)`
to Bevy `(x, z, -y)` as chosen in step 0. Apply the same transform to positions,
normals, labels, and later picking rays. Replace or update rendered wall entities
after a commit; keep the ground and camera separate from house rendering.

Learn here: `WallMesh` is a snapshot with positions, normals, indices, and
tagged sections. Bevy entities and asset handles are view state, not wall IDs.
The first adapter can rebuild all walls rather than update pieces incrementally.

**Check:** one wall appears in the expected direction and size. A second edit
refreshes the display without leaving old geometry behind.

### 2.3 Turn a pointer position into a floor-plan point (Done)

Cast the active camera's pointer ray onto the ground plane, and translate the
hit back to graph XY. Only accept a hit when the pointer belongs to the
viewport and the window is focused. Handle a ray parallel to the plane or
pointing away without inventing a position.

Learn here: `Option` is useful for a ray that has no valid ground hit. Keep
screen pixels, Bevy world coordinates, and graph metres distinct in names and
types where practical.

**Check:** a cursor marker tracks the ground under the pointer at several
camera angles. UI interaction does not start a wall draft.

### 2.4 Add snapping with visible priority (Done)

Collect eligible junction/corner, wall, and grid candidates near the pointer.
Choose a target by priority in that order, then by screen-space distance within
the same class. Expose grid spacing in the UI and a temporary snap-disable
modifier. Show the chosen target and the resulting coordinates.

Learn here: a snap result can carry both the position and its source. Apply
one consistent tolerance to graph geometry; use a pixel radius for whether a
target is convenient to select on screen.

**Check:** a junction wins over a nearby wall or grid point, and disabling
snapping gives the unsnapped ground hit. Changing grid spacing changes only
grid candidates.

### 2.5 Draw and preview straight walls

In the wall tool, the first left press starts a draft. A movement threshold
distinguishes a click from a drag: release after a drag confirms, while a click
leaves the draft active for a second click. Preview the proposed length, height,
and thickness in metres. Use `add_wall_with_dimensions` on the candidate graph,
then `wall_mesh::generate`; display an invalid preview and its reason if either
fails. Keep the same path for exact numeric values entered in the UI.

Learn here: an insertion can split or merge existing walls, so its returned
handles are not a complete description of the changed scene. Regenerate from
the candidate graph and treat its mesh as the preview of the entire result.

**Check:** connected walls can be built from an empty graph with both gestures.
The preview follows snapping, Escape cancels it, and invalid geometry never
changes the committed house.

### Step 2 is complete when

- [ ] Straight wall placement uses one candidate/validate/commit path.
- [ ] The rendered mesh agrees with the committed graph after every edit.
- [ ] Snap priority, grid spacing, and temporary snap disable are visible.
- [ ] Invalid graph or mesh proposals show a reason and leave state unchanged.
- [ ] `cargo fmt --all --check`, `cargo check -p house_editor`, and relevant
      `wall_graph`/`wall_mesh` tests pass; wall gestures are tried manually.

## Useful project references

- [Wall graph API and identifier behavior](../crates/wall_graph/README.md).
- [Wall mesh output and validation](../crates/wall_mesh/README.md).

Next: [step 3, picking and selection](editor-step-3.md).
