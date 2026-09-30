# Editor step 1: camera and viewport controls

This expands step 1 of [the implementation order](editor-implementation-order.md).
Start after [step 0](editor-step-0.md): the separate editor app should open on
an empty scene with a camera and ground plane. Work through one control at a
time and use the checks to judge how it feels.

## Tasks

### 1.1 Give the camera explicit orbit state

Represent the camera's focus point, distance, yaw, and pitch as editor state.
One reasonable choice is a component on the camera; it gives a camera update
system something specific to query. Derive the camera transform from that
state, rather than incrementally rotating and translating the current
transform in several unrelated systems.

Learn here: `Query` accesses components and `Res`/`ResMut` access resources.
Decide which values belong to the camera entity and which are global editor
settings. Keep the transformation math in one place so orbit, pan, zoom, and
reset agree about the focus point.

**Check:** changing the initial focus or distance changes the view in the way
you expect. The camera keeps looking at its focus.

### 1.2 Add middle-button orbit

While middle mouse is held, horizontal pointer motion changes yaw and vertical
motion changes pitch. Clamp pitch so the camera cannot flip or travel below
the ground. Keep left-button motion unused by the camera. Mouse motion is
already a per-frame delta; avoid multiplying it by frame time a second time.

Learn here: Bevy's `ButtonInput<MouseButton>` reports button state, and
`AccumulatedMouseMotion` reports the motion collected during the frame.
Choose a sensitivity that feels steady at different frame rates.

**Check:** orbit works in both directions, pitch stops cleanly at its limits,
and left drag does nothing to the camera.

### 1.3 Add right-button pan

Right drag moves the focus across the ground plane; the camera follows because
its transform is derived from that focus. Pan relative to the current camera
orientation, so dragging still feels sensible after orbiting. Scale the pan
speed with camera distance so a far-away view does not move painfully slowly.

Learn here: a camera has local right/up directions, while the ground plane has
world axes. Decide how to project the drag onto the ground and test a near
top-down view as well as a shallow angle. Handle any direction that becomes
nearly zero instead of normalizing it blindly.

**Check:** pan does not change yaw, pitch, or distance. The focus stays on the
ground plane, and movement is useful at both near and far zoom levels.

### 1.4 Add wheel zoom

Wheel input changes orbit distance while leaving the focus in place. Clamp the
distance to avoid passing through the focus or zooming so far out that the
editor becomes unusable. Bevy reports both line and pixel scroll units; test
with the input device you have.

Learn here: `AccumulatedMouseScroll` provides a frame's scroll input. Choose
whether a multiplicative distance change feels more consistent than a fixed
metres-per-notch change; try both before settling on one.

**Check:** repeated scrolling never crosses the near or far limits. Orbit and
pan still feel predictable after zooming.

### 1.5 Add a reset/focus action and scale cues

Provide one simple action that returns the camera to a known view of the
origin. It can be a key or a minimal UI button for now. Later the same camera
operation can frame the house or a selected object. Add a few visual scale cues
on the ground (for example, metre marks and clear axes); avoid making an
infinite-grid shader a prerequisite for camera work.

Learn here: separate the input that *requests* a reset from the camera
operation that performs it. That keeps a future UI button and keyboard shortcut
from implementing the same camera math twice.

**Check:** after arbitrary orbit, pan, and zoom, reset returns to the same
useful view. You can tell how large one metre is.

### 1.6 Keep UI and viewport input separate

Add a small editor UI region so you can test pointer ownership. Camera gestures
should not start when the pointer is over that region. Ignore camera input
while the window is unfocused. If a drag begins in the viewport and the pointer
crosses the UI, decide whether that drag continues until release; implement
one consistent rule and verify it manually.

Learn here: UI interaction and viewport input are different consumers of the
same mouse. Keep the decision about which one owns a gesture in one place.
You do not need the full properties panel yet.

**Check:** orbit and pan work over the viewport, but interacting with the test
UI cannot move the camera. Button release never leaves a stuck drag state.

### Step 1 is complete when

- [ ] Middle drag orbits, right drag pans, and the wheel zooms.
- [ ] Left drag remains available for future build and move tools.
- [ ] Reset/focus returns to a predictable view; ground scale is visible.
- [ ] UI and window focus prevent unintended camera gestures.
- [ ] You have manually tried the controls at near/far zoom and shallow/top-down
      angles, then run the workspace formatter and `cargo check -p house_editor`.

## Useful Bevy references

These are references for the concepts above, not code to copy into the editor:

- [Resources](https://bevy.org/learn/quick-start/getting-started/resources/)
  for global editor settings versus components on the camera.
- [Bevy camera orbit example](https://bevy.org/examples-webgpu/camera/camera-orbit/)
  for input and transform concepts; adapt the controls to this editor.
- [Bevy mouse input API](https://docs.rs/bevy/latest/bevy/input/mouse/index.html)
  for motion and scroll resources.
