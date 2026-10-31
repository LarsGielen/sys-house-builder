use bevy::{input_focus::InputFocus, prelude::*, text::EditableText};
use wall_graph::OpeningSpec;

use crate::{
	actions::ActionSender,
	actions::EditorAction,
	actions::property::{OpeningProperty, PropertyAction, PropertyActionPlugin},
	actions::settings::{SettingsAction, SettingsActionPlugin},
	document::edit::{EditNotice, EditPreview, OpeningEditProposal, PreviewState, Proposal},
	document::house::House,
	input::hints::hint,
	input::{Action, ActionState, ActionsPlugin},
	tools::selection::SelectedTarget,
	ui::fields::{FocusedField, field_row, field_text, parse_field_value, show_in_field},
	viewport::snapping::SnapSettings,
};

/// Typed properties of the selected opening, plus the independent snapping-gap control.
pub struct OpeningFieldsPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpeningFieldsSystems;

impl Plugin for OpeningFieldsPlugin {
	fn build(&self, app: &mut App) {
		ActionsPlugin::ensure(app);
		PropertyActionPlugin::ensure(app);
		SettingsActionPlugin::ensure(app);
		app.init_resource::<EditPreview>()
			.init_resource::<EditNotice>()
			.add_systems(
				PostUpdate,
				(
					send_submitted_value,
					apply_property_actions,
					submit_snap_gap,
					show_values,
					show_snap_gap,
				)
					.chain()
					.in_set(OpeningFieldsSystems),
			);
	}
}

type OpeningField = OpeningProperty;

impl OpeningProperty {
	fn label(self) -> &'static str {
		match self {
			Self::CenterDistance => "Position (m)",
			Self::Width => "Width (m)",
			Self::Height => "Height (m)",
			Self::Bottom => "Bottom (m)",
		}
	}

	fn read(self, spec: &OpeningSpec) -> f64 {
		match self {
			Self::CenterDistance => spec.center_distance,
			Self::Width => f64::from(spec.width),
			Self::Height => f64::from(spec.height),
			Self::Bottom => f64::from(spec.bottom),
		}
	}
}

pub fn opening_field_controls() -> impl Bundle {
	(
		Node {
			flex_direction: FlexDirection::Column,
			row_gap: Val::Px(6.0),
			..default()
		},
		children![
			hint("Selected opening: {FieldSubmit} applies, {FieldCancel} reverts"),
			field_row(
				OpeningField::CenterDistance.label(),
				OpeningField::CenterDistance
			),
			field_row(OpeningField::Width.label(), OpeningField::Width),
			field_row(OpeningField::Height.label(), OpeningField::Height),
			field_row(OpeningField::Bottom.label(), OpeningField::Bottom),
		],
	)
}

/// The spec an edit builds on: the draft when one is pending, so a rejected value can be
/// fixed by changing another field, otherwise the committed opening.
fn base_spec(
	house: &House,
	preview: &EditPreview,
	opening: wall_graph::OpeningId,
) -> Option<OpeningSpec> {
	match &preview.state {
		Some(PreviewState {
			proposal: Proposal::OpeningEdit(edit),
			..
		}) if edit.opening == opening => Some(edit.spec),
		_ => house.graph().opening(opening).map(|opening| opening.spec),
	}
}

/// Sends Enter in an opening field as a submit of the value it holds.
fn send_submitted_value(
	actions: Res<ActionState>,
	focused: FocusedField<OpeningField>,
	selected: Res<SelectedTarget>,
	house: Res<House>,
	mut properties: ActionSender,
	mut notice: ResMut<EditNotice>,
) {
	if !actions.just_pressed(Action::FieldSubmit) {
		return;
	}
	let Some((property, text)) = focused.get() else {
		return;
	};
	let value = match parse_field_value(&text, property.label()) {
		Ok(value) => value,
		Err(error) => {
			notice.message = Some(error);
			return;
		}
	};
	notice.message = None;
	if let Some(opening) = selected.opening() {
		properties.send(PropertyAction::SubmitOpening {
			revision: house.revision(),
			opening,
			property,
			value,
		});
	}
}

/// Applies an opening property edit as one commit on the same opening. Invalid proposals stay
/// as drafts so another property can be changed to repair them.
fn apply_property_actions(
	mut actions: MessageReader<EditorAction>,
	mut house: ResMut<House>,
	mut preview: ResMut<EditPreview>,
	mut notice: ResMut<EditNotice>,
) {
	for action in actions.read().filter_map(|action| action.property()) {
		let PropertyAction::SubmitOpening {
			revision,
			opening,
			property,
			value,
		} = action
		else {
			continue;
		};
		if revision != house.revision() {
			notice.message = Some("The document changed; apply the value again".to_string());
			continue;
		}
		if !property.is_representable(value) {
			notice.message = Some(format!("{} is not a valid number", property.label()));
			continue;
		}
		let Some(base) = base_spec(&house, &preview, opening) else {
			continue;
		};
		let edit = OpeningEditProposal {
			opening,
			spec: property.write(base, value),
		};
		EditPreview::update(
			preview.reborrow(),
			Proposal::OpeningEdit(edit),
			house.revision(),
			house.committed(),
		);
		if let Some(candidate) = preview.take_valid(Proposal::OpeningEdit(edit), house.revision()) {
			house.commit(candidate);
			notice.message = None;
		}
	}
}

/// Shows the draft, or the selected opening, in every field that is not being typed into.
fn show_values(
	focus: Option<Res<InputFocus>>,
	house: Res<House>,
	preview: Res<EditPreview>,
	selected: Res<SelectedTarget>,
	mut fields: Query<(Entity, &OpeningField, &mut EditableText)>,
) {
	let focused = focus.and_then(|focus| focus.get());
	let spec = selected
		.opening()
		.and_then(|opening| base_spec(&house, &preview, opening));
	for (entity, &field, mut text) in &mut fields {
		if Some(entity) == focused {
			continue;
		}
		let shown = spec.as_ref().map(|spec| field.read(spec));
		show_in_field(&mut text, &shown.map(field_text).unwrap_or_default());
	}
}

#[derive(Component, Clone, Copy)]
struct SnapGapField;

pub fn snap_gap_control() -> impl Bundle {
	field_row("Opening gap (m)", SnapGapField)
}

fn submit_snap_gap(
	actions: Res<ActionState>,
	focused: FocusedField<SnapGapField>,
	mut requests: ActionSender,
	mut notice: ResMut<EditNotice>,
) {
	if !actions.just_pressed(Action::FieldSubmit) {
		return;
	}
	let Some((_, text)) = focused.get() else {
		return;
	};
	match parse_field_value(&text, "Opening gap (m)") {
		Ok(value) => {
			requests.send(SettingsAction::SetOpeningGap(value as f32));
		}
		Err(error) => notice.message = Some(error),
	}
}

fn show_snap_gap(
	focus: Option<Res<InputFocus>>,
	settings: Res<SnapSettings>,
	mut fields: Query<(Entity, &mut EditableText), With<SnapGapField>>,
) {
	let focused = focus.and_then(|focus| focus.get());
	for (entity, mut text) in &mut fields {
		if Some(entity) != focused {
			show_in_field(&mut text, &field_text(f64::from(settings.opening_gap())));
		}
	}
}

#[cfg(test)]
#[path = "../tests/ui/opening_fields.rs"]
mod tests;
