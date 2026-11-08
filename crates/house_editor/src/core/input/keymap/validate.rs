use std::collections::BTreeMap;

use crate::core::input::{
	ActionState, AxisBinding, ButtonBinding, Scale, Shortcut, ValueKind,
	interpretation::Interpretation,
	keymap::file::{Entry, ParsedEntry},
};

/// Parses one context's bindings. A shortcut, or a button of a composite, may be used once per
/// context: two contexts may bind the same input, because the higher one then shadows the lower.
pub(super) fn parse_context_bindings(
	context: &'static str,
	bindings: BTreeMap<String, Vec<Entry>>,
	actions: &ActionState,
) -> Result<(Vec<ButtonBinding>, Vec<AxisBinding>), String> {
	let mut button_bindings: Vec<ButtonBinding> = Vec::new();
	let mut axis_bindings: Vec<AxisBinding> = Vec::new();

	for (action_name, entries) in bindings {
		let action = actions
			.registered_name(&action_name)
			.ok_or_else(|| format!("unknown action \"{action_name}\""))?;

		for entry in &entries {
			let parsed = entry
				.parse()
				.map_err(|error| format!("action \"{action_name}\": {error}"))?;

			match parsed {
				ParsedEntry::Axis { input, modifiers } => {
					if actions.value_kind_of(action) != Some(input.value_kind()) {
						return Err(format!(
							"action \"{action_name}\" cannot be bound to \"{input}\": the kinds do not match"
						));
					}

					if entry.interpret().is_some() {
						return Err(format!(
							"action \"{action_name}\": only a shortcut can have `interpret`, not \"{input}\""
						));
					}

					if let Some(other) = axis_bindings
						.iter()
						.find(|other| other.input == input && other.modifiers == modifiers)
					{
						return Err(format!(
							"\"{modifiers}{input}\" is bound to both {} and {action}",
							other.action
						));
					}

					let scale = match entry.scale() {
						Some(scale) => scale
							.validate(input.value_kind())
							.map_err(|error| format!("action \"{action_name}\": {error}"))?,
						None => Scale::identity(input.value_kind()),
					};

					axis_bindings.push(AxisBinding {
						input,
						modifiers,
						scale,
						action,
						context,
					});
				}
				ParsedEntry::Shortcut(shortcut) => {
					if entry.scale().is_some() {
						return Err(format!(
							"action \"{action_name}\": a shortcut cannot have a scale (\"{shortcut}\")"
						));
					}

					// A shortcut is a button press, so it can drive a button or flip a toggle.
					if !matches!(
						actions.value_kind_of(action),
						Some(ValueKind::Button | ValueKind::Toggle)
					) {
						return Err(format!(
							"action \"{action_name}\" is not a button, so it cannot be bound to the shortcut \"{shortcut}\""
						));
					}

					let interpretation = match entry.interpret() {
						Some(text) => text
							.parse()
							.map_err(|error| format!("action \"{action_name}\": {error}"))?,
						None => Interpretation::default(),
					};

					if let Some(other) = button_bindings
						.iter()
						.find(|existing| existing.shortcut == shortcut)
					{
						return Err(format!(
							"shortcut \"{shortcut}\" is bound to both {} and {action}",
							other.action
						));
					}

					button_bindings.push(ButtonBinding {
						shortcut,
						action,
						context,
						interpretation,
					});
				}
			}
		}
	}

	check_direction_conflicts(&button_bindings, &axis_bindings)?;
	Ok((button_bindings, axis_bindings))
}

/// Within a context, a button with its modifiers may be used by one binding only. Otherwise one press would drive
/// two actions, such as `s` saving while it also moves the camera back. Equal shortcuts and equal
/// composites are already rejected while parsing, so only the directions of composites remain.
fn check_direction_conflicts(
	button_bindings: &[ButtonBinding],
	axis_bindings: &[AxisBinding],
) -> Result<(), String> {
	for (index, binding) in axis_bindings.iter().enumerate() {
		for direction in binding.input.directions() {
			let claimed = Shortcut::new(direction, binding.modifiers);

			if let Some(other) = button_bindings
				.iter()
				.find(|other| other.shortcut == claimed)
			{
				return Err(format!(
					"\"{claimed}\" is used by both {} and the composite of {}",
					other.action, binding.action
				));
			}

			for other in &axis_bindings[index + 1..] {
				if other.modifiers == binding.modifiers
					&& other.input.directions().contains(&direction)
				{
					return Err(format!(
						"\"{claimed}\" is used by the composites of both {} and {}",
						binding.action, other.action
					));
				}
			}
		}
	}
	Ok(())
}
