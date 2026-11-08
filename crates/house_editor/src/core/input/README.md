# Input

## The keymap file

The keymap is `assets/keymap.toml`. If it's missing or has a mistake in it, the app logs what's wrong and
runs with the copy of the keymap that's built into the binary, so a typo never leaves you without
controls. If your file leaves out an action the built-in one binds, you get a warning for that too, but
nothing is filled in for you. You might have removed it on purpose.

Bindings are grouped by context, which is just a named part of the editor where certain controls apply:

```toml
[contexts.global.bindings]
"editor.undo" = ["ctrl+z"]
"editor.redo" = ["ctrl+shift+z", "ctrl+y"]

[contexts.viewport.bindings]
"camera.orbit" = ["mouse_right"]
```

Each line is an action name and a list of bindings. An action can have as many as you like. The action
names come from the code, so a name that doesn't exist is an error, and so is a context that doesn't
exist. At the moment there are two contexts: `global` is always on while the editor is open, and
`viewport` is on while the cursor is over the 3D view.

### Shortcuts

A shortcut is a string with the modifiers first and the key last. Capitalisation doesn't matter.

```toml
"editor.save" = ["ctrl+s"]
"camera.pan"  = ["shift+mouse_right"]
```

The modifiers are `ctrl`, `shift` and `alt`, and the left and right keys count as the same. For keys you
have `a` to `z`, `0` to `9`, `f1` to `f12`, `space`, `enter`, `escape`, `tab`, `backspace`, `delete` and
the arrows (`up`, `down`, `left`, `right`). The mouse buttons are `mouse_left`, `mouse_middle` and
`mouse_right`. A modifier can also be the key itself, so `"shift"` is a shortcut for pressing Shift, and
`"ctrl+shift"` means Shift pressed while Ctrl is already down.

Modifiers have to match exactly. `ctrl+z` does nothing while Shift is held, which is why it can sit next to
`ctrl+shift+z` without both firing.

### Mouse movement and scrolling

Two more inputs exist for actions that take a movement instead of a press: `mouse_motion` and `scroll`.
Scrolling is measured in lines, and trackpads are converted to lines so they feel like a wheel.

```toml
"camera.look" = ["mouse_motion", "shift+mouse_motion"]
"camera.zoom" = ["scroll"]
```

The exact-modifier rule applies here too. With only `"scroll"` bound, scrolling stops working while
Shift is held, and you'd add `"shift+scroll"` if something should happen then. That's also why `look`
lists both `mouse_motion` and `shift+mouse_motion`: dragging with Shift held (to pan) still has to move.

### Options

When a binding needs more than an input, write it as a table: `{ input = "...", option = ... }`.

`scale` multiplies an axis. It's a single number for `scroll` and a pair, `[x, y]`, for `mouse_motion`.
A negative number reverses the direction. It can't be zero.

```toml
"camera.zoom" = [{ input = "scroll", scale = -1.0 }]
"camera.look" = [{ input = "mouse_motion", scale = [0.5, -0.5] }]
```

`interpret` changes when a shortcut counts. Normally it's active from the moment you press until you let
go (`"pressed"`). With `"released"` it's active for one moment, when you let go.

```toml
"some.action" = [{ input = "e", interpret = "released" }]
```

This only works on shortcuts, not on axes.

### Composites

A composite builds a direction out of held buttons. This is how WASD becomes movement:

```toml
"camera.move" = [
    { composite = "2d", up = "w", down = "s", left = "a", right = "d" },
    { composite = "2d", up = "up", down = "down", left = "left", right = "right" },
]
```

A `"2d"` composite needs all four directions, and a `"1d"` one needs only `up` and `down`. The directions
can be keys, mouse buttons or modifier keys. Opposite directions cancel out, diagonals aren't faster than
straight lines, and holding WASD and the arrows together doesn't double your speed. You can add
`modifiers = ["shift"]` to require modifiers, and `scale` works as it does elsewhere. A modifier key can't
be a direction and a required modifier of the same composite.

### What gets rejected

The file is checked as a whole before it's used. If anything is wrong, none of it is used.

Within one context, an input with its modifiers can belong to one binding, whether that's a shortcut or a
direction of a composite. The same key in two different contexts is fine, because the higher one wins when
it's active (more on that below). A binding also has to fit its action: a plain key can't drive a movement
axis (use a composite for that), `mouse_motion` can't drive a button, and shortcuts only work on actions
that are pressed or toggled.

