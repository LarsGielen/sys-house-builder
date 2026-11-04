use std::collections::BTreeMap;

use bevy::{asset::io::file::FileAssetReader, prelude::*};
use serde::Deserialize;

use crate::editor::input::{Action, ActionState, Shortcut};

const KEYMAP_FILE: &str = "keymap.toml";

#[derive(Deserialize)]
struct KeymapFile {
	bindings: BTreeMap<String, Vec<String>>,
}

/// Shortcuts bound to registered actions by name, so the keymap works for any action group.
#[derive(Resource)]
pub struct Keymap {
	bindings: Vec<(Shortcut, &'static str)>,
}

impl Keymap {
	pub fn bindings(&self) -> &[(Shortcut, &'static str)] {
		&self.bindings
	}

	pub fn shortcuts_for(&self, action: impl Action) -> impl Iterator<Item = &Shortcut> {
		let name = action.name();
		self.bindings
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

		let mut bindings: Vec<(Shortcut, &'static str)> = Vec::new();
		for (action_name, shortcut_texts) in file.bindings {
			let action = actions
				.registered_name(&action_name)
				.ok_or_else(|| format!("unknown action \"{action_name}\""))?;
			for shortcut_text in shortcut_texts {
				let shortcut: Shortcut = shortcut_text
					.parse()
					.map_err(|error| format!("action \"{action_name}\": {error}"))?;
				if let Some((_, other_action)) =
					bindings.iter().find(|(existing, _)| *existing == shortcut)
				{
					return Err(format!(
						"shortcut \"{shortcut_text}\" is bound to both {other_action} and {action}"
					));
				}
				bindings.push((shortcut, action));
			}
		}
		Ok(Self { bindings })
	}
}

#[cfg(test)]
#[path = "tests/keymap_tests.rs"]
mod tests;
