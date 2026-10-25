# House editor: reusable boundaries and action-driven input

Review date: 2026-10-24. Status: proposal only; no implementation changes.

## Recommendation

Establish a shared action entry point for every editor operation, extract the
validated document/editing layer as the first new crate, and then separate
reusable interaction logic from Bevy systems and presentation. Keep the existing
geometry crates independent of Bevy.

The useful boundaries are:

1. **Document and validated edits**: house state, opening categories, proposals,
   candidates, history, dirty tracking, and project serialization.
2. **Editor interaction**: actions, selection, drafts, picking, snapping, property
   edits, and project replacement workflow, independent of widgets and devices.
3. **Bevy integration**: scheduling, camera/window adapters, scene entities, UI,
   and configurable input bindings.

Start with ordinary Rust modules where a boundary is still moving. The first
strong new library-crate candidate is `house_document`; the second is
`house_editor_core` once its inputs and outputs are explicit. Make the existing
`house_editor` package a library plus a thin binary so its Bevy integration can
also be reused. Avoid a crate per tool or per small utility.

The input requirement is architectural: a wall tool must receive an action such
as a tool press, update, release, or cancel. It must never decide that
`MouseButton::Left` means placing a wall. UI controls and shortcuts must invoke
the same actions, with the same validation and outcomes.

## Scope and current strengths

Reviewed all 34 production Rust files, all 30 files under `src/tests/`, and the
crate manifest: 7,734 production lines, 5,513 test/support lines, and 215 test
functions. Also checked the workspace manifests, existing implementation plans,
the geometry crates' public boundaries, and the demo's mesh adapter. Source
references below are relative to `crates/house_editor/src/` unless otherwise stated.

The workspace already contains `wall_graph`, `wall_mesh`, and the `house_builder`
demo. The document/editor application layer is what remains concentrated in
`house_editor`; the geometry and mesh algorithms are already reusable.

Several foundations should be preserved:

- `EditCandidate::build` constructs graph, mesh, and opening metadata together.
  A graph-valid edit can still fail mesh generation; commit must remain after
  both checks (`edit.rs:188`).
- Proposals operate on cloned graphs. Failed operations leave the committed
  document unchanged.
- `EditPreview::take_valid` requires the expected proposal and house revision.
  Successful interactive edits take the actual previewed candidate rather than
  independently rebuilding it (`edit.rs:454`).
- `House` owns undo/redo and uses content IDs for dirty tracking. Undoing back to
  the saved state works even after branching history (`house.rs:17`).
- `ActivateTool` already separates a UI trigger from tool activation
  (`wall_tool_active.rs:24`). Other request messages provide useful partial seams.
- `project_file` and `project_io` already separate text format from filesystem
  access. Imports validate a fresh document before replacing the current one.
- Existing tests cover atomicity, metadata preservation, exact preview commits,
  gestures, history, persistence, and important frame-ordering constraints.

This report is based on source inspection. No tests, build, or interactive
reproduction were run for this documentation-only task. The behavior issues
below are source-derived findings and need regression tests during implementation.

## Input and mutation audit