### Which context gets the key

Contexts are stacked by priority, and the code decides the order. When you press something, the highest
active context that binds that exact key and modifiers handles it. A context can also block input from the
ones underneath it without binding anything, like a text field swallowing letters and Ctrl+Z, or a dialog
swallowing everything. It can let specific shortcuts through, such as Ctrl+S. Whatever isn't bound or
blocked falls through to the next context down.

So `w` can move the camera in the viewport and mean something else in a tool that sits on top of it, and
Ctrl+Z keeps working everywhere except inside a text field.

### Held controls

A shortcut like orbit, where you hold a button and drag, only needs its modifiers at the moment it starts.
Letting go of Shift halfway through a Shift+drag doesn't end the drag. It does end when its context goes
away, for instance if the cursor leaves the viewport.

Axes work differently. Mouse movement, scroll and composites are looked at every frame, so pressing or
releasing a modifier switches to the matching binding right away.

## Using it from code

### Defining actions

An action is a stable name plus the kind of value it carries. They're normally grouped in an enum.

```rust
use crate::core::input::{Action, ActionGroup, ValueKind};

#[derive(Clone, Copy)]
pub enum EditorAction {
    Undo,
    ToggleSnap,
}

impl Action for EditorAction {
    // These end up in the keymap file, so they have to stay the same even if a variant is renamed.
    fn name(self) -> &'static str {
        match self {
            Self::Undo => "editor.undo",
            Self::ToggleSnap => "editor.toggle_snap",
        }
    }

    fn value(self) -> ValueKind {
        match self {
            Self::Undo => ValueKind::Button,
            Self::ToggleSnap => ValueKind::Toggle,
        }
    }
}

impl ActionGroup for EditorAction {
    fn all() -> &'static [Self] {
        &[Self::Undo, Self::ToggleSnap]
    }
}
```

There are four kinds of value. A `Button` is either pressed or not. A `Toggle` is an on/off value that
stays where it is, and each press flips it. `Axis1D` is a number and `Axis2D` a vector, and both take an
`AxisMode`: `Relative` for things that report what changed this frame (mouse motion, scroll), `Absolute`
for things that report where something is right now (held movement keys).

### Setting up

Register the actions and contexts, then load the keymap. The order matters, because the keymap is checked
against what's registered.

```rust
app.add_plugins(InputPlugin);

app.add_input_actions::<EditorAction>()
   .add_input_actions::<CameraAction>();

app.add_input_context(InputContext::new("global").priority(0))
   .add_input_context(InputContext::new("viewport").priority(10));

app.insert_resource(Keymap::load(
    app.world().resource::<ActionState>(),
    app.world().resource::<InputContexts>(),
    include_str!("../../assets/keymap.toml"), // the built-in fallback
));
```

Registering a context whose name or priority is already taken panics. That's a mistake in the setup code,
so it's not something to recover from. The built-in keymap has to be valid too, or the app stops at
startup.

### Reading actions

Read `ActionState` in `Update` or later.

```rust
fn camera_system(state: Res<ActionState>, time: Res<Time>) {
    if state.pressed(CameraAction::Orbit) {
        let delta = state.axis2d(CameraAction::Look);
        // ...
    }
    let movement = state.axis2d(CameraAction::Move) * time.delta_secs();
    let zoom = state.axis1d(CameraAction::Zoom);
}

fn editor_system(state: Res<ActionState>) {
    if state.just_pressed(EditorAction::Undo) {
        // runs once, on the frame of the press
    }
    if state.just_toggled(EditorAction::ToggleSnap) {
        let snapping = state.toggled(EditorAction::ToggleSnap);
    }
}
```

For buttons there's `pressed`, `just_pressed` (the first frame only) and `just_released`. For toggles
there's `toggled` for the current value, `just_toggled` for the frame it changed, and `set_toggled` for
setting it from code, say to give it a starting value. Axes are read with `axis1d` and `axis2d`. Asking
for the wrong kind, like `pressed` on an axis, just gives you false or zero.

Relative axes are already per frame, so use them as they come. Absolute ones are a direction that's held
right now, so multiply by `time.delta_secs()` if you want a speed that doesn't depend on the frame rate.

