use bevy::{input_focus::InputFocus, prelude::*, text::EditableText};
use wall_graph::{Wall, WallDimensions};

use crate::{
	edit::{EditNotice, EditPreview, PreviewState, Proposal, WallDimensionsProposal},
	house::House,
	text_fields::{FocusedField, field_row, field_text, parse_field_value, show_in_field},
	wall_tool_selection::{SelectedTarget, WallSelectionCarryOver},
};

/// Typed height and thickness for the selected wall, previewed while typing.
pub struct WallFieldsPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WallFieldsSystems;

impl Plugin for WallFieldsPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<EditPreview>()
			.init_resource::<EditNotice>()
			.add_systems(
				PostUpdate,
				(preview_typed_value, submit_field, show_values)
					.chain()
					.in_set(WallFieldsSystems),
			);
	}
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum WallField {
	Height,
	Thickness,
}

impl WallField {
	fn label(self) -> &'static str {
		match self {
			Self::Height => "Wall height (m)",
			Self::Thickness => "Wall thickness (m)",
		}
	}

	fn read(self, dimensions: &WallDimensions) -> f64 {
		match self {
			Self::Height => f64::from(dimensions.height),
			Self::Thickness => f64::from(dimensions.thickness),
		}
	}

	fn write(self, dimensions: WallDimensions, value: f64) -> WallDimensions {
		match self {
			Self::Height => WallDimensions {
				height: value as f32,
				..dimensions
			},
			Self::Thickness => WallDimensions {
				thickness: value as f32,
				..dimensions
			},
		}
	}
}

pub fn wall_field_controls() -> impl Bundle {
	(
		Node {
			flex_direction: FlexDirection::Column,
			row_gap: Val::Px(6.0),
			..default()
		},
		children![
			Text::new("Selected wall: type to preview, Enter applies"),
			field_row(WallField::Height.label(), WallField::Height),
			field_row(WallField::Thickness.label(), WallField::Thickness),
		],
	)
}

/// The dimensions an edit builds on: the draft when one is pending, otherwise the wall's own.
fn base_dimensions(house: &House, preview: &EditPreview, wall: Wall) -> Option<WallDimensions> {
	match &preview.state {
		Some(PreviewState {
			proposal: Proposal::WallDimensions(edit),
			..
		}) if edit.wall == wall => Some(edit.dimensions),
		_ => house.graph().wall_dimensions(wall),
	}
}

/// Previews whatever is typed, so the new mesh is visible before it is applied. Text that is
/// not a number yet, such as a lone minus sign, leaves the preview as it was.
fn preview_typed_value(
	focused: FocusedField<WallField>,
	selected: Res<SelectedTarget>,
	house: Res<House>,
	mut preview: ResMut<EditPreview>,
) {
	let (Some((field, text)), Some(wall)) = (focused.get(), selected.wall()) else {
		return;
	};
	let (Ok(value), Some(base), Some(committed)) = (
		text.trim().parse::<f64>(),
		base_dimensions(&house, &preview, wall),
		house.graph().wall_dimensions(wall),
	) else {
		return;
	};
	let edit = WallDimensionsProposal {
		wall,
		dimensions: field.write(base, value),
	};

	// Typing the wall's current value back means there is nothing to preview.
	if edit.dimensions == committed {
		if preview.holds_wall_dimensions() {
			preview.state = None;
		}
		return;
	}
	EditPreview::update(
		preview.reborrow(),
		Proposal::WallDimensions(edit),
		house.revision(),
		house.committed(),
	);
}

/// Enter applies the previewed dimensions. A rejected preview stays, with its reason.
fn submit_field(
	keyboard: Res<ButtonInput<KeyCode>>,
	focused: FocusedField<WallField>,
	selected: Res<SelectedTarget>,
	mut house: ResMut<House>,
	mut preview: ResMut<EditPreview>,
	mut carry_over: ResMut<WallSelectionCarryOver>,
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
	let Some(wall) = selected.wall() else {
		return;
	};
	let Some(dimensions) = base_dimensions(&house, &preview, wall) else {
		return;
	};
	let proposal = Proposal::WallDimensions(WallDimensionsProposal {
		wall,
		dimensions: field.write(dimensions, value),
	});
	if let Some(candidate) = preview.take_valid(proposal, house.revision()) {
		carry_over.0 = Some(wall);
		house.commit(candidate);
		notice.message = None;
	}
}

/// Shows the draft, or the selected wall, in every field that is not being typed into.
fn show_values(
	focus: Option<Res<InputFocus>>,
	house: Res<House>,
	preview: Res<EditPreview>,
	selected: Res<SelectedTarget>,
	mut fields: Query<(Entity, &WallField, &mut EditableText)>,
) {
	let focused = focus.and_then(|focus| focus.get());
	let dimensions = selected
		.wall()
		.and_then(|wall| base_dimensions(&house, &preview, wall));
	for (entity, &field, mut text) in &mut fields {
		if Some(entity) == focused {
			continue;
		}
		let wanted = dimensions
			.as_ref()
			.map(|dimensions| field_text(field.read(dimensions)))
			.unwrap_or_default();
		show_in_field(&mut text, &wanted);
	}
}

#[cfg(test)]
#[path = "tests/wall_fields.rs"]
mod tests;