| Area | Current behavior | Required action boundary |
| --- | --- | --- |
| Active tool | UI emits `ActivateTool`; handler changes `ActiveTool` | Retain this pattern; route activation through the common action dispatcher and cancel the previous gesture deliberately. |
| Straight walls | `DrawingInput` and `advance_draft` read left press/release and Escape directly (`wall_tool_drawing.rs:115`, `:183`) | Tool press/update/release/cancel actions with pointer data; retain click-click and drag-release behavior. |
| Arcs | `ArcInput` reads left button and Escape; tool also draws guides (`wall_tool_arc.rs:108`, `:193`, `:373`) | Same tool action lifecycle, explicit arc-mode action, and separate guide presentation. |
| Opening placement | Left click commits; Shift disables snapping (`wall_tool_opening.rs:84`, `:126`) | Pointer-update and tool-press actions plus semantic snapping state. |
| Delete tool | Left click commits the removal under the pointer (`wall_tool_delete.rs:37`) | Tool update/press actions; preserve hover preview as the existing confirmation behavior. |
| Selection | Left click selects; Escape clears; handle priority is coordinated across modules (`wall_tool_selection.rs:134`) | Tool press and explicit selection/cancel actions resolved by one owner. |
| Wall handles | Reads left press/release, Escape, and Shift; commits and carries selection to rebuilt walls (`wall_tool_handles.rs:158`) | Begin/update/end/cancel gesture actions; snapping comes from state, never a key lookup. |
| Opening manipulation | Drag reads mouse/Escape/Shift; request handler also checks Delete (`wall_tool_edit_opening.rs:85`, `:274`) | Gesture actions and delete/duplicate actions through the same mutation path. |
| Selected wall deletion | Messages coexist with direct Delete, Enter, and Escape checks (`wall_tool_edit_wall.rs:88`, `:110`) | Request deletion, confirm pending deletion, and cancel pending edit actions. |
| Wall/opening properties | Widgets parse input, update previews, and call `house.commit` (`wall_fields.rs:93`, `:130`; `opening_fields.rs:118`) | Widgets emit property preview/submit/cancel actions carrying target and value. Handlers own validation and commit. |
| Arc properties | Widgets mutate `ArcDraft`, preview, and commit (`arc_fields.rs:79`, `:101`, `:155`) | Arc angle/radius preview/submit/reset-override actions handled with the arc state machine. |
| Tool settings | UI directly mutates wall dimensions, arc mode, and opening category (`wall_ui.rs:99`, `:157`; `opening_ui.rs:129`) | Set/step drawing dimensions, set arc mode, and set opening category actions. |
| Snap settings | UI directly steps grid spacing and sets gap; `snapping_disabled` reads Shift in multiple consumers (`editor_ui.rs:485`; `opening_fields.rs:192`; `snapping.rs:140`) | Set/step grid spacing, set opening gap, and set temporary snap suppression actions. Readouts observe suppression state. |
| History | Buttons emit requests, but history handler also recognizes Ctrl+Z/Ctrl+Shift+Z/Ctrl+Y (`history.rs:36`) | Binding adapter emits undo/redo; history handler accepts only actions. |
| Project workflow | Messages coexist with Ctrl+S/Ctrl+O; handler queries path widgets (`project.rs:78`, `:171`) | Explicit open/save actions with captured paths; unsaved decisions are actions too. |
| Window close | Window event becomes `ProjectRequest::Close` (`project.rs:162`) | Retain an OS-event adapter, emitting the same close action as any other source. |
| Camera | Right/middle buttons, WASD, Home, motion, and wheel are embedded in the controller (`orbit_camera.rs:13`, `:101`, `:180`, `:269`, `:295`) | Camera begin/end, orbit delta, pan delta/axis, zoom, and reset actions. Cursor capture stays in the platform adapter. |
| Text focus | Left click blurs; Escape blurs and mutates preview (`text_fields.rs:145`, `:162`) | Widget/focus adapter emits focus and field-cancel actions; document preview cancellation belongs to interaction logic. |
| Panel scrolling | UI directly reads wheel accumulation (`editor_ui.rs:199`) | Input adapter routes wheel to a panel-scroll action when the panel owns it, otherwise to camera zoom. |
| Ground pointer | Reads the window cursor, UI interactions, and grab mode (`ground_pointer.rs:75`) | Platform adapter supplies pointer/window state; projection produces a current-frame pointer sample for semantic actions. Marker drawing is separate. |

### Concrete issues to address while establishing these boundaries

**1. A pending Open does not remember which path was requested.**
`PendingReplacement` stores only `ProjectRequest`, and both saving and opening
call `target_path()` against the live `EditableText` query. Opening B while A is
modified can therefore save A's contents over B when the user chooses Save. If
the field is changed to A for the save, the subsequent Open also reads A instead
of B (`project.rs:92`, `:103`, `:138`, `:204`). The existing
`saving_first_writes_the_old_project_then_opens_the_other` test changes the field
before answering, but never asserts that the other document was opened
(`tests/project.rs:258`).

Capture the open destination in the action and pending operation. Resolve the
save destination independently from the current project path or an explicit
Save As action. An unnamed project needs a save-path request before continuing.
Do not re-read widgets after an unsaved decision. This is a correctness change
with data implications, so review that behavior explicitly in its implementation
step rather than hiding it inside a mechanical module move.

**2. Submitting an impossible arc radius can commit an older preview.**
When `sweep_for_radius` rejects a numeric radius, live preview sets a notice and
returns without invalidating the previous candidate (`arc_fields.rs:123`).
Submission only checks that the text parses as a finite number and then takes
whatever arc is currently previewed (`arc_fields.rs:169`). For a four-metre
chord, entering radius `1` and submitting can therefore commit the old arc.
The existing test checks the radius error before submission, not after Enter.

A field submit action must validate its own current value and bind confirmation
to that specific draft/value. Keeping an older preview visible is fine; treating
it as confirmation of rejected input is not.

**3. Shortcut and camera inputs can overlap.**
The camera's WASD pan checks do not exclude Ctrl, while project handling uses
Ctrl+S (`orbit_camera.rs:269`; `project.rs:179`). When viewport keyboard input is
otherwise allowed, saving can also pan the camera. Binding resolution needs
explicit modifier matching and context priority.

**4. Frame order currently substitutes for explicit ownership.**
Several systems independently read the same Escape, Delete, Enter, or press.
Some check `TextFocus`, while straight-wall cancellation does not. The arc field
relies on Escape being observed after text focus is cleared. Tool commands with
readers behind `run_if(tool_active(...))` can also remain unread while inactive;
the router must reject or consume inapplicable actions immediately rather than
allowing them to affect a later tool/selection.