When several inputs drive the same action, a button counts as pressed if any of them presses it, so a
click and a key at the same moment are one press. An axis follows whichever source moved it first until
that source goes back to neutral, which keeps a keyboard and a controller from fighting over the camera.
A toggle flips once per press, however many sources press it and however long the key is held.

None of this state is kept between frames except toggles. It's rebuilt each frame from what's held and
what exists, so a despawned widget or a closed dialog can't leave an input stuck.

### Contexts

A context is on the stack for as long as an entity with an `InputContextRef` exists, and whatever UI owns
the context is responsible for spawning it.

```rust
// on for as long as the editor is open
commands.spawn((
    InputContextRef("global"),
    DespawnOnExit::<AppState>(AppState::Editor),
));

// on only while the viewport is hovered
commands.entity(viewport).insert(InputContextRef("viewport"));
commands.entity(viewport).remove::<InputContextRef>();
```

Two entities with the same context count once. Using a name that was never registered logs a warning and
is ignored.

Blocking is set up where the context is registered, in code, and never in the keymap file. That way an
edited file can't make a dialog leak input.

```rust
app.add_input_context(
    InputContext::new("text_input")
        .priority(20)
        .block(Block::Typing)
        .pass(Shortcut::ctrl(KeyCode::KeyS)),
);
app.add_input_context(InputContext::new("dialog").priority(30).block(Block::All));
```

`Block::All` blocks everything. `Block::Modifier(m)` blocks anything that includes that modifier, plus the
modifier key itself, and `Block::key(KeyCode)` blocks one key with any modifiers. `Block::Typing` is
meant for text fields: plain and shifted letters, digits and space, the editing keys (backspace, delete,
enter, arrows) and Ctrl+A/C/V/X/Z/Y. Ctrl+Z and Ctrl+Y are blocked even when the field has nothing to
undo, so they always act on the text and never on the scene. `pass` lets one exact shortcut through even
if a block rule matches it. A context that binds an input itself always wins over anything it would pass.

### Buttons and toggles in the UI

A widget triggers an action by carrying a `UiAction` and sending a click when it's activated. The click
goes through the same path as a key press, so the two behave the same, and the widget doesn't need to
know what the action does. The widgets in the project already do this:

```rust
@icon_button(EditorAction::Undo, icons.get(Icon::Undo), size, ButtonOptions { hint: Some("Undo"), ..default() })
@toggle(EditorAction::ToggleSnap, "Snap", ButtonOptions { hint: Some("Snap to grid"), ..default() })
```

A button with `UiAction::default()` has no action and does nothing. A button for a toggle looks selected
while the toggle is on, no matter who flipped it. Clicks ignore contexts, so a button always does what it
says. Hovering a button shows its hint with the shortcut, like `Undo (Ctrl+Z)`, and the shortcut is
dimmed if it wouldn't work right now.

For a widget of your own, put a `UiAction` (made with `UiAction::from(action)`) on the entity and trigger
the click when it's activated:

```rust
commands.trigger(UiActivated::button(action_name));
```

### Showing a shortcut

`InputHints` gives you the shortcut to show for an action, for a tooltip or a menu.

```rust
fn update_label(hints: InputHints) {
    if let Some(hint) = hints.for_action("editor.undo") {
        // hint.shortcut displays as "Ctrl+Z"
        // hint.availability is Available, ContextInactive or Blocked
    }
}
```

You get the best shortcut for the action: one that works now if there is one, otherwise one that's
blocked, otherwise one from a context that isn't active. What to do with the availability is up to
whatever draws it.

### Timing

Input is collected in `PreUpdate` after Bevy has processed the devices, so it's ready when `Update` runs.
That's why you should read it in `Update` or later. A click on a UI button reaches the action state one
frame after it happens, which you won't notice.

### If something doesn't work

If the app uses the built-in keymap instead of yours, the log says what's wrong with the file. An
"unknown action" or "unknown context" means the name isn't registered, or is registered after the keymap
loads. "Is bound to both X and Y" means the same input with the same modifiers appears twice in one
context.

When a binding is valid but does nothing, check these in order: is its context active, does a higher
context bind or block that input, and are the modifiers held exactly the ones written? A control that
stops when the cursor leaves the viewport has lost its context, since the viewport context follows the
hover.
