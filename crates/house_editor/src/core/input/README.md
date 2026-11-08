# Input

The input module turns raw keyboard, mouse and UI input into **actions**: named things that can
happen in the editor, such as `editor.undo` or `camera.orbit`. Your code reacts to actions and never
to keys. Which input triggers an action is decided by a keymap file that users can edit.

This guide covers the public API. For how a frame is processed internally, see the module
documentation in `mod.rs`.

## Contents

- [Quick start](#quick-start)
- [Concepts](#concepts)
- [Defining actions](#defining-actions)
- [Registering contexts](#registering-contexts)
- [Loading the keymap](#loading-the-keymap)
- [Reading actions](#reading-actions)
- [Contexts in practice](#contexts-in-practice)
- [UI widgets](#ui-widgets)
- [Shortcut hints](#shortcut-hints)
- [The keymap file](#the-keymap-file)
- [Reference](#reference)

## Quick start

Four steps connect an action to an input:

1. Define the action and register its group.
2. Register the contexts the keymap can name.
3. Load the keymap.
4. Read the action in a system.

```rust
// 1. Define the action (see "Defining actions").
app.add_plugins(InputPlugin)
    .add_input_actions::<EditorAction>();

// 2. Register contexts.
app.add_input_context(InputContext::new("global").priority(0));

// 3. Load the keymap once actions and contexts exist.
app.insert_resource(Keymap::load(
    app.world().resource::<ActionState>(),
    app.world().resource::<InputContexts>(),
    include_str!("../../assets/keymap.toml"),
));

// 4. Read the action.
fn handle_save(state: Res<ActionState>) {
    if state.just_pressed(EditorAction::Save) {
        // save the project
    }
}
```

```toml
# assets/keymap.toml
[contexts.global.bindings]
"editor.save" = ["ctrl+s"]
```

## Concepts

| Term | Meaning |
| --- | --- |
| **Action** | Something that can happen. It has a stable name (`camera.orbit`) and a value kind. It says *what* happens, never which input causes it. |
| **Value kind** | What an action carries: a button, a toggle, or a 1D or 2D axis. |
| **Keymap** | The file that binds inputs to actions by name. |
| **Binding** | One keymap entry: an input bound to an action within a context. |
| **Shortcut** | A button input with the exact modifiers it needs, such as `ctrl+shift+z`. Called a `ButtonCombo` in code. |
| **Context** | A named, prioritized layer such as `global` or `viewport`. Its bindings only work while it is active. |
| **Source** | The kind of device an input comes from: keyboard, mouse or UI. |

## Defining actions

Implement `Action` for an enum and `ActionGroup` to list its variants. Registering the group makes
every action available to the keymap.

```rust
#[derive(Clone, Copy)]
enum EditorAction {
    Undo,
    ToggleSnap,
}

impl Action for EditorAction {
    // Written in the keymap file, so keep names stable even if you rename a variant.
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

Names must be unique across all groups. `Action` requires `Copy`; `ActionGroup` also requires
`'static`.

### Value kinds

The value kind decides which inputs an action can be bound to.

| `ValueKind` | Meaning | Can be bound to |
| --- | --- | --- |
| `Button` | Held or not. | Shortcuts |
| `Toggle` | Flips on every press and keeps its value between frames. | Shortcuts |
| `Axis1D(mode)` | One number. | `scroll` (relative), 1D composites (absolute) |
| `Axis2D(mode)` | Two numbers. | `mouse_motion` (relative), 2D composites (absolute) |

`AxisMode` describes what an axis value means:

- `Relative`: the change since last frame, such as mouse motion. Bindings of one source add up.
- `Absolute`: a position, such as which directions are held. Bindings of one source cancel out
  (W and S) but never exceed the strongest single binding, so WASD and the arrow keys held together
  are still one stick at full tilt.

The keymap is rejected at load time if an input does not match the kind of its action. For example,
`mouse_motion` cannot drive a button or an absolute axis.

## Registering contexts

Register each kind of context at startup with `add_input_context`. This must happen before the
keymap loads.

```rust
app.add_input_context(InputContext::new("global").priority(0))
    .add_input_context(InputContext::new("viewport").priority(10));
```

`add_input_context` panics if the name or the priority is already taken, because that is a mistake
in startup code. Priorities must be unique so that the order of the stack is never ambiguous. A
higher priority sees an input first.

`InputContext` has a builder API for modal behavior:

| Method | Effect |
| --- | --- |
| `priority(i32)` | Sets the priority. The default is `0`. |
| `block(Block)` | Swallows a class of input for the contexts below, even where this context binds nothing. |
| `pass(ButtonCombo)` | Lets exactly this shortcut through to the contexts below, even if a block rule matches it. |

Blocking and passing are written in code and never in the keymap file, so an edited keymap cannot
make a modal context leak input.

```rust
// A text field: swallow typing, but keep Ctrl+S working.
InputContext::new("text_field")
    .priority(20)
    .block(Block::Typing)
    .pass(ButtonCombo::ctrl(KeyCode::KeyS))
```

| `Block` | Swallows |
| --- | --- |
| `Block::All` | Every input. |
| `Block::Modifier(modifier)` | Any input that includes this modifier, and the modifier key itself. |
| `Block::Key(key)` / `Block::key(key)` | This key with any modifiers. |
| `Block::Typing` | What a text field consumes: letters, digits and space with no modifier or only Shift; backspace, delete, enter and the arrow keys; and Ctrl+A/C/V/X/Z/Y. Ctrl+Z and Ctrl+Y are swallowed even when there is nothing to undo in the field, so they never act on the scene. Mouse input is not swallowed. |

## Loading the keymap

```rust
Keymap::load(actions: &ActionState, contexts: &InputContexts, built_in: &str) -> Keymap
```

`load` reads `assets/keymap.toml` from the asset folder. `built_in` is the default keymap text,
usually embedded with `include_str!`.

- If the file is missing or invalid, `load` logs a warning and uses `built_in`. A broken file never
  leaves the app without shortcuts.
- If the file is valid but leaves an action unbound that the built-in keymap binds, `load` logs a
  warning for each one. It does not fill them in, because a user may have removed a binding on
  purpose.
- The built-in keymap must be valid. `load` panics otherwise.

Actions and contexts must be registered before you call `load`. The keymap is validated against
them, and the whole keymap is built before it is returned, so an error never produces a partial
keymap.

The keymap is a normal Bevy resource. Until you insert it, the keyboard and mouse sources skip
their frames instead of panicking, so you do not need to order plugins around it.

## Reading actions

Read the resolved state from `Res<ActionState>` in `Update` or later. All reads take an action
value (`impl Action`).

| Method | Returns |
| --- | --- |
| `pressed(action)` | `true` while a button action is held. `false` for any other kind, toggles included. |
| `just_pressed(action)` | `true` for the frame a button action became held. |
| `just_released(action)` | `true` for the frame a button action stopped being held. |
| `toggled(action)` | `true` while a toggle is on. `false` for anything that is not a toggle. |
| `just_toggled(action)` | `true` if a toggle changed since the last frame began. |
| `axis1d(action)` | The value of a 1D axis, or `0.0` for any other kind. |
| `axis2d(action)` | The value of a 2D axis, or `Vec2::ZERO` for any other kind. |
| `toggled_named(name)` | Like `toggled`, for code that only has the action name. |

Reading an action with the wrong method returns the neutral value (`false`, `0.0` or zero) rather
than an error. If an action is not registered, it also reads as neutral.

```rust
fn camera_system(state: Res<ActionState>) {
    let look = state.axis2d(CameraAction::Look);   // mouse delta this frame
    let movement = state.axis2d(CameraAction::Move); // held directions, diagonals normalized
    if state.pressed(CameraAction::Orbit) {
        // orbit with `look`
    }
}
```

### How a frame is timed

Input runs in `PreUpdate`, after Bevy has processed the devices. It has three phases, exposed as
`ActionSystems`:

1. `Begin`: clears last frame's contributions and rebuilds the active contexts.
2. `Sources`: the sources contribute signals to actions.
3. `Resolve`: the contributions of each action are combined into one value.

Read `ActionState` in `Update` or later so you never see a half-built frame. If you write your own
source system, put it in `ActionSystems::Sources`.

### Several inputs, one action

- **Buttons**: an action is held if any of its shortcuts is held. With two shortcuts, it stays
  pressed until both are released.
- **Toggles**: an action flips on the press from whichever source presses it, and not again while
  the input stays held.
- **Axes**: inputs from the same source combine. Different sources compete: the first source to
  leave neutral owns the axis until it returns to neutral. This stops a keyboard and a future
  controller from fighting over the camera.

## Contexts in practice

A context is active while at least one entity with an `InputContextRef` for it exists. The UI that
owns the context spawns and despawns the entity.

```rust
// The viewport context is active while this entity exists.
commands.spawn(InputContextRef("viewport"));
```

The active contexts are rebuilt every frame from the entities that exist, so a despawned dialog can
never leave its context behind. If an entity names a context that was never registered, input logs
a warning once and ignores it.

When several contexts bind the same input, the highest one wins:

- **Live**: a binding acts only if its context is active and no higher context shadows or blocks
  its input.
- **Shadow**: a higher context that binds the same input (with the same modifiers) handles it, so
  the lower binding stays quiet. `ctrl+s` and `s` are different inputs.
- **Block**: a higher context can also swallow a whole class of input without binding it (see
  [Registering contexts](#registering-contexts)).

A shortcut that started keeps running while its input is held, but ends as soon as its binding is
no longer live. A dialog that opens in the middle of a drag therefore cannot leave the drag running.

## UI widgets

A widget can drive an action directly, without the keymap.

- Put a `UiAction` component on the widget. It converts from any action (`UiAction::from(action)`),
  and `UiAction::default()` is a widget with no action yet.
- When the widget is activated, trigger `UiActivated::button(action)`, or
  `UiActivated::button_named(name)` if the widget only knows the action's name.

```rust
commands.trigger(UiActivated::button(EditorAction::Undo));
```

The activation presses the action for one frame, like any other input. It is buffered from `Update`
and contributed in the next `PreUpdate`, so the action reads as pressed in the frame after the
trigger. An unknown action name, or an activation whose signal does not fit the action's kind
(a button press for an axis), is ignored with a warning.

Because UI is its own source, a widget click and a keyboard shortcut for the same toggle flip the
same shared value.

## Shortcut hints

Use the `InputHints` system parameter to show the shortcut of an action, for example in a tooltip.

```rust
fn tooltip(hints: InputHints) {
    if let Some(hint) = hints.for_action(EditorAction::Undo) {
        // hint.shortcut is a ButtonCombo, e.g. "Ctrl+Z" when formatted with Display
        // hint.availability tells you how to draw it
    }
}
```

- `for_action(action)` and `for_action_named(name)` return the best `ShortcutHint` for an action,
  or `None` if it has no shortcut.
- "Best" means shortcuts that work now come first, then blocked ones, then those of inactive
  contexts. Within a group, the highest context comes first, then the order in the keymap file.
  When an action has several shortcuts, only the first is returned.
- Availability comes from the same check the sources use, so a hint cannot disagree with what the
  shortcut actually does.

`Availability` reports the state and leaves the look to your drawing code:

| `Availability` | Meaning |
| --- | --- |
| `Available` | Pressing the shortcut does something right now. |
| `ContextInactive` | The shortcut's context has no entity right now, such as the viewport while the cursor is elsewhere. |
| `Blocked` | The context is active, but a higher context binds or blocks the same input. |

`ButtonCombo` formats for display, so `ctrl+shift+z` prints as `Ctrl+Shift+Z` and `mouse_middle` as
`Mouse middle`.

Hints only describe button shortcuts. Axis bindings have no hint. `InputHints` needs the `Keymap`
resource to exist.

## The keymap file

The keymap is TOML with one section per context. Each key is an action name, and each value is a
list of entries.

```toml
[contexts.<context>.bindings]
"<action name>" = [ <entry>, ... ]
```

Unknown fields are errors, so a misspelled option is reported rather than ignored. A file in the old
format with a top-level `[bindings]` is reported for the same reason.

### Entries

An entry is either plain text or an inline table.

**Shortcut** (for button and toggle actions): modifiers first, the button last, case-insensitive.

```toml
"editor.redo" = ["ctrl+shift+z", "ctrl+y"]
```

**Axis input** (for axis actions): `mouse_motion` or `scroll`, optionally with modifiers.

```toml
"camera.look" = ["mouse_motion", "shift+mouse_motion"]
"camera.zoom" = ["scroll"]
```

**Table entry**: for anything that needs more than an input.

| Form | Use |
| --- | --- |
| `{ input = "scroll", scale = -1.0 }` | Multiplies an axis value. Use a number for 1D and `[x, y]` for 2D. Negative values invert. |
| `{ input = "e", interpret = "released" }` | Changes when a shortcut is active. |
| `{ composite = "2d", up = "w", down = "s", left = "a", right = "d" }` | Turns four buttons into a 2D absolute axis. |
| `{ composite = "1d", up = "space", down = "c" }` | Turns two buttons into a 1D absolute axis. |

A composite also takes `modifiers = ["shift"]` and `scale`. In an `input` entry, write modifiers in
the input text instead (`"shift+scroll"`); the `modifiers` field is only for composites.

### Available names

- **Modifiers**: `ctrl`, `shift`, `alt`. Left and right keys count as one.
- **Keys**: `a`–`z`, `0`–`9`, `f1`–`f12`, `space`, `enter`, `escape`, `tab`, `backspace`, `delete`,
  and the arrow keys `up`, `down`, `left`, `right`.
- **Mouse buttons**: `mouse_left`, `mouse_middle`, `mouse_right`. They are prefixed so they cannot
  collide with the arrow keys `left` and `right`.
- **Axis inputs**: `mouse_motion` (2D, relative), `scroll` (1D, relative).

Composite directions can be keys, mouse buttons, or modifier keys.

### Modifier matching

Modifiers must match exactly. A binding that needs `ctrl` does not match `ctrl+shift`, so
`ctrl+shift+z` does not also fire `ctrl+z`. This applies to axis inputs too: `scroll` stops while
Shift is held and `shift+scroll` takes over. Switching is immediate, because axis bindings are
evaluated every frame.

Shortcuts are different: modifiers only select the shortcut when it starts. After that it stays
active while its button is held, whatever the modifiers do, so releasing Shift in the middle of a
Shift+drag does not end the drag.

A composite does not move while a modifier it does not use is held, so the `S` of `ctrl+s` does not
also move the camera backward.

### Scroll units

Scroll is converted to lines, at 40 pixels per line, so an action sees one unit regardless of
whether the device is a wheel or a trackpad.

### Interpretation

`interpret` applies to shortcuts only. It is rejected on axis entries.

| Value | The shortcut is active |
| --- | --- |
| `"pressed"` (default) | While the button is held, starting on the press. |
| `"released"` | For one frame, when the button is released, even if a modifier was let go first. If the shortcut ends because its context went away, it was cancelled and does not fire. |

### What the loader rejects

The keymap fails to load, and the built-in keymap is used instead, when:

- a section names a context that is not registered, or an entry names an action that is not
  registered;
- an input does not fit its action's kind (a shortcut on an axis action, an axis input on a button,
  `mouse_motion` on an absolute axis);
- a `scale` has the wrong shape, is zero, or is not finite; or a shortcut has a `scale`;
- a shortcut, or an axis input with the same modifiers, is bound twice within one context;
- one button with its modifiers is used by two bindings within a context. That includes a shortcut
  and a composite direction, or two composites. Otherwise one press would drive two actions, such as
  `s` saving while it also moves the camera back;
- a composite uses the same button for two directions, is missing a direction, has a direction it
  does not take (`left` on a `1d`), or uses a modifier both as a direction and as a modifier;
- a modifier or button name is unknown or repeated.

Two different contexts may bind the same input. That is how shadowing works.

## Reference

### Exports from `core::input`

| Item | Purpose |
| --- | --- |
| `InputPlugin` | Adds the action state, the context stack and the sources. Does not insert the `Keymap`. |
| `InputAppExt` | `add_input_actions::<Group>()` and `add_input_context(context)` on `App`. |
| `ActionSystems` | `Begin`, `Sources`, `Resolve` system sets in `PreUpdate`. |
| `Action`, `ActionGroup` | Traits for defining actions. |
| `ValueKind`, `AxisMode` | The value of an action. |
| `ActionState` | Resource with the resolved value of every action. |
| `Keymap` | Resource holding the bindings. Created with `Keymap::load`. |
| `InputContext`, `InputContexts`, `InputContextRef` | Defining, storing and activating contexts. |
| `Block`, `ButtonCombo`, `Modifier` | Values for the blocking rules of a context. |
| `UiAction`, `UiActivated` | Connecting widgets to actions. |
| `InputHints`, `ShortcutHint`, `Availability` | Looking up shortcuts for display. |

### Not documented

The following is not covered because it is not observable from the public API or has not been
settled: gamepad and touch input (there are no such sources yet), and the stability of the public
API, which is still changing.