**5. A tolerance comment does not match the graph.**
`project_file.rs:31` says its `1e-3` position tolerance matches the graph, but
`crates/wall_graph/src/geometry.rs:6` defines distance tolerance as `1e-4`.
Decide whether host matching intentionally needs a different tolerance; document
and test the choice. Do not globally unify screen-pixel acquisition radii,
occlusion tolerance, mesh sampling deviation, and geometric equality: they serve
different purposes.

## Proposed ownership and dependency direction

```text
wall_graph                      wall_mesh -> wall_graph
       \                         /
        +---- house_document ---+
                    ^
                    |
             house_editor_core
                    ^
                    |
          house_editor library (Bevy adapters)
                    ^
                    |
          house_editor binary (app composition)
```

Here an arrow points toward a dependency. `house_editor_core` can also depend
directly on `wall_graph`/`wall_mesh` for query types. Filesystem functions are a
separate `project_io` module, initially in `house_document`; the interaction
controller returns project effects for an outer adapter to execute. It does not
open files or exit the application itself.

### `house_document`: strongest extraction candidate

Own the committed candidate, categories, validated proposals, history, revisions,
saved-content identity, project format, and isolated filesystem helpers.

- Split `edit.rs` into proposals/evaluation/candidate code and interaction preview
  bookkeeping. Move the former here, the latter to `house_editor_core`.
- Move `House` without the Bevy `Resource` derive. History is already document
  logic inside it; the shortcut system in `history.rs` is an adapter, not part of
  this crate.
- Keep graph and generated mesh as one accepted candidate. Engine-independent
  does not mean mesh-independent: current acceptance deliberately includes mesh
  feasibility.
- Keep `OpeningCategory` here rather than adding door/window semantics to
  `wall_graph`.
- Replace Bevy vector imports with the same `glam` version used by the existing
  geometry crates. This changes imports/dependency declarations, not coordinates.
- Preserve the version-1 file format and reconstruction semantics. Re-export only
  the types consumers need; parser helpers and candidate construction internals
  stay private or crate-visible.
- Move `EditCandidate::build` and its project-import caller together so its
  existing `pub(crate)` visibility does not need to become a public unchecked API.

Initially keep the bounds cache and whole-document snapshots. Relocating the
cache or replacing snapshots is a separate performance decision. The fixed
`mesh_settings()` seam should become an explicit shared validation policy if
mesh settings become configurable; candidate generation and opening bounds must
always use the same policy, and policy changes must invalidate previews/caches.

### `house_editor_core`: reusable interaction, not a generic editor framework

Own action data, action dispatch, active tool, selection, gesture drafts,
preview state, snap settings/algorithms, hit queries, and property-edit behavior.
Also own the pure pending-project workflow and its requested effects.

- Extract pure ray/triangle and opening-area picking from selection systems.
  Use graph-space origins/directions and results, with no `Camera`, `Ray3d`,
  `Gizmos`, or ECS query in the API.
- Keep screen-distance snapping reusable by retaining its projection callback or
  supplying projected candidate positions. The existing `choose_snap_target`
  already accepts a projection closure; it does not need a Bevy camera.
- Move opening snapping, opening area/path queries, and free-end tangent queries
  here. They are editing/query policy rather than graph mutation invariants.
- Refactor gesture advancement to ordinary functions/methods taking semantic
  input, settings, and document state. Keep separate straight-wall, arc,
  wall-handle, and opening-drag state machines; share only the repeated threshold
  and gesture-lifecycle policy.
- Move field meaning and proposal construction here. UI code keeps parsing and
  formatting concerns, but finite/range/representability checks are repeated at
  the action boundary because callers need not be widgets.
- Centralize commit effects: history entry, preview cleanup, notice clearing,
  revision changes, selection remapping, and gesture invalidation. Reuse
  `ReshapeReport` to keep selections on rebuilt walls.
- Keep rendering choices such as hiding committed geometry during a deletion
  preview in the presentation adapter. The core exposes proposal kind, outcome,
  and reshape effects.

The core should be usable from a plain Rust test without initializing a Bevy
`App`, assets, a window, or text widgets. A single small session/controller can
coordinate focused modules; no dynamic tool registry, trait object hierarchy,
global event bus, or plugin framework is justified yet.

### `house_editor`: reusable Bevy library and app shell

Keep `input`, `ui`, `viewport`, `render`, and `integration` modules here. A library
entry point can expose a composing editor plugin plus its action transport and
document/session resources. The binary only assembles plugins, window defaults,
and the initial scene.

The adapter can use local `Resource` newtypes around pure document/session types
and a local Bevy `Message` envelope around core actions. Rust's orphan rules mean
the adapter cannot simply implement Bevy's external `Resource` trait for a type
owned by a different crate. Avoid adding Bevy derives to the pure core to work
around this.

Camera state/pose control is a good reusable Bevy module once its raw input and
UI ownership checks move out. Ground picking, coordinate conversion, and mesh
conversion are also reusable Bevy modules. They do not each need a crate now.
Use explicit camera/window identity in adapter configuration when removing
current `Single<...>` assumptions; retain a simple single-viewport default.

