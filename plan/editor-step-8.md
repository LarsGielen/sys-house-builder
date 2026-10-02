# Editor step 8: wall and junction manipulation

This expands step 8 of [the implementation order](editor-implementation-order.md).
Start after [step 7](editor-step-7.md). These edits can affect several walls
and openings, so define the domain operation before wiring up drag handles.

## Tasks

### 8.1 Specify the effect of each drag

Write down what moving a whole wall, dragging one endpoint, and dragging a
shared junction should do to connected straight and curved walls. Include
opening positions and dimensions on affected hosts. Decide which points stay
fixed and which may move. Show the full affected region in the preview.

Learn here: a graph junction is shared topology, not two coincident but
independent endpoint coordinates. A drag policy should be expressible without
Bevy mouse events so it can be tested at the domain boundary.

**Check:** for a simple chain and a multi-wall junction, the expected result
of each handle drag is unambiguous before implementation.

### 8.2 Build an atomic domain mutation

Add a validated operation in `wall_graph` or another engine-independent domain
layer that proposes the entire changed layout. It must preserve unaffected
identifiers and attached openings where the policy allows. Validate all graph
rules before changing committed state, then validate the mesh through the
editor's candidate path. Do not commit removal and replacement as separate
actions.

Learn here: public mutation boundaries must be atomic: a failed operation
cannot consume IDs or leave half of a connected edit behind. Test both the
returned error and equality of geometry and identifiers after rejection.

**Check:** a failed drag leaves walls, nodes, openings, and IDs unchanged.
A successful drag keeps the intended openings attached and valid.

### 8.3 Preserve inferred tangency where possible

Infer smooth connections from the current geometry using a consistent
tolerance, without storing tangent flags. As a junction moves, adjust nearby
arcs to retain those tangencies when a valid solution exists. If not, keep the
dragged endpoints fixed and choose a valid curve that minimizes change from
the original arc. Mark the lost tangency visibly before commit.

Learn here: the fallback is a geometry choice that belongs in domain logic;
the UI should present the result, not invent a second arc algorithm. Keep
invalid fallback geometry as an error rather than forcing a mesh.

**Check:** a feasible drag stays tangent. An infeasible one previews the
best-fit valid connection and clearly identifies the lost tangent join.

### 8.4 Wire handles through preview and history

Activate straight endpoint, curved bend, junction, and whole-wall handles as
appropriate. Pointer movement updates one draft operation. Escape restores
the original view; release or confirmation creates one history entry. Refresh
selection and hit data after commit, clearing any invalid wall handle.

Learn here: the tool can keep its drag input in Bevy state while asking the
domain operation for a candidate document. This maintains the same commit
boundary used since step 2.

**Check:** a full drag is one undo step, and undo/redo restore all connected
geometry and openings. Rejected drags leave no history entry.

### Step 8 is complete when

- [ ] Whole-wall, endpoint, bend, and junction drags have explicit behavior.
- [ ] Connected walls and openings preview and commit atomically.
- [ ] Tangency is retained or visibly lost according to the domain result.
- [ ] Failed drags preserve identifiers, geometry, history, and saved state.
- [ ] Focused domain tests, relevant mesh tests, `cargo check -p house_editor`,
      and manual drag/undo scenarios pass.

## Useful project references

- [Wall graph architecture and invariants](../crates/wall_graph/docs/architecture.md).
- [Wall graph geometry tolerances](../crates/wall_graph/docs/geometry.md).

Next: [step 9, later editor breadth and performance](editor-step-9.md).
