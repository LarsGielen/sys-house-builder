# Collaboration style

Default to mentoring, not doing the work: explain concepts, point out bugs, and suggest direction/approach rather than writing full implementations. Only write or edit code when explicitly asked to.

The user is an experienced programmer (CS major) but new to Rust. Don't over-explain general CS/programming concepts (algorithms, data structures, OOP, etc.) — focus explanations on Rust-specific mechanics (ownership/borrowing, traits, lifetimes, the type system, tooling/ecosystem conventions) where the unfamiliarity actually is.

# Project

A house building system written in Rust, built from the ground up as a learning project.

- **Purpose**: learning, not shipping fast. Avoid unneeded dependencies — write things yourself (geometry, data structures, algorithms) except for things like math libraries, where using an established crate (e.g. `glam`) is fine.
- **Building paradigm**: free-form parametric — walls are arbitrary line segments/polygons (not grid-snapped), with doors/windows as parametric openings, and rooms computed geometrically from closed wall loops.
- **Engine**: Bevy, for the playable/visual app.
- **Portability goal**: the core logic must be engine-agnostic and actually reusable outside Bevy, not just conceptually portable. Cargo workspace layout:
  - `crates/wall_graph/` — pure Rust, no engine deps (only `glam` for math). The core domain model: a wall graph inspired by a half-edge structure but custom to walls. Nodes are wall corners, walls are pairs of twin half-edges, and rooms will later be read off closed loops. Modules: `ids`, `error`, `geometry`, `graph` (+ `graph/validation`, and tests under `graph/tests`). Later this crate will also hold geometry, mesh generation (plain vertex/index buffers, not engine mesh types) and other engine-agnostic logic.
  - `crates/house_builder/` — the Bevy app; depends on `wall_graph` directly (plain Rust dependency), wraps it in Bevy ECS. Currently builds a hardcoded demo `WallGraph` and renders its nodes and walls as 2D sprites. (The package used to be called `bevy_app`, which collided with Bevy's own internal crate of that name.)
  - `crates/ffi/` (not created yet) — a `cdylib` exposing `wall_graph` via a `#[repr(C)]`/`extern "C"` API, so Unity (P/Invoke) or Godot (GDExtension) can call the same compiled logic.
- **Sequencing**: build `wall_graph` + `house_builder` first and let the domain model stabilize; design the `ffi` C ABI last, once that API stops changing.
- **Conventions in `wall_graph`**: faces lie to the left of each half-edge; `next(e)` turns to the first edge clockwise from `twin(e)` (see the doc comment on `HalfEdge`). Walls that duplicate, overlap or start and end at the same node are rejected with a `WallError`; crossing walls (inserting a node at the intersection) are not handled yet. The public API is `WallGraph` (`new`, `add_node`, `add_wall`, plus read-only `nodes`, `walls`, `node_position`), `Wall`, `WallError`, and the opaque `WallNodeId`/`HalfEdgeId`; `WallNode`, `HalfEdge` and the rest stay private.
- **Formatting**: tabs (`rustfmt.toml` sets `hard_tabs`); run `cargo fmt --all`.
