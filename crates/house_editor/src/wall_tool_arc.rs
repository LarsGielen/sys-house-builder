use bevy::{ecs::system::SystemParam, prelude::*};
use wall_graph::{WallGraph, WallNodeId};

use crate::{
	coordinates::graph_to_bevy,
	edit::{ArcDefinition, ArcProposal, EditNotice, EditPreview, Proposal},
	ground_pointer::GroundPointer,
	history::DocumentRestored,
	house::House,
	snapping::{SnapSource, SnappedPointer},
	text_fields::TextFocus,
	wall_tool_active::{Tool, tool_active},
	wall_tool_drawing::WallDrawingSettings,
};

const DRAG_THRESHOLD_PIXELS: f32 = 6.0;
const GUIDE_COLOR: Color = Color::srgb(0.2, 0.8, 0.95);
const GUIDE_HEIGHT_METRES: f32 = 0.03;
const GUIDE_MARKER_RADIUS_METRES: f32 = 0.1;
const DIRECTION_GUIDE_LENGTH_METRES: f32 = 1.0;

/// Draws arcs. A free arc takes its endpoints like a straight wall and then a bend position;
/// the tangent modes continue one or two existing wall ends instead of asking for a bend.
pub struct WallToolArcPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WallToolArcSystems;

impl Plugin for WallToolArcPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<ArcMode>()
			.init_resource::<ArcDraft>()
			.init_resource::<WallDrawingSettings>()
			.init_resource::<EditPreview>()
			.init_resource::<EditNotice>()
			.add_message::<DocumentRestored>()
			.add_systems(
				PostUpdate,
				(
					discard_arc_draft_after_restore,
					restart_when_mode_changes.run_if(resource_changed::<ArcMode>),
					update_wall_arc.run_if(tool_active(Tool::DrawArc)),
					cancel_wall_arc.run_if(not(tool_active(Tool::DrawArc))),
					draw_arc_guides.run_if(tool_active(Tool::DrawArc)),
				)
					.chain()
					.in_set(WallToolArcSystems),
			);
	}
}

/// How the arc is defined. Switching modes abandons the arc in progress.
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArcMode {
	/// Endpoints, then a bend position.
	#[default]
	Free,
	/// Continue one wall end; the other endpoint is free.
	OneTangent,
	/// Continue a wall end at both ends.
	TwoTangents,
}

/// Where the current gesture is. Positions are graph metres unless noted.
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq)]
pub(crate) enum ArcDraft {
	#[default]
	Idle,
	/// A free arc's button is down. It is a click until the pointer passes the drag threshold.
	Pressed {
		start: Vec2,
		/// Viewport pixels, to measure the drag threshold.
		press_position: Vec2,
		dragged: bool,
	},
	/// The start is placed; the next click places the end of a free arc or confirms a
	/// tangent arc. `start_direction` is the direction of travel a tangent arc leaves in.
	AwaitingEnd {
		start: Vec2,
		start_direction: Option<Vec2>,
	},
	/// A free arc's endpoints are placed; the pointer picks the bend and a click confirms.
	AwaitingBend {
		start: Vec2,
		end: Vec2,
		bend: Vec2,
		/// A sweep typed into a field, which then wins over the pointer's bend.
		typed_sweep: Option<f32>,
	},
}

/// The direction a wall continuing the free wall end at `node` leaves in, or `None` when
/// `node` joins zero or several walls, which gives no single wall to continue.
fn free_end_continuation(graph: &WallGraph, node: WallNodeId) -> Option<Vec2> {
	let [wall] = graph.node_walls(node)?[..] else {
		return None;
	};
	let outgoing = if wall.origin() == node {
		graph.wall_tangent(wall, 0.0)?
	} else {
		-graph.wall_tangent(wall, 1.0)?
	};
	// A smooth join leaves in the opposite direction to the wall it continues.
	Some(-outgoing)
}

#[derive(SystemParam)]
struct ArcInput<'w, 's> {
	mouse: Res<'w, ButtonInput<MouseButton>>,
	keyboard: Res<'w, ButtonInput<KeyCode>>,
	ground_pointer: Res<'w, GroundPointer>,
	snapped_pointer: Res<'w, SnappedPointer>,
	typing: TextFocus<'w, 's>,
}

