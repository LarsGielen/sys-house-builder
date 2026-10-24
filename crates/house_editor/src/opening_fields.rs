use bevy::{input_focus::InputFocus, prelude::*, text::EditableText};
use wall_graph::OpeningSpec;

use crate::{
	edit::{EditNotice, EditPreview, OpeningEditProposal, PreviewState, Proposal},
	house::House,
	snapping::SnapSettings,
	text_fields::{FocusedField, field_row, field_text, parse_field_value, show_in_field},
	wall_tool_selection::SelectedTarget,
};

/// Typed properties of the selected opening, plus the independent snapping-gap control.
pub struct OpeningFieldsPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpeningFieldsSystems;

impl Plugin for OpeningFieldsPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<EditPreview>()
			.init_resource::<EditNotice>()
			.add_systems(
				PostUpdate,
				(submit_field, submit_snap_gap, show_values, show_snap_gap)
					.chain()
					.in_set(OpeningFieldsSystems),
			);
	}
}

/// What a text field edits. The opening fields apply to the selected opening.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum OpeningField {
	/// Distance of the centre along the host wall.
	CenterDistance,
	Width,
	Height,
	Bottom,
}

impl OpeningField {
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

	fn write(self, spec: OpeningSpec, value: f64) -> OpeningSpec {
		match self {
			Self::CenterDistance => OpeningSpec {
				center_distance: value,
				..spec
			},
			Self::Width => OpeningSpec {
				width: value as f32,
				..spec
			},
			Self::Height => OpeningSpec {
				height: value as f32,
				..spec
			},
			Self::Bottom => OpeningSpec {
				bottom: value as f32,
				..spec
			},
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
			Text::new("Selected opening: Enter applies, Esc reverts"),
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

fn submit_field(
	keyboard: Res<ButtonInput<KeyCode>>,
	focused: FocusedField<OpeningField>,
	selected: Res<SelectedTarget>,
	mut house: ResMut<House>,
	mut preview: ResMut<EditPreview>,
	mut notice: ResMut<EditNotice>,
) {
	if !keyboard.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter]) {
		return;
	}
	let Some((field, text)) = focused.get() else {
		return;
	};
	let value = match parse_field_value(&text, field.label()) {
		Ok(value) => value,
		Err(error) => {
			notice.message = Some(error);
			return;
		}
	};
	notice.message = None;

	let Some(opening) = selected.opening() else {
		return;
	};
	let Some(base) = base_spec(&house, &preview, opening) else {
		return;
	};
	let edit = OpeningEditProposal {
		opening,
		spec: field.write(base, value),
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
	// Invalid proposals stay as drafts so another property can be changed to repair them.
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
	keyboard: Res<ButtonInput<KeyCode>>,
	focused: FocusedField<SnapGapField>,
	mut settings: ResMut<SnapSettings>,
	mut notice: ResMut<EditNotice>,
) {
	if !keyboard.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter]) {
		return;
	}
	let Some((_, text)) = focused.get() else {
		return;
	};
	let result = parse_field_value(&text, "Opening gap (m)").and_then(|value| {
		settings
			.set_opening_gap(value as f32)
			.map_err(|error| error.to_string())
	});
	notice.message = result.err();
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
#[path = "tests/opening_fields.rs"]
mod tests;
