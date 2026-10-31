use bevy::{input_focus::InputFocus, prelude::*, text::EditableText};
use wall_graph::{Wall, WallDimensions};

use crate::{
	actions::ActionSender,
	actions::EditorAction,
	actions::property::{PropertyAction, PropertyActionPlugin, WallProperty},
	document::edit::{EditNotice, EditPreview, PreviewState, Proposal, WallDimensionsProposal},
	document::house::House,
	input::hints::hint,
	input::{Action, ActionState, ActionsPlugin},
	tools::selection::{SelectedTarget, WallSelectionCarryOver},
	ui::fields::{FocusedField, field_row, field_text, parse_field_value, show_in_field},
};

/// Typed height and thickness for the selected wall, previewed while typing.
pub struct WallFieldsPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WallFieldsSystems;

impl Plugin for WallFieldsPlugin {
	fn build(&self, app: &mut App) {
		ActionsPlugin::ensure(app);
		PropertyActionPlugin::ensure(app);
		app.init_resource::<EditPreview>()
			.init_resource::<EditNotice>()
			.add_systems(
				PostUpdate,
				(send_typed_value, apply_property_actions, show_values)
					.chain()
					.in_set(WallFieldsSystems),
			);
	}
}

type WallField = WallProperty;

impl WallProperty {
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
}

pub fn wall_field_controls() -> impl Bundle {
	(
		Node {
			flex_direction: FlexDirection::Column,
			row_gap: Val::Px(6.0),
			..default()
		},
		children![
			hint("Selected wall: type to preview, {FieldSubmit} applies"),
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

/// Sends what is typed as a preview, so the new mesh is visible before it is applied, and
/// Enter as a submit of that value. Text that is not a number yet, such as a lone minus sign,
/// sends nothing.
fn send_typed_value(
	actions: Res<ActionState>,
	focused: FocusedField<WallField>,
	selected: Res<SelectedTarget>,
	house: Res<House>,
	mut properties: ActionSender,
	mut notice: ResMut<EditNotice>,
) {
	let (Some((property, text)), Some(wall)) = (focused.get(), selected.wall()) else {
		return;
	};
	if actions.just_pressed(Action::FieldSubmit) {
		match parse_field_value(&text, property.label()) {
			Ok(value) => {
				properties.send(PropertyAction::SubmitWall {
					revision: house.revision(),
					wall,
					property,
					value,
				});
			}
			Err(error) => notice.message = Some(error),
		}
		return;
	}
	if let Ok(value) = text.trim().parse::<f64>() {
		properties.send(PropertyAction::PreviewWall {
			revision: house.revision(),
			wall,
			property,
			value,
		});
	}
}

/// Previews and applies wall property edits. A rejected preview stays, with its reason.
fn apply_property_actions(
	mut actions: MessageReader<EditorAction>,
	mut house: ResMut<House>,
	mut preview: ResMut<EditPreview>,
	mut carry_over: ResMut<WallSelectionCarryOver>,
	mut notice: ResMut<EditNotice>,
) {
	for action in actions.read().filter_map(|action| action.property()) {
		match action {
			PropertyAction::PreviewWall {
				revision,
				wall,
				property,
				value,
			} => {
				if revision != house.revision() {
					continue;
				}
				let (true, Some(base), Some(committed)) = (
					property.is_representable(value),
					base_dimensions(&house, &preview, wall),
					house.graph().wall_dimensions(wall),
				) else {
					continue;
				};
				let edit = WallDimensionsProposal {
					wall,
					dimensions: property.write(base, value),
				};
				// Typing the wall's current value back means there is nothing to preview.
				if edit.dimensions == committed {
					if preview.holds_wall_dimensions() {
						preview.state = None;
					}
					continue;
				}
				EditPreview::update(
					preview.reborrow(),
					Proposal::WallDimensions(edit),
					house.revision(),
					house.committed(),
				);
			}
			PropertyAction::SubmitWall {
				revision,
				wall,
				property,
				value,
			} => {
				if revision != house.revision() {
					notice.message =
						Some("The document changed; apply the value again".to_string());
					continue;
				}
				if !property.is_representable(value) {
					notice.message = Some(format!("{} is not a valid number", property.label()));
					continue;
				}
				let Some(dimensions) = base_dimensions(&house, &preview, wall) else {
					continue;
				};
				let proposal = Proposal::WallDimensions(WallDimensionsProposal {
					wall,
					dimensions: property.write(dimensions, value),
				});
				if let Some(candidate) = preview.take_valid(proposal, house.revision()) {
					carry_over.0 = Some(wall);
					house.commit(candidate);
					notice.message = None;
				}
			}
			_ => {}
		}
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
#[path = "../tests/ui/wall_fields.rs"]
mod tests;
