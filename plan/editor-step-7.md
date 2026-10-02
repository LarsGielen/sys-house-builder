# Editor step 7: curved walls and tangent placement

This expands step 7 of [the implementation order](editor-implementation-order.md).
Start after [step 6](editor-step-6.md). Use the same snapping, candidate graph,
mesh validation, commit, undo, and save paths as straight walls.

## Tasks

### 7.1 Check the domain's intended tangent joins

Before adding UI modes, write focused graph cases for the desired smooth joins:
one connected wall, two connected walls, and joins that must remain invalid.
The graph permits some smooth end-to-end continuations but rejects tangential
contacts and indistinguishable outgoing directions. If an intended result is
rejected, change the domain rule deliberately while preserving rejection of
ambiguous or overlapping geometry.

Learn here: the graph is the invariant boundary. Do not have the editor offset
an intended tangent endpoint to get an otherwise invalid join accepted.

**Check:** the accepted and rejected tangent cases are explicit in domain tests
before a tangent tool relies on them.

### 7.2 Construct a free arc from endpoints and bend

Reuse the straight-wall endpoint gesture: click-click or drag-release picks
endpoints, then a third pointer position chooses the bend before confirmation.
Show a bend handle and a live curve preview. Offer exact radius or angle entry
that controls the same draft. Convert the resulting curve to the inputs for
`add_arc_with_dimensions` and validate the candidate graph and mesh.

Learn here: circular arcs need a direction and sweep as well as endpoints.
Use the graph's geometry conventions for sweep and tolerances; reject nearly
collinear or otherwise undefined bend positions with a useful reason.

**Check:** changing the bend changes the expected side and size of the arc;
Escape cancels at either endpoint or bend stage.

### 7.3 Add one-connection tangent mode

Given the chosen endpoint and an existing wall's connection tangent, derive a
circular arc through the other endpoint without asking for a bend position.
Show the tangent target and the derived radius/angle. If the geometry has no
valid solution, display an invalid preview while keeping the house unchanged.

Learn here: isolate curve construction in engine-independent logic. The editor
collects pointer positions and target identities; the domain calculation
decides whether a circle exists under its tolerances.

**Check:** a feasible arc meets the host smoothly, while degenerate endpoint
choices explain why no tangent arc can be committed.

### 7.4 Add two-connection tangent mode

Use both connection positions and tangents to derive a circle when possible.
Make ambiguous, inconsistent, and overlapping solutions explicit errors.
Keep snapping and exact input behavior consistent with other wall tools.
Preview the full graph and mesh, including any splits or merges at the joins.

Learn here: two tangent constraints may overdetermine a circle. A failed solve
is a normal `Result`, not a reason to silently switch to a free arc.

**Check:** valid two-connection arcs join the expected walls. Impossible
requests show an error and leave the document, undo history, and saved state
unchanged.

### Step 7 is complete when

- [ ] Free arcs work with a bend pointer and exact radius/angle input.
- [ ] One- and two-connection tangent modes derive valid arcs where possible.
- [ ] Domain rules for accepted and rejected smooth joins are tested.
- [ ] Invalid graph, mesh, or tangent solutions cannot commit.
- [ ] Relevant `wall_graph` and `wall_mesh` tests, `cargo check -p house_editor`,
      and manual free/tangent arc scenarios pass.

## Useful project references

- [Wall graph geometry conventions](../crates/wall_graph/docs/geometry.md).
- [Wall graph arc API](../crates/wall_graph/README.md).

Next: [step 8, wall and junction manipulation](editor-step-8.md).
