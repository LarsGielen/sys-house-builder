# Editor step 6: project files

This expands step 6 of [the implementation order](editor-implementation-order.md).
Start after [step 5](editor-step-5.md). Save the editable document, not a Bevy
scene or an internal dump of `WallGraph` fields.

## Tasks

### 6.1 Define a small versioned file format

Document its version, metre units, graph XY floor-plan coordinates, and Z
elevation. Store resolved straight/arc wall geometry, dimensions, openings,
and editor metadata such as door/window categories. Do not store drawing
history or tangent flags. Choose a human-inspectable representation, and ask
before adding any serialization dependency under `AGENTS.md`.

Learn here: a file schema is a public boundary that can outlive the current
Rust structs. Use explicit format types and conversions so private graph
storage and Bevy components can change independently.

**Check:** a sample file makes its version, units, coordinates, wall shapes,
opening sizes, and categories understandable without running the editor.

### 6.2 Reconstruct and validate a candidate document

Parse the entire file into format values, validate numeric values and
references, and reconstruct a fresh graph through supported domain operations.
Then run `wall_mesh::generate` before accepting it. Report unsupported versions
and invalid data with an actionable location or object reference. Because graph
reconstruction can split or merge walls, resolve opening hosts and categories
through stable file-local references rather than saved raw wall handles.

Learn here: validation belongs at the import boundary. A partially rebuilt
graph must remain local until the whole document and mesh are valid. Consider
round-trip cases where insertion order changes the returned wall pieces.

**Check:** a malformed number, missing host, unsupported version, or mesh
failure rejects the whole import and leaves the current project intact.

### 6.3 Save without damaging an existing file

Serialize a complete document, write it to a temporary file in the target
directory, and replace the destination only after writing succeeds. Handle
write and replacement errors visibly. Do not mark the document clean on a
failed save. Keep file-path and error handling outside the graph crate.

Learn here: a `Result` from each file step should reach the UI. A temporary
file in the same directory supports a final rename on the same filesystem;
decide how to clean up that file if a step fails.

**Check:** a failed save preserves the previous file and leaves the editor
modified. A successful save updates the file and clears the modified indicator.

### 6.4 Connect open, save, and unsaved-change choices

Wire file actions to the editor state. Before opening another project or
closing a modified one, offer save, discard, or cancel. Only replace the active
document and history after a candidate import succeeds. After opening, rebuild
mesh, picking, handles, and status from the new document.

Learn here: opening a file is another atomic document replacement. Separate
the path chosen in the UI from the parser and validator so the latter can be
tested without Bevy.

**Check:** cancel retains the current project. A rejected import preserves
its geometry, history, and modified state.

### 6.5 Round-trip the full model

Save and reopen a layout containing straight walls, arcs, different wall
dimensions, openings, and both categories. Compare the reconstructed geometry
and metadata, allowing graph IDs to differ. Check that tangency is inferred
from restored geometry instead of a file flag.

Learn here: the graph's internal IDs are runtime handles, while file-local
references describe persistent relationships. A round-trip test should compare
meaning, not private storage order.

**Check:** the reopened layout renders the same valid house and preserves
opening categories. Saving it again does not lose information.

### Step 6 is complete when

- [ ] The format is versioned, readable, and documents units and coordinates.
- [ ] Import validates the whole file and mesh before replacing the project.
- [ ] Failed save and failed import preserve the prior file or document.
- [ ] Walls, arcs, dimensions, openings, and categories survive a round trip.
- [ ] Focused format tests and `cargo check -p house_editor` pass; open/save
      behavior is tried manually, including unsaved-change choices.

## Useful project references

- [Wall graph reconstruction behavior](../crates/wall_graph/README.md).
- [Wall mesh validation](../crates/wall_mesh/README.md).

Next: [step 7, curved-wall tools](editor-step-7.md).
