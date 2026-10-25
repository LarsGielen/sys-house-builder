use bevy::{input_focus::InputFocus, prelude::*, text::EditableText};
use house_editor_core::ArcDraft;
use wall_graph::{ArcError, arc_radius};

use crate::{
	actions::ActionSender,
	actions::EditorAction,
	actions::property::{ArcProperty, PropertyAction, PropertyActionPlugin},
	document::edit::{ArcDefinition, ArcProposal, EditNotice, EditPreview, PreviewState, Proposal},
	document::house::House,
	input::hints::hint,
	input::{Action, ActionState, ActionsPlugin},
	tools::arc::ArcDraftState,
	ui::fields::{FocusedField, field_row, field_text, parse_field_value, show_in_field},
};

/// Typed angle and radius for the arc being drawn. They control the same draft as the
/// pointer's bend, so a typed value replaces the bend until Escape drops it.
pub struct ArcFieldsPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArcFieldsSystems;

impl Plugin for ArcFieldsPlugin {
	fn build(&self, app: &mut App) {
		ActionsPlugin::ensure(app);
		PropertyActionPlugin::ensure(app);
		app.init_resource::<EditPreview>()
			.init_resource::<EditNotice>()
			.init_resource::<ArcDraftState>()
			.add_systems(
				PostUpdate,
				(send_typed_value, apply_property_actions, show_values)
					.chain()
					.in_set(ArcFieldsSystems),
			);
	}
}

type ArcField = ArcProperty;

impl ArcProperty {
	fn label(self) -> &'static str {
		match self {
			Self::Angle => "Arc angle (deg)",
			Self::Radius => "Arc radius (m)",
		}
	}
}

pub fn arc_field_controls() -> impl Bundle {
	(
		Node {
			flex_direction: FlexDirection::Column,
			row_gap: Val::Px(6.0),
			..default()
		},
		children![
			hint("Free arc: type to set the bend, {FieldSubmit} applies"),
			field_row(ArcField::Angle.label(), ArcField::Angle),
			field_row(ArcField::Radius.label(), ArcField::Radius),
		],
	)
}

/// The arc being previewed, once its endpoints are chosen.
fn previewed_arc(preview: &EditPreview) -> Option<ArcProposal> {
	match &preview.state {
		Some(PreviewState {
			proposal: Proposal::Arc(arc),
			..
		}) => Some(*arc),
		_ => None,
	}
}

/// The sweep a typed field value asks for, or why the arc cannot have it.
fn typed_sweep_for(property: ArcProperty, value: f64, arc: &ArcProposal) -> Result<f32, String> {
	if !(value as f32).is_finite() {
		return Err(format!("{} is not a valid number", property.label()));
	}
	match property {
		ArcProperty::Angle => Ok((value as f32).to_radians()),
		ArcProperty::Radius => arc
			.sweep_for_radius(value as f32)
			.map_err(|error: ArcError| error.to_string()),
	}
}

/// The arc between `start` and `end` as previewed, if that is the draft the action names.
fn previewed_arc_between(preview: &EditPreview, start: Vec2, end: Vec2) -> Option<ArcProposal> {
	previewed_arc(preview).filter(|arc| arc.start == start && arc.end == end)
}

/// Sends what is typed as a preview and Enter as a submit of that value. Text that is not a
/// number yet, such as a lone minus sign, sends nothing.
fn send_typed_value(
	actions: Res<ActionState>,
	focused: FocusedField<ArcField>,
	draft: Res<ArcDraftState>,
	house: Res<House>,
	mut properties: ActionSender,
	mut notice: ResMut<EditNotice>,
) {
	let (Some((property, text)), ArcDraft::AwaitingBend { start, end, .. }) =
		(focused.get(), draft.0)
	else {
		return;
	};
	if actions.just_pressed(Action::FieldSubmit) {
		match parse_field_value(&text, property.label()) {
			Ok(value) => {
				properties.send(PropertyAction::SubmitArc {
					revision: house.revision(),
					start,
					end,
					property,
					value,
				});
			}
			Err(error) => notice.message = Some(error),
		}
	} else if let Ok(value) = text.trim().parse::<f64>() {
		properties.send(PropertyAction::PreviewArc {
			revision: house.revision(),
			start,
			end,
			property,
			value,
		});
	}
}

