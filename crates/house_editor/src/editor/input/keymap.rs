use std::collections::BTreeMap;

use bevy::{asset::io::file::FileAssetReader, prelude::*};
use serde::Deserialize;

use crate::editor::input::{Action, ActionState, AxisInput, Shortcut, ValueKind};

const KEYMAP_FILE: &str = "keymap.toml";

#[derive(Deserialize)]
struct KeymapFile {
	bindings: BTreeMap<String, Vec<String>>,
}

/// Physical inputs bound to registered actions by name, so the keymap works for any action group.
#[derive(Resource)]
pub struct Keymap {
	button_bindings: Vec<(Shortcut, &'static str)>,
	axis_bindings: Vec<(AxisInput, &'static str)>,
}

impl Keymap {
	pub fn button_bindings(&self) -> &[(Shortcut, &'static str)] {
		&self.button_bindings
	}

	pub fn axis_bindings(&self) -> &[(AxisInput, &'static str)] {
		&self.axis_bindings
	}

	pub fn shortcuts_for(&self, action: impl Action) -> impl Iterator<Item = &Shortcut> {
		let name = action.name();
		self.button_bindings
			.iter()
			.filter(move |(_, bound_name)| *bound_name == name)
			.map(|(shortcut, _)| shortcut)
	}

	/// Reads the keymap from the assets folder, falling back to `built_in` if the file is missing
	/// or invalid so that a broken file never leaves the app without shortcuts.
	///
	/// The actions the keymap binds must already be registered.
	pub fn load(actions: &ActionState, built_in: &str) -> Self {
		let path = FileAssetReader::get_base_path()
			.join("assets")
			.join(KEYMAP_FILE);
		let text = match std::fs::read_to_string(&path) {
			Ok(text) => text,
			Err(error) => {
				warn!(
					"cannot read {}: {error}; using built-in keymap",
					path.display()
				);
				return Self::parse_built_in(built_in, actions);
			}
		};
		match Self::parse(&text, actions) {
			Ok(keymap) => keymap,
			Err(error) => {
				warn!("invalid {}: {error}; using built-in keymap", path.display());
				Self::parse_built_in(built_in, actions)
			}
		}
	}

	fn parse_built_in(built_in: &str, actions: &ActionState) -> Self {
		Self::parse(built_in, actions).expect("the built-in keymap.toml is valid")
	}

	// Builds the whole keymap before returning it, so an error never yields a partial keymap.
	pub(crate) fn parse(text: &str, actions: &ActionState) -> Result<Self, String> {
		let file: KeymapFile = toml::from_str(text).map_err(|error| error.to_string())?;

		let mut button_bindings: Vec<(Shortcut, &'static str)> = Vec::new();
		let mut axis_bindings: Vec<(AxisInput, &'static str)> = Vec::new();

		for (action_name, entry_texts) in file.bindings {
			let action = actions
				.registered_name(&action_name)
				.ok_or_else(|| format!("unknown action \"{action_name}\""))?;

			for entry_text in entry_texts {
				// Axis inputs are told apart from shortcuts by their last token
				let last_token = entry_text.rsplit('+').next().unwrap_or_default();

				if let Ok(input) = last_token.parse::<AxisInput>() {
					if entry_text.contains('+') {
						return Err(format!(
							"action \"{action_name}\": modifiers on axis inputs are not supported (\"{entry_text}\")"
						));
					}

					if actions.kind_of(action) != Some(input.value_kind()) {
						return Err(format!(
							"action \"{action_name}\" cannot be bound to \"{input}\": the kinds do not match"
						));
					}

					if let Some((_, other_action)) = axis_bindings
						.iter()
						.find(|(existing, _)| *existing == input)
					{
						return Err(format!(
							"axis input \"{input}\" is bound to both {other_action} and {action}"
						));
					}

					axis_bindings.push((input, action));
					continue;
				}

				// A shortcut is a button press, so it can only drive a button action.
				if actions.kind_of(action) != Some(ValueKind::Button) {
					return Err(format!(
						"action \"{action_name}\" is not a button, so it cannot be bound to the shortcut \"{entry_text}\""
					));
				}

				let shortcut: Shortcut = entry_text
					.parse()
					.map_err(|error| format!("action \"{action_name}\": {error}"))?;

				if let Some((_, other_action)) = button_bindings
					.iter()
					.find(|(existing, _)| *existing == shortcut)
				{
					return Err(format!(
						"shortcut \"{entry_text}\" is bound to both {other_action} and {action}"
					));
				}

				button_bindings.push((shortcut, action));
			}
		}

		Ok(Self {
			button_bindings,
			axis_bindings,
		})
	}
}

#[cfg(test)]
#[path = "tests/keymap_tests.rs"]
mod tests;
