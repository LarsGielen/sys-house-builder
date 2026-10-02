# Editor step 0: a separate app and an empty scene

This expands step 0 of [the implementation order](editor-implementation-order.md).
Work through one small checkpoint at a time. The prompts describe behavior and
point to useful Bevy concepts, while leaving the code and layout choices to
you. Keep the existing `house_builder` mesh inspector intact.

## Conventions to choose before coding

- Use the Bevy version already in this workspace (`0.19.1`). A new editor
  crate needs a manifest, a workspace entry, and references to the existing
  Bevy and `wall_graph` crates. Do not upgrade libraries or introduce new ones;
  `AGENTS.md` requires asking before dependency changes.
- Use one world unit as one metre. Bevy's vertical axis is Y; the ground is an
  XZ plane. `wall_graph` uses XY for the floor plan and Z for elevation. Write
  down the mapping before walls are introduced. Mapping graph `(x, y, z)` to
  Bevy `(x, z, -y)` keeps a right-handed orientation.
- Treat the scene as a view of the house. The `WallGraph` will own wall data;
  camera, ground plane, and visual guides are editor entities.

## Tasks

### 0.1 Create the app target (Done)

Create a small binary crate for the editor (the order document proposes
`crates/house_editor`) and add it to the workspace. Start with one entry point.
Use Bevy's normal app setup, then run it before adding any scene objects.

Learn here: what `App`, `DefaultPlugins`, and `Startup` each provide. A startup
system runs once; an update system runs during the app loop. You do not need a
custom plugin or a module tree yet.

**Check:** launching the editor opens its own window. Launching the existing
inspector still works independently. The editor contains no copied demo graph,
toolbar, or rendering code.

### 0.2 Make the empty scene legible (done)

Spawn one 3D camera looking toward the origin, a horizontal ground plane, and
enough lighting or an unlit material to see the plane. Give the editor window a
recognizable title. Keep the scene deliberately plain; a wall mesh, shadows,
sky, and materials are not needed yet.

Learn here: `Commands` creates entities; components such as `Camera3d` and
`Transform` describe them; mesh and material assets are stored separately from
the entities that reference them. Try changing the camera position and plane
size yourself to see which component owns each behavior.

**Check:** the plane is visible, the camera sees its origin, and the scene is
clearly empty. Resizing the window does not break the view.

### 0.3 Establish an empty house resource

Make an empty `WallGraph` available to editor systems, but do not generate a
mesh for it yet. Because `WallGraph` comes from another crate, a small local
wrapper is the natural way to make it a Bevy `Resource`. This also makes the
boundary between house data and editor state visible while learning ECS.

Learn here: an entity component belongs to one entity; a resource is one value
shared by systems. Avoid treating the ground-plane entity as the house model.

**Check:** the app still starts with zero walls. A temporary observation in a
startup or update system can confirm the graph is empty; remove that probe
when it is no longer useful.

### Step 0 is complete when

- [ ] The new editor runs from an empty house and displays a ground plane.
- [ ] The inspector remains a separate, working app.
- [ ] You can explain which startup system creates the camera and ground, and
      where the empty graph is stored.
- [ ] `cargo check -p house_editor` and a manual `cargo run -p house_editor`
      succeed (adjust the package name if you chose another one).

## Useful Bevy references

These are references for the concepts above, not code to copy into the editor:

- [Bevy app and plugins](https://bevy.org/learn/quick-start/getting-started/apps/)
  and [startup systems and components](https://bevy.org/learn/quick-start/getting-started/ecs/).
- [Resources](https://bevy.org/learn/quick-start/getting-started/resources/)
  for the empty house resource and other shared state.

Next: [step 1, camera and viewport controls](editor-step-1.md).
