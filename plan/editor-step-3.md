# Editor step 3: picking, selection, and edit handles

This expands step 3 of [the implementation order](editor-implementation-order.md).
Start after [step 2](editor-step-2.md): committed straight walls should already
render and rebuild correctly. Add wall selection now; prepare the picking model
for openings without implementing opening placement yet.

## Tasks

### 3.1 Define editor hit targets

Represent a hit as a domain target such as a wall, junction, or eventually an
opening, plus hit distance and position. Use `SurfaceSource` on generated mesh
sections to associate wall triangles with the current graph. Keep picking data
in sync with each mesh rebuild. Decide how a junction surface maps to a wall
selection when several walls meet; expose a handle as a separate target later.

Learn here: a Bevy entity ID identifies a rendered entity, while a `Wall`
handle identifies one graph state. `SurfaceSource` tags belong to the meshed
snapshot, so never carry a wall handle across a graph replacement unchecked.

**Check:** pointing at different wall pieces reports the expected current
wall, including near junctions and end caps.

### 3.2 Add hover and click selection

Raycast only from a pointer owned by the viewport. Highlight the hovered wall
without committing selection. A left click in the selection tool selects the
nearest eligible wall; clicking empty space clears selection. Escape clears it
when no edit is active. Entering a wall-building tool also clears selection.

Learn here: hover is transient input-derived state; selection persists until a
user action or model change invalidates it. Keep selection rules in one place
so tools do not each implement their own stale-handle behavior.

**Check:** hover and selection are visually distinct. UI clicks cannot select
walls, and empty-space clicks clear the current selection.

### 3.3 Show details and current edit handles

Display selected wall length, height, thickness, and endpoint coordinates.
Draw visible endpoint and junction handles using `wall_graph` queries. The
handles can remain informational until wall manipulation arrives in step 8;
avoid promising a drag that has no implementation yet. Ensure highlights and
handles follow the selected wall after a view refresh.

Learn here: rendering a selected object can query its geometry from the graph
instead of retaining a second editable copy of it in Bevy. Keep metres and
graph-to-world conversion consistent with step 2.

**Check:** the properties panel and handles match the selected graph piece,
including a wall that was split by a later insertion.

### 3.4 Invalidate stale selection after edits

After every commit, check whether the selected handle still refers to the
intended wall. Clear it when insertion, merging, deletion, or rebuild makes it
stale; never allow an old handle to highlight a different piece. Refresh hover
and hit data from the same committed graph and mesh.

Learn here: wall handles are not permanent object IDs. `OpeningId` has different
stability guarantees, which will matter in step 4. Do not use `optimize` as an
implicit scene refresh unless every external reference is remapped.

**Check:** selecting a wall, then crossing or replacing it with another edit,
cannot leave a misleading highlight or panel value.

### Step 3 is complete when

- [ ] Hover, selection, clear selection, and properties work on current walls.
- [ ] Endpoint/junction handles are visible and agree with graph geometry.
- [ ] Rebuilding the mesh also rebuilds or invalidates picking identities.
- [ ] A stale wall handle never selects or highlights another wall.
- [ ] `cargo fmt --all --check` and `cargo check -p house_editor` pass, and
      picking is tried manually at junctions, caps, UI edges, and after splits.

## Useful project references

- [Wall graph API and handle lifetime](../crates/wall_graph/README.md).
- [Surface tags and mesh sections](../crates/wall_mesh/src/output.rs).

Next: [step 4, openings and properties](editor-step-4.md).
