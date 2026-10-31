use bevy::{
	ecs::system::SystemParam,
	input_focus::InputFocus,
	prelude::*,
	text::{EditableText, EditableTextFilter, TextCursorStyle},
	ui_widgets::SelectAllOnFocus,
};

use crate::{
	actions::ActionSender,
	actions::property::{PropertyAction, PropertyActionPlugin, discard_cancelled_draft},
	input::{Action, ActionState, ActionsPlugin},
	ui::theme,
	viewport::ground_pointer::GroundPointer,
};

/// What every typed numeric field shares: focus handling, leaving a field, and its look.
pub struct TextFieldsPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextFieldsSystems;

impl Plugin for TextFieldsPlugin {
	fn build(&self, app: &mut App) {
		ActionsPlugin::ensure(app);
		PropertyActionPlugin::ensure(app);
		app.add_systems(
			PostUpdate,
			(
				blur_on_viewport_click,
				revert_on_escape,
				discard_cancelled_draft,
			)
				.chain()
				.in_set(TextFieldsSystems),
		);
	}
}

/// Whether a text field has keyboard focus. Key presses then belong to the field, so tools
/// must not treat them as shortcuts.
#[derive(SystemParam)]
pub struct TextFocus<'w, 's> {
	// Absent in apps without the input focus plugin, where nothing can be typed into.
	focus: Option<Res<'w, InputFocus>>,
	fields: Query<'w, 's, (), With<EditableText>>,
}

impl TextFocus<'_, '_> {
	pub fn active(&self) -> bool {
		self.focus
			.as_ref()
			.and_then(|focus| focus.get())
			.is_some_and(|entity| self.fields.contains(entity))
	}
}

/// The text field being typed into, if it is a field of kind `F`, with its text.
#[derive(SystemParam)]
pub struct FocusedField<'w, 's, F: Component + Copy> {
	focus: Option<Res<'w, InputFocus>>,
	fields: Query<'w, 's, (&'static F, &'static EditableText)>,
}

impl<F: Component + Copy> FocusedField<'_, '_, F> {
	pub fn get(&self) -> Option<(F, String)> {
		let entity = self.focus.as_ref()?.get()?;
		let (&field, text) = self.fields.get(entity).ok()?;
		Some((field, text.value().to_string()))
	}
}

/// A labelled field, with `field` identifying what it edits.
pub fn field_row<F: Component>(label: &str, field: F) -> impl Bundle {
	(
		Node {
			flex_direction: FlexDirection::Row,
			align_items: AlignItems::Center,
			justify_content: JustifyContent::SpaceBetween,
			column_gap: Val::Px(8.0),
			..default()
		},
		children![Text::new(label.to_string()), text_field(field)],
	)
}

/// A wide field for free text, which accepts any character.
pub fn wide_text_field<F: Component>(field: F) -> impl Bundle {
	(
		field,
		Node {
			width: Val::Px(190.0),
			padding: UiRect::axes(Val::Px(6.0), Val::Px(2.0)),
			border: UiRect::all(Val::Px(1.0)),
			overflow: Overflow::clip(),
			..default()
		},
		BorderColor::from(theme::PANEL_BORDER),
		BackgroundColor(theme::FIELD),
		EditableText {
			visible_width: Some(28.0),
			allow_newlines: false,
			..default()
		},
		TextLayout::no_wrap(),
		TextCursorStyle::default(),
		SelectAllOnFocus,
	)
}

fn text_field<F: Component>(field: F) -> impl Bundle {
	(
		field,
		Node {
			width: Val::Px(80.0),
			padding: UiRect::axes(Val::Px(6.0), Val::Px(2.0)),
			border: UiRect::all(Val::Px(1.0)),
			..default()
		},
		BorderColor::from(theme::PANEL_BORDER),
		BackgroundColor(theme::FIELD),
		EditableText {
			visible_width: Some(7.0),
			allow_newlines: false,
			..default()
		},
		EditableTextFilter::new(|character| {
			character.is_ascii_digit() || matches!(character, '.' | '-')
		}),
		TextLayout::no_wrap(),
		TextCursorStyle::default(),
		SelectAllOnFocus,
	)
}

pub fn parse_field_value(text: &str, label: &str) -> Result<f64, String> {
	match text.trim().parse::<f64>() {
		Ok(value) if value.is_finite() => Ok(value),
		_ => Err(format!("{label} is not a finite number")),
	}
}

/// Text for a field showing `value`.
pub fn field_text(value: f64) -> String {
	format!("{value:.3}")
}

/// Replaces a field's text, unless it already shows `wanted`.
pub fn show_in_field(text: &mut Mut<EditableText>, wanted: &str) {
	// Setting identical text would still trigger a text layout every frame.
	if text.value() != wanted {
		text.editor_mut().set_text(wanted);
	}
}

/// A click in the viewport ends typing, so keys go back to the tools.
fn blur_on_viewport_click(
	actions: Res<ActionState>,
	ground_pointer: Res<GroundPointer>,
	focus: Option<ResMut<InputFocus>>,
) {
	let Some(mut focus) = focus else {
		return;
	};
	if actions.just_pressed(Action::ToolPress)
		&& ground_pointer.ray.is_some()
		&& focus.get().is_some()
	{
		focus.clear();
	}
}

/// Cancelling in a field leaves it. The draft it was building is dropped by whoever owns it.
fn revert_on_escape(
	actions: Res<ActionState>,
	focus: Option<ResMut<InputFocus>>,
	mut properties: ActionSender,
) {
	let Some(mut focus) = focus else {
		return;
	};
	if actions.just_pressed(Action::FieldCancel) {
		focus.clear();
		properties.send(PropertyAction::CancelDraft);
	}
}