fn update_wall_arc(
	input: ArcInput,
	mode: Res<ArcMode>,
	settings: Res<WallDrawingSettings>,
	mut house: ResMut<House>,
	mut notice: ResMut<EditNotice>,
	mut draft: ResMut<ArcDraft>,
	mut preview: ResMut<EditPreview>,
) {
	// Escape belongs to a focused text field, which reverts its own edit.
	if input.keyboard.just_pressed(KeyCode::Escape) && !input.typing.active() {
		draft.set_if_neq(ArcDraft::Idle);
		if preview.holds_arc_proposal() {
			preview.state = None;
		}
		return;
	}

	let (next_draft, confirmed) = advance_draft(*draft, *mode, &input, house.graph(), &mut notice);
	draft.set_if_neq(next_draft);

	let proposal = arc_proposal(next_draft, *mode, &input, &house, &settings);
	match proposal {
		Some(proposal) => {
			// Reborrowing retains Bevy's change tracking until the helper actually writes.
			EditPreview::update(
				preview.reborrow(),
				Proposal::Arc(proposal),
				house.revision(),
				house.committed(),
			);
		}
		// Keep the last preview while the pointer is off the ground.
		None if (input.snapped_pointer.target.is_some() || next_draft == ArcDraft::Idle)
			&& preview.holds_arc_proposal() =>
		{
			preview.state = None;
		}
		None => {}
	}

	if confirmed
		&& let Some(proposal) = proposal
		&& let Some(candidate) = preview.take_valid(Proposal::Arc(proposal), house.revision())
	{
		house.commit(candidate);
		notice.message = None;
		*draft = ArcDraft::Idle;
	}
}

/// Leaving the tool abandons an unfinished arc instead of resuming it on return.
fn cancel_wall_arc(mut draft: ResMut<ArcDraft>, mut preview: ResMut<EditPreview>) {
	draft.set_if_neq(ArcDraft::Idle);
	if preview.holds_arc_proposal() {
		preview.state = None;
	}
}

fn restart_when_mode_changes(mut draft: ResMut<ArcDraft>, mut preview: ResMut<EditPreview>) {
	draft.set_if_neq(ArcDraft::Idle);
	if preview.holds_arc_proposal() {
		preview.state = None;
	}
}

/// An arc started on the house before an undo or redo would end on a different house.
fn discard_arc_draft_after_restore(
	mut restored: MessageReader<DocumentRestored>,
	mut draft: ResMut<ArcDraft>,
) {
	if restored.read().count() > 0 {
		draft.set_if_neq(ArcDraft::Idle);
	}
}

/// The next draft state, and whether this click asks to commit the preview.
fn advance_draft(
	draft: ArcDraft,
	mode: ArcMode,
	input: &ArcInput,
	graph: &WallGraph,
	notice: &mut EditNotice,
) -> (ArcDraft, bool) {
	let pressed = input.mouse.just_pressed(MouseButton::Left) && input.ground_pointer.hit.is_some();
	let target = input.snapped_pointer.target;
	match (draft, mode) {
		(ArcDraft::Idle, ArcMode::Free) => {
			let started = input
				.mouse
				.just_pressed(MouseButton::Left)
				.then(|| Some((input.ground_pointer.hit?, target?)))
				.flatten();
			match started {
				Some((hit, target)) => (
					ArcDraft::Pressed {
						start: target.position,
						press_position: hit.viewport_position,
						dragged: false,
					},
					false,
				),
				None => (draft, false),
			}
		}
		(ArcDraft::Idle, _) => {
			let (true, Some(target)) = (pressed, target) else {
				return (draft, false);
			};
			let direction = match target.source {
				SnapSource::Junction(node) => free_end_continuation(graph, node),
				_ => None,
			};
			match direction {
				Some(direction) => {
					notice.message = None;
					(
						ArcDraft::AwaitingEnd {
							start: target.position,
							start_direction: Some(direction),
						},
						false,
					)
				}
				None => {
					notice.message = Some("Start on a free wall end to continue it".to_string());
					(draft, false)
				}
			}
		}
		(
			ArcDraft::Pressed {
				start,
				press_position,
				dragged,
			},
			_,
		) => {
			let dragged = dragged
				|| input.ground_pointer.hit.is_some_and(|hit| {
					hit.viewport_position.distance(press_position) > DRAG_THRESHOLD_PIXELS
				});
			// The release is handled even when the pointer is over the panel, so a drag
			// that ends there still finishes.
			if !input.mouse.just_released(MouseButton::Left) {
				return (
					ArcDraft::Pressed {
						start,
						press_position,
						dragged,
					},
					false,
				);
			}
			match (dragged, target) {
				(true, Some(target)) => (bend_stage(start, target.position), false),
				_ => (
					ArcDraft::AwaitingEnd {
						start,
						start_direction: None,
					},
					false,
				),
			}
		}
		(ArcDraft::AwaitingEnd { start, .. }, ArcMode::Free) => match (pressed, target) {
			(true, Some(target)) => (bend_stage(start, target.position), false),
			_ => (draft, false),
		},
		(ArcDraft::AwaitingEnd { .. }, _) => (draft, pressed),
		(
			ArcDraft::AwaitingBend {
				start,
				end,
				bend,
				typed_sweep,
			},
			_,
		) => {
			// A typed sweep ignores the pointer, so moving it must not refresh the draft.
			let bend = match (typed_sweep, target) {
				(None, Some(target)) => target.position,
				_ => bend,
			};
			(
				ArcDraft::AwaitingBend {
					start,
					end,
					bend,
					typed_sweep,
				},
				pressed,
			)
		}
	}
}