/// Previews and applies typed arc values, and drops a typed value when its field is cancelled.
/// A rejected value keeps the older preview visible but never confirms it.
fn apply_property_actions(
	mut actions: MessageReader<EditorAction>,
	mut house: ResMut<House>,
	mut draft: ResMut<ArcDraftState>,
	mut preview: ResMut<EditPreview>,
	mut notice: ResMut<EditNotice>,
) {
	for action in actions.read().filter_map(|action| action.property()) {
		match action {
			PropertyAction::CancelDraft => {
				if let ArcDraft::AwaitingBend {
					start,
					end,
					bend,
					typed_sweep: Some(_),
				} = draft.0
				{
					draft.0 = ArcDraft::AwaitingBend {
						start,
						end,
						bend,
						typed_sweep: None,
					};
				}
			}
			PropertyAction::PreviewArc {
				revision,
				start,
				end,
				property,
				value,
			} => {
				if revision != house.revision() {
					continue;
				}
				let (
					Some(arc),
					ArcDraft::AwaitingBend {
						start: draft_start,
						end: draft_end,
						bend,
						typed_sweep,
					},
				) = (previewed_arc_between(&preview, start, end), draft.0)
				else {
					continue;
				};
				if (draft_start, draft_end) != (start, end) {
					continue;
				}
				let sweep = match typed_sweep_for(property, value, &arc) {
					Ok(sweep) => sweep,
					Err(error) => {
						notice.message = Some(error);
						continue;
					}
				};
				notice.message = None;
				if typed_sweep != Some(sweep) {
					draft.0 = ArcDraft::AwaitingBend {
						start,
						end,
						bend,
						typed_sweep: Some(sweep),
					};
				}
				EditPreview::update(
					preview.reborrow(),
					Proposal::Arc(ArcProposal {
						definition: ArcDefinition::Sweep(sweep),
						..arc
					}),
					house.revision(),
					house.committed(),
				);
			}
			PropertyAction::SubmitArc {
				revision,
				start,
				end,
				property,
				value,
			} => {
				if revision != house.revision() {
					notice.message =
						Some("The document changed; apply the value again".to_string());
					continue;
				}
				let Some(arc) = previewed_arc_between(&preview, start, end) else {
					continue;
				};
				// The previewed arc may predate this value, because a rejected value leaves it
				// in place. Only the arc this very value asks for may be confirmed.
				let sweep = match typed_sweep_for(property, value, &arc) {
					Ok(sweep) => sweep,
					Err(error) => {
						notice.message = Some(error);
						continue;
					}
				};
				let typed = Proposal::Arc(ArcProposal {
					definition: ArcDefinition::Sweep(sweep),
					..arc
				});
				if let Some(candidate) = preview.take_valid(typed, house.revision()) {
					house.commit(candidate);
					notice.message = None;
					draft.0 = ArcDraft::Idle;
				}
			}
			_ => {}
		}
	}
}

/// Shows the arc being drawn in every field that is not being typed into.
fn show_values(
	focus: Option<Res<InputFocus>>,
	preview: Res<EditPreview>,
	mut fields: Query<(Entity, &ArcField, &mut EditableText)>,
) {
	let focused = focus.and_then(|focus| focus.get());
	let derived = previewed_arc(&preview).and_then(|arc| {
		let sweep = arc.sweep().ok()?;
		Some((sweep, arc_radius(arc.start, arc.end, sweep)))
	});
	for (entity, &field, mut text) in &mut fields {
		if Some(entity) == focused {
			continue;
		}
		let wanted = match (derived, field) {
			(Some((sweep, _)), ArcField::Angle) => field_text(f64::from(sweep.to_degrees())),
			(Some((_, Some(radius))), ArcField::Radius) => field_text(f64::from(radius)),
			_ => String::new(),
		};
		show_in_field(&mut text, &wanted);
	}
}

#[cfg(test)]
#[path = "../tests/ui/arc_fields.rs"]
mod tests;