There is a real second consumer for coordinate/mesh conversion in
`crates/house_builder/src/wall_render.rs`. A later `wall_mesh_bevy` adapter crate
is reasonable if both applications adopt it. Share conversion and triangle/tag
handling, while keeping the demo's colored inspector and editor's previews as
separate presentation policies. Updating the demo is optional follow-up scope,
not a prerequisite for this refactor.

## Complete source disposition

These are ownership destinations, not a demand to create a separate crate or
file for every row. Existing tests should move with the behavior they exercise.

| Existing file(s) | Proposed home and split |
| --- | --- |
| `main.rs` | Binary composition only; move plugin assembly into the Bevy library. Scene defaults remain app configuration. |
| `editor_schedule.rs` | Bevy `integration/schedule`; compose stable phase sets instead of importing every leaf tool's schedule into shared logic. |
| `edit.rs` | `house_document::{proposal, candidate, edit}` plus `house_editor_core::{preview, notice}`; separate presentation helpers. |
| `house.rs` | `house_document::document`; preserve history/content IDs and bounds-cache invalidation. |
| `history.rs` | Core undo/redo action handling and restore effects; Bevy transport in integration; shortcuts in input. |
| `opening_category.rs` | `house_document::opening_category`. |
| `project_file.rs` | `house_document::project_file`; retain parser, writer, import validation, and file-local IDs. |
| `project_io.rs` | `house_document::project_io` initially; keep disk access callable independently of the UI. Separate crate only if a real consumer needs format without filesystem code. |
| `unsaved_changes.rs` | Core project workflow; use document dirty state, independent of UI. |
| `project.rs` | Core pending-operation state machine; filesystem/window-exit effects in integration; `PathField` belongs to UI; keyboard shortcuts belong to input. |
| `project_ui.rs` | Bevy `ui/project`; action producers and readouts only. |
| `coordinates.rs` | Bevy `viewport/coordinates`; one shared rotation for positions, normals, rays, and guides. |
| `ground_pointer.rs` | Bevy input/projection adapter plus separate render marker; core owns neutral pointer data types. |
| `orbit_camera.rs` | Bevy `viewport/orbit_camera` controller; raw bindings and cursor capture in input/platform adapter. |
| `snapping.rs` | Core snap settings, targets, grid, selection algorithm; camera projection adapter in viewport; marker in render; Shift binding in input. |
| `opening_snap.rs` | Core `snapping/opening`; accept explicit options including snap radius. |
| `opening_area.rs` | Core `picking/opening_area` and wall-path helpers; keep the curved-opening chord approximation documented. |
| `wall_tool_active.rs` | Core active-tool state and activation action; Bevy message/resource wrapper in integration. |
| `wall_tool_drawing.rs` | Core straight-wall state machine/settings; input adapter emits semantic gesture actions. |
| `wall_tool_arc.rs` | Core arc state machine/mode/tangent queries; guides in render. |
| `wall_tool_opening.rs` | Core opening placement/settings/proposal construction; ray conversion stays in viewport. |
| `wall_tool_delete.rs` | Core deletion tool, using shared picking/proposal/commit paths. |
| `wall_tool_selection.rs` | Core selection and geometry picking; highlight/outline/handle drawing in render. |
| `wall_tool_handles.rs` | Core handle queries, drag state, and proposals; handle drawing in render. |
| `wall_tool_edit_opening.rs` | Core opening drag/delete/duplicate actions; shortcut recognition in input. |
| `wall_tool_edit_wall.rs` | Core selected-wall deletion workflow and draft invalidation; keyboard recognition in input. |
| `text_fields.rs` | Bevy `ui/fields` widgets/focus adapter; remove document-preview mutation from generic field helpers. |
| `opening_fields.rs` | UI fields/formatting emit core opening-property and gap actions; handlers validate and mutate. |
| `wall_fields.rs` | UI fields/formatting emit core wall-property preview/submit actions; handlers validate and commit. |
| `arc_fields.rs` | UI fields/formatting emit core angle/radius/override actions; arc controller owns draft updates and confirmation. |
| `editor_ui.rs` | Bevy `ui/panel`, tool/history/snap controls and readouts; bindings go in input; settings mutations go through actions. |
| `wall_ui.rs` | Bevy wall/arc controls and readouts; emit dimension, arc-mode, and deletion actions. |
| `opening_ui.rs` | Bevy opening controls and readouts; emit category, duplicate, and deletion actions. |
| `wall_render.rs` | Bevy `render/{mesh, preview, removal, guides}` as needed; expose a small mesh adapter, keep editor visual policy local. |
| `Cargo.toml` | Retain Bevy integration; add internal library dependencies only in approved implementation work. No external input framework is needed. |

## Action contract

### Shared path

```text
OS/device events -> configurable binding/context resolver --+
UI widgets ------> action producers ------------------------+-> action dispatcher
Tests/other hosts -> direct actions ------------------------+          |
                                                        validate/advance state
                                                                  |
                                                  document + preview + effects
                                                                  |
                                                      Bevy scene/UI adapters
```