/// Endpoints chosen; until the pointer moves the bend sits on the chord's midpoint.
fn bend_stage(start: Vec2, end: Vec2) -> ArcDraft {
	ArcDraft::AwaitingBend {
		start,
		end,
		bend: (start + end) / 2.0,
		typed_sweep: None,
	}
}

/// The arc the draft would draw, or `None` while it has too little to propose.
fn arc_proposal(
	draft: ArcDraft,
	mode: ArcMode,
	input: &ArcInput,
	house: &House,
	settings: &WallDrawingSettings,
) -> Option<ArcProposal> {
	let dimensions = settings.dimensions();
	match draft {
		ArcDraft::AwaitingBend {
			start,
			end,
			bend,
			typed_sweep,
		} => Some(ArcProposal {
			start,
			end,
			definition: typed_sweep.map_or(ArcDefinition::Bend(bend), ArcDefinition::Sweep),
			dimensions,
		}),
		ArcDraft::AwaitingEnd {
			start,
			start_direction: Some(start_direction),
		} => {
			let target = input.snapped_pointer.target?;
			let definition = match mode {
				ArcMode::TwoTangents => ArcDefinition::TwoTangents {
					start_direction,
					end_direction: match target.source {
						SnapSource::Junction(node) => {
							free_end_continuation(house.graph(), node).map(|direction| -direction)
						}
						_ => None,
					},
				},
				_ => ArcDefinition::OneTangent { start_direction },
			};
			Some(ArcProposal {
				start,
				end: target.position,
				definition,
				dimensions,
			})
		}
		_ => None,
	}
}

/// Shows what the arc is built from: the chord, the bend, and the wall ends it continues.
fn draw_arc_guides(draft: Res<ArcDraft>, snapped_pointer: Res<SnappedPointer>, mut gizmos: Gizmos) {
	let to_bevy = |position: Vec2| graph_to_bevy(position.extend(GUIDE_HEIGHT_METRES));
	let marker = |position: Vec2, gizmos: &mut Gizmos| {
		gizmos.sphere(
			Isometry3d::from_translation(to_bevy(position)),
			GUIDE_MARKER_RADIUS_METRES,
			GUIDE_COLOR,
		);
	};
	match *draft {
		ArcDraft::Idle => {}
		ArcDraft::Pressed { start, .. } => marker(start, &mut gizmos),
		ArcDraft::AwaitingEnd {
			start,
			start_direction,
		} => {
			marker(start, &mut gizmos);
			let guide_end = start_direction.map_or_else(
				|| snapped_pointer.target.map(|target| target.position),
				|direction| Some(start + direction * DIRECTION_GUIDE_LENGTH_METRES),
			);
			if let Some(end) = guide_end {
				gizmos.line(to_bevy(start), to_bevy(end), GUIDE_COLOR);
			}
		}
		ArcDraft::AwaitingBend {
			start, end, bend, ..
		} => {
			marker(start, &mut gizmos);
			marker(end, &mut gizmos);
			gizmos.line(to_bevy(start), to_bevy(end), GUIDE_COLOR);
			marker(bend, &mut gizmos);
		}
	}
}

#[cfg(test)]
#[path = "tests/wall_tool_arc.rs"]
mod tests;
