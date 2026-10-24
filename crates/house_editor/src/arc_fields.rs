use bevy::{input_focus::InputFocus, prelude::*, text::EditableText};
use wall_graph::arc_radius;

use crate::{
	edit::{ArcDefinition, ArcProposal, EditNotice, EditPreview, PreviewState, Proposal},
	house::House,
	text_fields::{FocusedField, field_row, field_text, parse_field_value, show_in_field},
	wall_tool_arc::ArcDraft,
};

/// Typed angle and radius for the arc being drawn. They control the same draft as the
/// pointer's bend, so a typed value replaces the bend until Escape drops it.
pub struct ArcFieldsPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArcFieldsSystems;

impl Plugin for ArcFieldsPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<EditPreview>()
			.init_resource::<EditNotice>()
			.init_resource::<ArcDraft>()
			.add_systems(
				PostUpdate,
				(
					drop_typed_sweep_on_escape,
					preview_typed_value,
					submit_field,
					show_values,
				)
					.chain()
					.in_set(ArcFieldsSystems),
			);
	}
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum ArcField {
	Angle,
	Radius,
}

impl ArcField {
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
			Text::new("Free arc: type to set the bend, Enter applies"),
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

/// Escape in a field blurs it before this runs; the arc then returns to the pointer's bend.
fn drop_typed_sweep_on_escape(keyboard: Res<ButtonInput<KeyCode>>, mut draft: ResMut<ArcDraft>) {
	if !keyboard.just_pressed(KeyCode::Escape) {
		return;
	}
	if let ArcDraft::AwaitingBend {
		start,
		end,
		bend,
		typed_sweep: Some(_),
	} = *draft
	{
		*draft = ArcDraft::AwaitingBend {
			start,
			end,
			bend,
			typed_sweep: None,
		};
	}
}

/// Previews whatever is typed. Text that is not a number yet, such as a lone minus sign,
/// leaves the preview as it was.
fn preview_typed_value(
	focused: FocusedField<ArcField>,
	house: Res<House>,
	mut draft: ResMut<ArcDraft>,
	mut preview: ResMut<EditPreview>,
	mut notice: ResMut<EditNotice>,
) {
	let (Some((field, text)), Some(arc)) = (focused.get(), previewed_arc(&preview)) else {
		return;
	};
	let ArcDraft::AwaitingBend {
		start,
		end,
		bend,
		typed_sweep,
	} = *draft
	else {
		return;
	};
	let Ok(value) = text.trim().parse::<f64>() else {
		return;
	};
	let sweep = match field {
		ArcField::Angle => Ok((value as f32).to_radians()),
		ArcField::Radius => arc.sweep_for_radius(value as f32),
	};
	let sweep = match sweep {
		Ok(sweep) => sweep,
		Err(error) => {
			notice.message = Some(error.to_string());
			return;
		}
	};
	notice.message = None;
	if typed_sweep != Some(sweep) {
		*draft = ArcDraft::AwaitingBend {
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

/// Enter applies the previewed arc. A rejected preview stays, with its reason.
fn submit_field(
	keyboard: Res<ButtonInput<KeyCode>>,
	focused: FocusedField<ArcField>,
	mut house: ResMut<House>,
	mut draft: ResMut<ArcDraft>,
	mut preview: ResMut<EditPreview>,
	mut notice: ResMut<EditNotice>,
) {
	if !keyboard.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter]) {
		return;
	}
	let Some((field, text)) = focused.get() else {
		return;
	};
	if let Err(error) = parse_field_value(&text, field.label()) {
		notice.message = Some(error);
		return;
	}
	let Some(arc) = previewed_arc(&preview) else {
		return;
	};
	if let Some(candidate) = preview.take_valid(Proposal::Arc(arc), house.revision()) {
		house.commit(candidate);
		notice.message = None;
		*draft = ArcDraft::Idle;
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
#[path = "tests/arc_fields.rs"]
mod tests;
