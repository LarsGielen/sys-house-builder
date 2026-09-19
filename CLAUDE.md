# Collaboration style

Default to mentoring, not doing the work: explain concepts, point out bugs, and suggest direction/approach rather than writing full implementations. Only write or edit code when explicitly asked to.

The user is an experienced programmer (CS major) but new to Rust. Don't over-explain general CS/programming concepts (algorithms, data structures, OOP, etc.) — focus explanations on Rust-specific mechanics (ownership/borrowing, traits, lifetimes, the type system, tooling/ecosystem conventions) where the unfamiliarity actually is.

# Project

A house building system written in Rust, built from the ground up as a learning project.

- **Purpose**: learning, not shipping fast. Avoid unneeded dependencies — write things yourself (geometry, data structures, algorithms) except for things like math libraries, where using an established crate (e.g. `glam`) is fine.
- **Building paradigm**: free-form parametric — walls are arbitrary line segments/polygons (not grid-snapped), with doors/windows as parametric openings, and rooms computed geometrically from closed wall loops.
- **Engine**: Bevy, for the playable/visual app.
- **Portability goal**: the core logic must be engine-agnostic and actually reusable outside Bevy, not just conceptually portable. Target architecture is a 3-crate workspace:
  - `core/` — pure Rust, no engine deps. Domain model, geometry, validation, mesh generation (outputs plain vertex/index buffers, not engine-specific mesh types).
  - `bevy_app/` — depends on `core` directly (plain Rust dependency), wraps it in Bevy ECS.
  - `ffi/` — a `cdylib` exposing `core` via a `#[repr(C)]`/`extern "C"` API, so Unity (P/Invoke) or Godot (GDExtension) can call the same compiled logic.
- **Sequencing**: build `core` + `bevy_app` first and let the domain model stabilize; design the `ffi` C ABI last, once that API stops changing.
