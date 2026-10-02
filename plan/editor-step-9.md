# Editor step 9: later breadth and performance

This expands step 9 of [the implementation order](editor-implementation-order.md).
This is a set of later extensions, not a requirement to implement them all at
once. Take each one through the editor paths established in steps 2–8.

## Tasks

### 9.1 Make interiors accessible in the viewport

Add a way to hide or cut away obscuring walls while editing interiors. Keep
the complete house in the document and in saved files; visibility changes
only rendering and picking. Choose the visual technique after trying it on
a room with openings and adjacent walls.

Learn here: display visibility is editor state, while wall existence is domain
state. Picking should agree with what the user can currently see and target.

**Check:** cutaway reveals interior targets without deleting walls, changing
their mesh validation, or marking the document modified.

### 9.2 Add visible door and window objects

When appearance and behavior are defined, render door/window objects from
the categories and opening geometry already stored in the document. Keep
them synchronized with opening movement, resizing, deletion, undo/redo, and
file loading. Decide whether an object needs additional domain properties
before extending the file schema.

Learn here: a visual object can be a scene adapter over existing document
data. Add new model data only for behavior the user can edit or save.

**Check:** visible objects follow their openings and restore correctly after
undo/redo and save/reopen.

### 9.3 Extend the document for independent objects and floors

When domain rules are clear, add furniture, stairs, or upper floors one object
type at a time. Define coordinates, elevation, ownership, validation, and
persistence outside Bevy before adding placement UI. Consider multi-selection
only after a concrete editing workflow needs it.

Learn here: reuse the existing draft, selection, commit, history, and project
format boundaries. Avoid a generic tool framework until repeated object types
show which behavior is actually shared.

**Check:** each new object type can be placed, selected, changed, undone,
saved, and reopened without a separate editing lifecycle.

### 9.4 Measure before changing rebuild strategy

Try realistic houses and measure wall mesh generation, scene replacement,
picking, and preview latency separately. Use the existing mesh benchmark as
one input, then profile the interaction path that is actually slow. Optimize
only the measured bottleneck and compare the same scenario afterward.

Learn here: whole-graph rebuilds are simple and correct; incremental updates
add identity and cache invalidation work. Keep a correctness comparison to a
full rebuild when introducing a faster path.

**Check:** the chosen interaction is measurably faster on a representative
house and still produces the same geometry and picking behavior.

### Step 9 is complete when

- [ ] Each chosen extension follows the established editor lifecycle.
- [ ] Visibility aids do not change domain geometry or dirty state.
- [ ] New persisted object types have engine-independent rules and round trips.
- [ ] Performance changes are backed by before/after measurements and a
      correctness check on the same representative houses.

## Useful project references

- [Editor feature list](editor-features.md).
- [Wall mesh benchmarks](../crates/wall_mesh/README.md#benchmarks).
