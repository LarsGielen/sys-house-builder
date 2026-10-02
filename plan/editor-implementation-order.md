# Editor implementation order (draft)

This document proposes an implementation sequence for the capabilities in
[editor-features.md](editor-features.md). The phases are dependencies, not
deadlines. Each phase should leave a usable, reviewable part of the editor.
Build the editor as a fresh Bevy app with an empty house. The existing
`house_builder` demo is a mesh inspector, not an editor starting point; leave
it intact and do not copy its app structure. Reuse the engine-independent
`wall_graph` and `wall_mesh` crates as their capabilities are needed.
Use the separate guides for [step 0](editor-step-0.md),
[step 1](editor-step-1.md), [step 2](editor-step-2.md),
[step 3](editor-step-3.md), [step 4](editor-step-4.md),
[step 5](editor-step-5.md), [step 6](editor-step-6.md),
[step 7](editor-step-7.md), [step 8](editor-step-8.md), and
[step 9](editor-step-9.md) while building the editor.

## System boundaries

These are logical responsibilities, not a proposal to create a crate or file
for each one.

| System | Responsibility | Likely home |
| --- | --- | --- |
| House model | Wall geometry, topology, dimensions, openings, and atomic validation | `crates/wall_graph` |
| Mesh generation | Turn a valid graph into a closed, tagged mesh; report mesh-specific errors | `crates/wall_mesh` |
| Editor document | Current house plus editor-owned data such as door/window categories; later furniture and floors | Add engine-independent data when the graph alone is insufficient |
| Edit coordination | Active tool, selection, draft change, commit, undo/redo, and dirty state | New editor app (proposed `crates/house_editor`), with reusable domain operations kept outside Bevy |
| Viewport input | Pointer ray, camera controls, ground-plane hit, and UI input exclusion | New editor app |
| Picking and snapping | Hit targets, edit handles, snap candidates, and visual feedback | New editor app, querying `wall_graph` geometry |
| Scene rendering | Convert tagged wall meshes to Bevy entities, refresh them, and draw previews/highlights | New editor app; build its adapter as the need arises |
| Properties UI | Tool controls, exact numeric input, categories, errors, grid settings | New editor app |
| Project I/O | Versioned data format, whole-file validation, safe save/open | Engine-independent format and file boundary; Bevy only triggers the action |

The system names above describe responsibilities, not an up-front module
layout. Keep the initial app small and introduce boundaries as the behavior
requires them. New dependencies need approval under `AGENTS.md`.

For interactive edits, the Bevy-side flow is: collect pointer/UI input, compute
the hovered target and snap candidate, update a draft, validate its graph and
mesh, render the preview, then commit only on confirmation. A commit refreshes
the scene and picking data and records one undo step. This is a data-flow guide,
not a requirement for one ECS system per step.

## 0. New Bevy app from an empty scene

**Features:** create a separate editor app target, open a window, and render
an empty ground plane with a camera. Start with an empty `WallGraph`; add the
graph-to-world coordinate conversion when the first wall is rendered.

**Touches:** new editor app and workspace configuration; Bevy startup setup.
The existing demo remains untouched. This phase is intentionally small so
Bevy app setup, resources, components, and systems can be learned in context.

**Checkpoint:** the new app starts on an empty scene. No demo house or demo
toolbar appears.

## 1. Editor shell and camera

**Features:** orbiting 3D viewport; middle-drag orbit, right-drag pan, wheel
zoom; ground plane and scale cues; frame house/selection; left button reserved
for tools; viewport input ignored over UI.

**Touches:** viewport input, camera and UI systems in the new editor app.

**Checkpoint:** the empty scene can be inspected without any left-button
camera action or accidental viewport action while using the UI.

## 2. Shared edit and preview path, then straight walls

**Features:** separate draft from committed house; validate graph changes and
mesh generation before commit; show invalid geometry and reasons; Escape
cancels; refresh rendered mesh and picking data after commit. Draw a straight
wall by either click-click or press-drag-release, using a movement threshold.
Preview length, height, and thickness. Snap to eligible corners/junctions,
then walls, then grid; show the chosen target, support temporary snap disable
and user-configurable grid spacing. Show lengths and metres consistently.

**Touches:** edit coordination and scene rendering in the new editor app;
`wall_graph::add_wall_with_dimensions`; `wall_mesh::generate`; viewport input,
snapping, and properties UI. Keep the same proposal/validation/commit path for
later tools. Build the first graph-to-Bevy mesh adapter here, without copying
the inspector's rendering structure. Correctness comes before incremental
mesh updates.

**Checkpoint:** connected straight walls can be built from an empty graph.
Invalid proposals leave the graph unchanged and explain the failure. The
displayed mesh matches the committed graph after every edit.

## 3. Picking, selection, and edit handles

**Features:** wall hover and click selection, clear selection, highlight and
outline, properties for the selected object, and visible endpoint/junction
handles. Add opening picking across the hole and opening handles when openings
arrive in phase 4. Clear selection when entering a wall-building tool and after
any edit that invalidates the selected wall handle.

