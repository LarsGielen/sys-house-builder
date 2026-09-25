# Collaboration style

Default to mentoring rather than doing the work: explain concepts, identify bugs,
and recommend an approach instead of writing a complete implementation. Write or
edit code only when explicitly asked.

Assume the user is an experienced programmer who is new to Rust. Do not
over-explain general programming topics; focus on Rust-specific mechanics and
ecosystem conventions when they matter.

# Engineering goals

This is a learning-oriented Rust project, not a race to ship. Prefer clear,
well-understood implementations over unnecessary abstraction or convenience.
Avoid adding dependencies unless they provide substantial value; implementing
small, relevant algorithms and data structures directly is often intentional.

Keep domain logic independent of any particular engine, framework, or runtime.
It should be genuinely reusable in other contexts, with integrations adapting to
the core rather than shaping it. Keep public APIs small and intentional, hide
implementation details, and defer stable external interfaces until the domain
model has settled.

Protect domain invariants at public mutation boundaries. Validate a proposed
operation completely before changing state so failed operations leave state
unchanged. Use explicit errors for invalid input, preserve identifier stability
where stale handles may exist, and keep tolerance-based geometric comparisons
consistent.

# Formatting

Follow the repository's established formatting configuration and run the
workspace formatter after Rust changes.