Use ordinary enums and typed payloads. One ordered action stream at the adapter
boundary is preferable to many independent message readers that lose ordering
between action types. The dispatcher can call focused handlers by action family;
it does not need to become a large ECS system with every resource/query.
Existing messages can temporarily translate into the common stream, but remove
their direct keyboard paths as each family migrates.

Suggested action families, with names illustrative rather than final public APIs:

| Family | Required operations and payloads |
| --- | --- |
| Tool/session | Activate tool; tool press/update/release; cancel gesture; select target/clear selection. Carry a pointer sample or explicit target, plus gesture identity where relevant. |
| Editing | Preview proposal; confirm a specific preview; request/confirm wall deletion; delete opening; duplicate opening with explicit target/gap. Programmatic callers can submit explicit proposals without simulating clicks. |
| Properties | Preview/submit wall dimension or opening property; preview/submit arc angle/radius; cancel field draft; clear typed arc override. Include target, property, current value, and draft identity. |
| Settings | Set/step wall drawing dimensions; set opening category and supported placement dimensions; set arc mode; set/step grid spacing; set opening gap; set temporary snapping suppression. |
| History | Undo; redo. These cause document-restored effects and cancel drafts. |
| Project | New; open a captured path; save current project; save to an explicit path; close; decide Save/Discard/Cancel for a particular pending request. Save and open destinations are distinct. |
| Camera | Begin/end navigation gesture; orbit delta; pan delta; pan axis with frame duration; zoom amount; reset. Target the configured camera/viewport. |
| UI/platform | Focus/blur, submit/cancel focused field, scroll panel, window focus lost, pointer unavailable, window close. Keep UI-only actions local to the adapter while document effects use core actions. |

Distinguish an **action** from a **proposal**. A proposal describes a potential
document change. Actions also include navigation, settings, cancellation, and
selection, which usually do not create history entries or dirty the document.
Do not treat every pointer update as a committed document command.

### Pointer and continuous-input semantics

Supply a device-neutral pointer sample containing viewport position in logical
pixels, graph-space ray, optional ground hit, and resolved snap information as
needed. A ray may exist without a ground hit; opening placement and selection
must continue to work on elevated surfaces in that case.

Represent press/update/release distinctly. A single `PlaceWall` trigger cannot
express the current click-click gesture, drag threshold, free-arc bend stage, or
continuous handle preview. The core interprets these semantic phases according
to the active tool; it never receives a physical key or button code.

For pointer actions queued before camera movement/projection is complete, retain
the original screen position and resolve it against the current camera before
dispatch. Do not resolve a press using a later unrelated pointer position. Keep
discrete edge order; continuous motion may be coalesced only where that does not
change press/release or gesture-threshold behavior.

Camera pan/orbit/zoom can use per-frame values or accumulated deltas. Temporary
snap suppression and navigation axes need held state with start/end semantics,
including reset on focus loss or binding changes. Avoid emitting a toggle on
every held-key frame.

### Configurable bindings and defaults

Create one binding resource/profile in the Bevy input adapter. It maps physical
triggers, modifiers, phase, and context to semantic actions. Changing the
resource must be sufficient to rebind an operation without changing tool,
history, camera, or project code. Persistence or a binding-editor UI can follow
later; neither is necessary to establish this boundary.

Preserve current bindings as the initial profile:

| Current default | Mapped operation |
| --- | --- |
| Left press/release/movement in viewport | Active-tool gesture phases |
| Escape | Contextual field/gesture/pending-edit cancellation, then selection clearing |
| Enter / Numpad Enter | Focused field submit, otherwise pending wall-deletion confirmation |
| Delete | Selected-target deletion request; wall retains its preview/confirmation step |
| Ctrl+Z | Undo |
| Ctrl+Shift+Z / Ctrl+Y | Redo |
| Ctrl+S / Ctrl+O | Save / Open |
| Shift held | Temporary snapping suppression |
| Right drag / middle drag | Orbit / pan, matching current code |
| WASD | Camera pan axis, without conflicting shortcut modifiers |
| Home | Camera reset |
| Wheel over panel / viewport | Panel scroll / camera zoom |
| Window close | Project close request |

The older implementation plan lists a different orbit/pan button order; use
the current implementation as the behavior baseline and make that mapping
configurable. Tool buttons remain action producers even where no keyboard
shortcut currently exists. Generate shortcut labels and interaction hints from
binding metadata so rebinding does not leave hard-coded “Hold Shift”, “Enter”,
or click instructions in readouts.

Interpret “nothing hard coded” as no fixed physical-input-to-editor-operation
mapping outside the binding profile. Put adjustable interaction policy—drag
threshold, camera sensitivity/speed/limits, snap radii and grid choices,
drawing defaults/steps—in validated settings with defaults. Domain invariants,
file version, coordinate conventions, and styling may remain deliberate named
constants; making every number a user preference would obscure the model.
Keep geometry validation independent of settings that only affect convenience.

