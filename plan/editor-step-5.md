# Editor step 5: undo, redo, and document state

This expands step 5 of [the implementation order](editor-implementation-order.md).
Start after [step 4](editor-step-4.md). The editor now has enough operations to
make history useful. Keep history over the editable document, not Bevy entities.

## Tasks

### 5.1 Define one history entry per committed edit

Choose a straightforward history representation for the current document. A
snapshot of the `WallGraph` and editor metadata is reasonable while projects
are small. Record a history entry only when a valid operation commits, not on
every pointer movement. A finished drag, placement, property change, or delete
is one step.

Learn here: `Clone` of an engine-independent document can make the ownership
story simple. History entries should not contain entity IDs, mesh handles,
camera state, or transient drafts.

**Check:** moving an opening through many preview positions adds one undo
step, and cancelling it adds none.

### 5.2 Implement undo and redo through one restore path

On undo or redo, restore graph and category metadata together, discard active
draft/hover state, and rebuild mesh, picking, handles, and properties from the
restored document. A new commit after undo clears the redo branch. Disable
actions that have no available history entry.

Learn here: history restoration is another document replacement, so it should
use the same scene-refresh boundary as a normal commit. Avoid trying to undo
individual Bevy entity mutations.

**Check:** walls, openings, dimension changes, deletions, and categories all
undo and redo as complete operations. A new edit after undo cannot redo the
discarded future edit.

### 5.3 Track whether the document has changed

Show a modified indicator for edits since the last saved or opened state.
Decide how undoing back to that state clears the indicator; a saved revision
marker or document comparison can work. Treat an unsaved new project as its
own initial state. Expose a single query for UI and later file actions.

Learn here: history position alone is insufficient after branching: a new edit
at an old position is not the old saved document. Keep the dirty rule tied to
document content or a unique revision identity.

**Check:** editing marks the project modified, saving marks it clean, undoing
back to saved content clears the indicator, and redoing changes it again.

### 5.4 Prepare unsaved-change handling

Define the choice presented before replacing or closing a modified document:
save, discard, or cancel. It can become active with project I/O in step 6.
Make sure cancel preserves the current document and history. Keep this decision
at the editor boundary, not inside `WallGraph`.

**Check:** a cancelled replacement would leave the current project, selection,
and undo history in place.

### Step 5 is complete when

- [ ] Every committed edit contributes one meaningful undo step.
- [ ] Undo/redo restore geometry and editor metadata together.
- [ ] Scene and picking data match every restored document.
- [ ] The modified indicator handles undo, redo, and branching correctly.
- [ ] `cargo fmt --all --check`, `cargo check -p house_editor`, and focused
      history checks pass; mixed editing sequences are tried manually.

## Useful project references

- [Wall graph handle lifetime](../crates/wall_graph/README.md).
- [Implementation order and document boundary](editor-implementation-order.md).

Next: [step 6, project files](editor-step-6.md).