**Touches:** `wall_mesh` surface tags as picking identities; `wall_graph` queries
for exact wall/opening positions; picking, highlights, handles, and properties
UI in the new editor app. Use `SurfaceSource` tags from generated meshes. An
opening needs a separate editor hit target because the center of its hole has
no triangles.

**Checkpoint:** walls drawn in phase 2 can be selected and highlighted. A
stale wall handle never highlights a different wall after a mesh rebuild.

## 4. Openings and basic property edits

**Features:** wall-host picking for an opening; 3D preview with width, height,
bottom elevation, and wall position; door/window categories; opening placement,
dragging, resizing, deletion, and duplication; snapping to centers and sides
of other openings with an offset option. Exact numeric editing for opening
properties and wall height/thickness. Delete selected walls with clear feedback
about their openings. Show the corresponding edit handles.

**Touches:** `wall_graph` opening and dimension APIs; `wall_mesh` regeneration;
editor document for the door/window category; new editor app picking, tool
state, preview, handles, and properties UI. Existing graph insertion already
transfers openings across splits and merges; removing a wall removes its
openings. The UI should make that removal visible before commit.

**Checkpoint:** a small layout with windows and doors can be built and edited.
The category is retained even though both kinds use the same geometric hole.

## 5. Undo/redo and document state

**Features:** undo/redo for committed edits; modified indicator and unsaved
changes handling. One completed drag or placement is one undo step; moving a
preview does not fill the history.

**Touches:** edit coordination, editor document, and new editor app status
UI. Restore model and editor metadata together, then rebuild the scene and
picking data. Keep history independent of Bevy entity IDs.

**Checkpoint:** placement, edits, deletion, and opening categories undo and
redo as complete operations. A project visibly shows when it has changed.

## 6. Project files

**Features:** versioned human-inspectable file with documented metres and
coordinates; export/save, import/open, whole-file validation, safe replacement
of an existing file, and a save/reopen check. Handle unsaved changes before
opening another project.

**Touches:** editor document; engine-independent project format and file I/O;
`wall_graph` reconstruction/validation; `wall_mesh` validation after load;
new editor app file actions and status UI. Store resolved geometry, openings,
and editor metadata. Define a snapshot/reconstruction path rather than
serializing private graph internals. Raw in-memory wall handles may change on
reload, so saved door/window categories need reliable opening references.
Tangency is inferred from geometry rather than stored as a flag.

**Checkpoint:** walls, arcs, dimensions, openings, and categories survive a
round trip. A rejected import leaves the current project intact, and a failed
save leaves the previous file intact.

## 7. Curved-wall tools and tangent placement

**Features:** free arcs with a third pointer position and exact radius/angle
entry; a visible bend handle; one- or two-connection tangent modes that derive
the curve without a bend input; clear preview errors when the requested circle
does not exist. The click/drag endpoint gesture still applies; a free arc needs
its bend step before confirmation.

**Touches:** curve construction and validation at the engine-independent
boundary; `wall_graph::add_arc_with_dimensions` and tangent queries; possibly
`wall_graph` topology validation; `wall_mesh` validation; new editor app arc
tool, handles, snapping, properties UI, and preview.

**Graph prerequisite:** test the intended smooth joins first. The graph allows
some smooth end-to-end continuations but rejects tangential contacts and
indistinguishable outgoing directions. If an intended join is rejected, change
the domain rule deliberately while retaining rejection of ambiguous or
overlapping geometry. Do not make the editor silently offset a tangent wall to
get it accepted.

**Checkpoint:** free and tangent arcs build the expected graph and mesh, and
impossible tangent requests show an invalid preview without changing the house.

## 8. Wall and junction manipulation

**Features:** move a wall or drag its endpoints/junctions and show all affected
connected walls and openings; infer existing tangencies and keep them where possible. If
exact tangency is impossible, keep dragged endpoints fixed and minimize change
from the original arc, visibly indicating lost tangency. Reject any result the
graph or mesh cannot accept. Support straight and curved wall handles, and
keep opening behavior explicit during host-wall moves.

**Touches:** new atomic mutation operation(s) in `wall_graph` or another
engine-independent domain layer; opening transfer and validation; `wall_mesh`;
new editor app handles, drag input, preview, selection refresh, and undo/redo.
Removing a wall and adding a replacement as separate committed actions would
break atomicity and can lose openings, so this needs one validated transaction.

**Checkpoint:** successful drags preserve a valid house and its attached
openings; failed drags leave identifiers and geometry unchanged. The preview
shows when a former tangent connection becomes a best-fit connection.

## 9. Later editor breadth and performance

**Features:** hide/cut away obscuring walls for interior editing; consider
multi-selection; add visible door/window objects;
extend the document for furniture, stairs, and upper floors when those domain
rules exist. Measure mesh rebuilding and picking on realistic houses, then
improve only the slow paths.

**Touches:** scene rendering, picking, and UI in the new editor app; editor
document and engine-independent domain logic for new object types; mesh
generation only when the physical geometry requires it. Cutaway is a rendering
concern and should not remove walls from the house model.

**Checkpoint:** each extension uses the same selection, preview, validation,
undo/redo, and persistence paths rather than a separate editing workflow.