Bevy may continue handling character insertion, caret movement, and native text
editing inside the widget adapter. Every effect on editor state still passes
through an action. Likewise, marker/readout/transform synchronization observes
state directly; presentation updates do not need to emit editor actions.

### Ownership, validation, and ordering rules

- Resolve context centrally: unfocused window, pending project decision, focused
  text field, UI-owned pointer, captured gesture, and ordinary viewport. Specify
  exact precedence and enabled actions, rather than spreading `TextFocus` and
  interaction checks across tools.
- Decide the pending-unsaved-prompt policy explicitly. Currently project
  replacement requests are gated, but other editing/camera systems do not check
  the prompt. Whether it should be fully modal is a product choice, not an
  accidental consequence of the refactor. In either case the pending operation
  and its destinations must remain stable.
- A press has one gesture owner: camera, UI, wall handle, opening drag, or active
  tool. Selection and beginning a drag from that selection can be one coordinated
  response; independent readers must not start competing gestures.
- Continue delivering release/cancel to the gesture owner when the pointer moves
  over UI or misses the ground. Preserve the current valid-last-preview policy
  where tools intentionally finish off the ground. Window focus loss must end
  capture and clear held inputs even if no release arrives.
- Enter/Escape are routed once. A field's Escape must not also cancel the entire
  arc or clear selection; submitting a field must not confirm an unrelated
  deletion. Cancellation takes priority over tool confirmation in one frame.
- Validate action targets at execution. Include document revision/draft identity
  for target-sensitive operations and bind delayed UI requests to their original
  target. Clear pending actions on document replacement; an ID from another
  document must never be accepted merely because its numeric value exists.
- Drain/reject disabled or stale actions promptly with an appropriate outcome.
  Do not defer a delete or duplicate to the next selection or tool activation.
- Process document-changing actions in a documented sequence. Undo/new/open
  invalidates earlier pointer-derived context; later actions must re-evaluate
  against the new revision or be rejected. Preserve multiple intentional history
  steps; avoid duplicate execution when a handler formerly had both a request
  and an inline shortcut path.
- Invalid values, including NaN/infinity, overflowing `f64` to `f32` conversion,
  nonpositive spacing, invalid camera parameters, and stale handles, leave state
  unchanged and produce explicit rejection information. UI filtering alone does
  not protect actions invoked programmatically.
- Make active-tool/settings/selection writes private to their owners. UI code
  may mutate widget text, focus, and visual state, but never commit the document
  or directly change editor settings/drafts.

## Scheduling constraints

The present schedule is meaningful: camera work runs in `Update`; project and
history restoration precede tools; current camera transforms feed pointer and
snap queries; field submission runs after `EditableTextSystems`; readouts and
render updates precede text measurement/layout and transform propagation
(`editor_schedule.rs:23`). Preserve these guarantees rather than simply moving
all input into the start of `Update`.

Use explicit adapter phases:

1. Capture device edges and UI intent; resolve focus/context and early actions
   needed for camera navigation and settings.
2. Apply navigation actions and update camera projection/pose.
3. After the camera update, process project/history/tool transitions and ensure
   document-dependent query state is current.
4. Project pointer samples, compute snap/hover/handle targets, and dispatch tool
   gesture actions. Handle priority must be established before selection.
5. After Bevy applies queued text edits, produce field value/submit actions and
   dispatch them in the same frame. A generic Enter action must wait for this
   value instead of reading yesterday's widget text.
6. Reconcile selection/previews after commits; render meshes/markers/guides and
   update UI before the existing layout/propagation deadlines.

Actions produced in a phase need an explicitly ordered consumer in that phase
or a documented later phase. Do not leave them dependent on incidental plugin
registration order. Make the integration plugin own this composition; reusable
core modules should know no Bevy schedule names.

## Staged implementation plan

Each stage should be small enough to understand and review. Do not mix changes
to geometry algorithms, the persistence schema, history strategy, or renderer
performance into structural moves.

### 1. Record behavior and cover the two correctness gaps

Establish a test baseline. Add focused regression tests for rejected arc-radius
submission and Save-then-Open preserving distinct destinations, including an
assertion on the actually opened document. Review/fix those behaviors separately
from mechanical moves. Preserve the existing deletion confirmations and gesture
semantics; settle prompt modality before implementing any changed policy.

Exit condition: the intended behavior is explicit, failures are reproducible,
and current checks have a recorded result.

### 2. Establish actions and configurable bindings inside the current crate

Add cohesive `actions`, `input`, and settings modules without introducing a new
input dependency. Translate the existing tool/history/project/deletion request
messages into the action stream, then migrate consumers family by family.
Introduce explicit path/target/value payloads and context ownership early.

Migrate history/project shortcuts, settings buttons, selection/deletion,
wall/opening/arc gestures, property fields, snapping, camera, and panel/focus
input. Remove each old raw-input path when its action consumer lands. Update
help labels from the binding profile.

