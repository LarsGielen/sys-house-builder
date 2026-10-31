# Parametric opening-shape editor

Status: proposed. Order: 8. Mostly independent of stairs/furniture. See the
[roadmap](README.md) for the shared generation and validation pipeline.

## Intended behavior

Draw a custom opening outline using straight and curved segments, then reuse it
at different sizes through named parameters. Provide a pipeline that supports
additional profile/segment types without coupling the model to the editor UI.

Named width/height parameters are in scope. Formula support is a proposed next
stage; confirm the initial expression vocabulary before building that editor.

## Current support and gaps

`wall_graph` supports exact line and circular arc geometry, plus arc solvers.
Openings currently use rectangular `OpeningSpec` values. Validation, overlap,
wall clearance, meshing, opening picking, and preview outlines assume those
rectangles.

The wall mesher subdivides stretches at rectangular opening edges and subtracts
constant vertical intervals. General arches/round holes require a substantive
extension, not just a new outline widget. The existing simple-polygon
triangulator also does not solve wall faces containing arbitrary holes.

## Proposed architecture

Separate three kinds of data:

- Template: named parameters, segment definitions, and parameter relationships.
- Evaluated profile: a validated closed contour in wall-local coordinates.
- Instance: template reference/parameters plus a host wall and placement.

Use horizontal distance along the host wall and vertical elevation as profile
coordinates. Define the mapping onto straight/curved host surfaces explicitly.
An opening's width follows the wall centerline convention already used today.

Reuse focused line/arc value operations, evaluation, and solvers. Do not reuse
the entire wall graph as the contour editor: profiles do not need physical wall
thickness, automatic wall merging, or general junction insertion behavior.

The evaluated contour must reach the domain layer responsible for opening
invariants. A document-only decorative profile around a rectangular graph hole
would let geometry and validation disagree.

## Implementation steps

1. Define a closed contour of line/circular-arc segments with consistent winding,
   continuity, bounds, and explicit validity checks.
2. Implement evaluated rectangle, arch, and round fixtures without an editor.
3. Generalize wall cutouts using a deliberate wall-local face clipping and
   triangulation strategy. Generate reveal surfaces around each contour and map
   them onto the host's thickness, including curved walls.
4. Validate containment, self-intersection, degenerate segments, overlap with
   other openings, junction clearance, and meshing limits. Conservative bounds
   may accelerate checks but must not replace actual contour validation.
5. Preserve the rectangular opening path/API where useful while migrating stored
   openings to the generalized representation.
6. Add named dimensions and reusable templates, with atomic instance updates.
7. Add line/arc drawing, handles, snapping, contour closure, and parameter binding
   in the engine-independent gesture layer and Bevy UI.
8. Add a small expression model when needed, with parameter dependency ordering,
   cycle detection, finite arithmetic checks, and explicit error reporting.
9. Update opening picking, gizmos, finishes/UVs, persistence, and navigation
   clearance to consume the evaluated contour.

Do not resize a circular arc by independently scaling sampled X/Y points: that
produces an ellipse. Re-evaluate its authored constraints. A round profile can
constrain width/height together; elliptical segments can be a later explicit
extension.

## Extension and persistence policy

Begin with one simple closed contour and bounded line/arc segments. Multiple
contours, islands, Béziers, and full SVG import are later capabilities. An
SVG-like drawing interaction does not require supporting the entire SVG format.

Persist authored templates and parameters, with explicit template version/update
behavior so editing a shared template does not unexpectedly change every house.
Generate contours/meshes during candidate validation. Invalid parameter edits
leave the last committed template/instances unchanged.

## Systems touched

| System | Responsibility |
| --- | --- |
| Reusable geometry | Contours, line/arc evaluation, intersections, bounds |
| `wall_graph` | Opening geometry, fit/overlap/clearance invariants |
| `wall_mesh` | Generalized cutouts, reveals, tags, UVs, curved-host mapping |
| `house_document` | Templates, instances, proposals, migration, persistence |
| Core/editor | Profile drawing, parameters, shape-aware picking/previews |
| Navigation adapter | Body-clearance queries against resolved opening shape |

## Validation and completion criteria

- One template produces several valid sizes without losing curve intent.
- Invalid dimensions, expression cycles, self-intersections, and failed fits
  are rejected atomically.
- Arch and round holes create closed, correctly wound meshes with meaningful
  surface tags, including on curved hosts.
- Overlap checks use the documented shape/gap policy consistently.
- Picking matches the actual contour, and navigation does not route a tall
  character through a profile that only clears at its center.
- Existing rectangular projects migrate; templates/instances round-trip and undo.