Exit condition: all editor effects are invocable through actions, rebindings
change behavior without tool edits, and physical input types occur only in
designated input/widget adapters and their tests.

### 3. Extract `house_document`

Move candidate evaluation/proposals, document/history/dirty tracking, categories,
format, and filesystem modules together. Replace Bevy vectors/derives at this
boundary and adapt Bevy resources with local wrappers. Move tests with their
modules; do not widen APIs simply to keep tests compiling.

Exit condition: document editing and file round trips build/test without Bevy,
and the editor retains the existing validation, history, and preview guarantees.

### 4. Separate and extract `house_editor_core`

Move geometry queries and snapping first, then selection/settings, then each
gesture state machine and property workflow. Move project pending-operation
logic into a pure controller with explicit I/O/exit effects. Consolidate commit
and invalidation effects as responsibilities become clear.

Exit condition: tests drive tool actions, drafts, selection, validation, undo,
and pending project decisions without a Bevy app, window, UI, or assets.

### 5. Organize the Bevy integration as a library

Expose a small composing plugin and action submission surface. Separate input,
viewport/camera, render, UI, and schedule modules; move markers/highlights/guides
out of tool logic. Keep `main.rs` as app composition. Share scene conversion
with the demo only as a separately scoped follow-up if desired.

Exit condition: another Bevy host can install the integration and send actions
without copying `main.rs` or enabling this specific sidebar UI. Core tools remain
usable without the stock physical bindings.

### 6. Verify the boundary and remove migration shims

Remove obsolete messages, inline shortcut helpers, duplicated focus policy, and
direct UI mutations. Audit all files again, exercise an alternate binding
profile, and confirm behavior across frame boundaries and project replacement.

Exit condition: the checks below pass, the dependency graph is acyclic, and
unnecessary public exports have been removed.

New crates require manifest/path dependencies, and pure vector imports will
require explicit `glam` declarations. Obtain approval for those dependency
changes before implementing them, as required by `AGENTS.md`. This report adds
no dependencies and does not assume approval for future manifest changes.

## Verification plan

Move the existing test suites with their responsibilities:

- Document: candidate/proposal tests from `tests/edit.rs`, `house`,
  `opening_category`, `project_file`, and `project_io`.
- Core: preview tests currently in `tests/edit.rs`, `snapping`, `opening_snap`,
  `opening_area`, all eight `wall_tool_*` behavior suites plus activation,
  field-operation cases from `opening_fields`/`wall_fields`/`arc_fields`, history
  action cases, and project/unsaved workflow cases.
- Bevy adapters: `coordinates`, `ground_pointer`, `orbit_camera`, `wall_render`,
  `editor_ui`, the integrated `editor` suite, and field focus/text-ordering cases.
  Split mixed suites as needed rather than dropping coverage.
- Split `tests/support.rs` into pure fixtures and minimal Bevy app helpers.
  Keep tests under each crate's `src/tests/` and retain `#[cfg(test)]` plus
  `#[path = "tests/<module>.rs"]` declarations.

Add meaningful action-boundary coverage:

1. UI, shortcut, and direct-action invocation have the same result; rebinding
   confirm/cancel/tool press/navigation/snap suppression disables the old binding
   and enables the new one without changing any consumer.
2. Ctrl+S saves once and does not pan; focused text retains text-edit shortcuts;
   unfocused windows do not generate viewport actions.
3. Press over UI does not use an old ground hit. A ray without a ground hit can
   still pick an opening/wall. Release over UI ends an owned drag. Focus loss,
   tool switch, and document restore discard capture and unfinished gestures.
4. Handle press does not become a competing selection; selecting then dragging
   an opening/wall still works. Escape wins over a simultaneous tool confirm.
5. Field Enter includes text queued in that frame. Invalid text/radius never
   commits an older valid preview. A field cancel does not cancel another owner.
6. One successful gesture is one undo step; rejected/cancelled previews are none;
   graph, mesh, categories, dirty state, and selection remaps remain consistent.
7. Stale target/revision/draft actions are rejected, including actions buffered
   across inactive tools, undo, or project replacement.
8. Save A then Open B still opens B after the prompt; an unnamed A requests its
   own save path; failed saves/imports preserve document/history/dirty state.
9. Unchanged actions/previews do not repeatedly regenerate meshes or text.
   Marker transforms and newly committed geometry appear in the same frame.
10. Invalid configuration/action payloads fail without partial mutation; snap
    acquisition/release priority and geometric tolerance behavior stay covered.

Run targeted tests after each change, then the workspace formatter and relevant
workspace checks at integration milestones. Expected commands, to be run during
implementation rather than claimed as already passing:

```sh
cargo test -p house_editor
cargo test -p house_document
cargo test -p house_editor_core
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
```

The two new-package commands only apply after those packages exist. Preserve
the repository's tab-based Rust formatting and run `cargo fmt --all` after Rust
changes. Run an editor smoke test with both the default and an alternate binding
profile; automated headless tests cannot establish interactive usability alone.

Use a source audit as a supplement to behavioral tests: search production code
for `ButtonInput`, `KeyCode`, `MouseButton`, mouse accumulators, and window cursor
reads outside designated adapter modules; inspect UI code for `House::commit`,
proposal evaluation, or direct mutation of core settings/drafts. Allow widget
text/focus/style changes and platform projection/capture in their proper adapters.

## Deliberately deferred

Do not add a generic command framework, binding persistence/UI, async file I/O,
incremental meshes, spatial acceleration, history compression, schema changes,
new tool shortcuts, furniture/floors, or a broad settings screen as part of this
refactor. The boundaries above support those later, but each needs its own
concrete use case and validation. Keep this work focused on reuse, explicit
ownership, and one action path for every input source.

## Implementation status

`cargo test --workspace` and `cargo fmt --all -- --check` pass. Tests: `house_document` 51,
`house_editor_core` 82, `house_editor` 147 (plus the geometry crates). Clippy is clean for the
three editor crates; the remaining warnings are in `wall_graph`, `wall_mesh` and `house_builder`.

Implemented:

- **Stage 1.** Save-then-Open keeps the requested destination; an unnamed project asks where to
  save instead of overwriting the file to open. Submitting an impossible arc radius no longer
  commits an older preview. Regression tests for both.
- **Stage 2.** `input.rs` is the only module that reads keys and buttons (the window cursor is
  read in `ground_pointer`/`orbit_camera`). `Bindings` resolve in a context (viewport, text
  field) into `ActionState` for gestures and held state; every consumer reads it. Fixed: Ctrl+S
  also panning, Escape and Enter routed once by context, straight-wall cancel ignoring text
  focus, unfocused windows producing actions. Control names in the panel are generated from
  `Bindings`, so rebinding updates them (the edit readout refreshes when the preview changes).
- **One action stream.** Requests (tool activation, history, project and unsaved-changes answers,
  settings, property edits, wall deletion/confirmation, opening delete/duplicate) are all
  `EditorAction` messages sent through `ActionSender`. Each consumer reads the stream in order
  and acts on its own kind, so an answer sent before its question is ignored and a question and
  answer in one frame are handled in sequence. Actions for a tool that is not active are
  consumed, not left to fire later. Property edits carry the document revision they were made
  against and are refused if the document has changed.
  - **Settings** (`SettingsAction`): wall height/thickness, arc mode, opening category, grid
    spacing, opening gap; applied before tools run. The gap field takes effect one frame after
    Enter.
  - **Properties** (`PropertyAction`): wall, opening and arc fields and field cancel, carrying
    target, value and revision. Handlers validate (finite, representable as `f32`); a submit is
    applied only if that value's preview is valid. They can be sent with no widget.
  - **Modal prompt.** While an unsaved-changes question is open `resolve_actions` produces no
    actions and a second project request is ignored.
- **Stage 3.** `house_document` (proposals, `House` with history and dirty tracking, categories,
  project format and file I/O) builds and tests without Bevy. `house_editor::House` is a thin
  resource wrapper with explicit mutation forwarding.
- **Stage 4.** `house_editor_core` holds: `Action`/`ActionState`; snapping policy and algorithm;
  `Preview` bookkeeping; picking, `SelectionTarget` and `Tool`; the wall-drawing, arc,
  handle-drag and opening-drag state machines with their proposals; opening placement defaults
  and proposal; selection retention, draft staleness and deletion-confirmation decisions;
  opening snap/area queries; `InteractionSettings`; and `ProjectWorkflow`. Bevy resources wrap
  these through small `Deref` newtypes.
- **Settings.** `InteractionSettings` (drag threshold, snap acquire/release radii) and
  `CameraSettings` (distance and elevation limits, orbit/pan/keyboard speeds, zoom factor) have
  defaults equal to the old constants and validate on every change; refused values leave the
  setting unchanged.
- **Stage 5.** `house_editor` is a library exposing `HouseEditorPlugin`, `Bindings`,
  `ActionState`, `EditorAction`, `ActionSender`, `InteractionConfig`, `CameraSettings` and
  `OrbitCamera`; the binary only sets up the window and scene.
- **Issue 5.** The project-file tolerance comment says `1e-3` is deliberately different from the
  graph's `1e-4`; the value is unchanged and the reason is not tested.

Audit: no key, button or scroll type is read outside `input.rs`; `house_document` and
`house_editor_core` have no Bevy dependency; the crate graph is
`wall_graph`/`wall_mesh` <- `house_document` <- `house_editor_core` <- `house_editor`.

Not done:

- Bindings are not saved or loaded; there is no file format for them. Saving needs a way to
  name every key, which Bevy's `KeyCode` does not provide.
- The Bevy systems for hover/select, the delete tool and opening placement stay in
  `house_editor` as thin wrappers over core decisions.
- Gestures are read from `ActionState` rather than sent as actions, so they have no revision or
  identity of their own; they are resolved against the current document each frame.
- No interactive smoke test with the default or an alternate binding profile was run.
