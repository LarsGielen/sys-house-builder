use std::collections::BTreeMap;

use bevy::{asset::io::file::FileAssetReader, prelude::*};
use serde::Deserialize;

use crate::editor::input::{
	Action, ActionState, AxisBinding, AxisInput, Scale, Shortcut, ShortcutInput, ValueKind,
	modifiers::Modifiers,
};

const KEYMAP_FILE: &str = "keymap.toml";

#[derive(Deserialize)]
struct KeymapFile {
	bindings: BTreeMap<String, Vec<Entry>>,
}

/// One binding in the keymap file: plain text (`"ctrl+z"`), or a table for the bindings that need
/// more than an input:
///
/// - `{ input = "scroll", scale = -1.0 }`
/// - `{ composite = "2d", up = "w", down = "s", left = "a", right = "d", modifiers = ["shift"] }`
/// - `{ composite = "1d", up = "space", down = "c" }`
#[derive(Deserialize)]
#[serde(untagged)]
enum Entry {
	Text(String),
	Table(EntryTable),
}

// One struct with optional fields instead of a variant per shape, so that mistakes get a message
// that names the problem instead of "data did not match any variant". Unknown fields are rejected
// so that a misspelled option is an error, not silently ignored.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EntryTable {
	input: Option<String>,
	composite: Option<String>,
	up: Option<String>,
	down: Option<String>,
	left: Option<String>,
	right: Option<String>,
	modifiers: Option<Vec<String>>,
	scale: Option<Scale>,
}

/// What an entry binds, before it is checked against the action it is bound to.
enum ParsedEntry {
	Shortcut(Shortcut),
	Axis {
		input: AxisInput,
		modifiers: Modifiers,
	},
}

impl Entry {
	fn scale(&self) -> Option<Scale> {
		match self {
			Self::Text(_) => None,
			Self::Table(table) => table.scale,
		}
	}

	fn parse(&self) -> Result<ParsedEntry, String> {
		match self {
			Self::Text(text) => parse_input_text(text),
			Self::Table(table) => table.parse(),
		}
	}
}

/// Parses `shift+scroll` or `ctrl+z`. Axis inputs are told apart from shortcuts by their last token.
fn parse_input_text(text: &str) -> Result<ParsedEntry, String> {
	let (modifiers_text, last_token) = text.rsplit_once('+').unwrap_or(("", text));

	if let Ok(input) = last_token.parse::<AxisInput>() {
		let modifiers = modifiers_text
			.parse()
			.map_err(|error| format!("{error} in \"{text}\""))?;
		return Ok(ParsedEntry::Axis { input, modifiers });
	}

	text.parse().map(ParsedEntry::Shortcut)
}

impl EntryTable {
	fn parse(&self) -> Result<ParsedEntry, String> {
		match (&self.input, &self.composite) {
			(Some(_), Some(_)) => {
				Err("an entry has either `input` or `composite`, not both".into())
			}
			(None, None) => Err("an entry needs `input` or `composite`".into()),
			(Some(input), None) => {
				if self.up.is_some()
					|| self.down.is_some()
					|| self.left.is_some()
					|| self.right.is_some()
				{
					return Err("`up`, `down`, `left` and `right` only go with `composite`".into());
				}
				if self.modifiers.is_some() {
					return Err(
						"`modifiers` only goes with `composite`; write them in the input, such as \"shift+scroll\"".into(),
					);
				}
				parse_input_text(input)
			}
			(None, Some(shape)) => self.parse_composite(shape),
		}
	}

	fn parse_composite(&self, shape: &str) -> Result<ParsedEntry, String> {
		let modifiers = self
			.modifiers
			.as_deref()
			.unwrap_or_default()
			.join("+")
			.parse()
			.map_err(|error| format!("{error} in `modifiers`"))?;

		let shape = shape.trim().to_ascii_lowercase();
		let input = match shape.as_str() {
			"1d" => {
				self.reject_directions(&shape, [("left", &self.left), ("right", &self.right)])?;
				AxisInput::composite_1d(
					direction("up", &self.up, &shape)?,
					direction("down", &self.down, &shape)?,
				)?
			}
			"2d" => AxisInput::composite_2d(
				direction("up", &self.up, &shape)?,
				direction("down", &self.down, &shape)?,
				direction("left", &self.left, &shape)?,
				direction("right", &self.right, &shape)?,
			)?,
			_ => {
				return Err(format!(
					"unknown composite \"{shape}\", expected \"1d\" or \"2d\""
				));
			}
		};
		Ok(ParsedEntry::Axis { input, modifiers })
	}

	fn reject_directions(
		&self,
		shape: &str,
		directions: [(&str, &Option<String>); 2],
	) -> Result<(), String> {
		for (name, value) in directions {
			if value.is_some() {
				return Err(format!("composite {shape} does not take `{name}`"));
			}
		}
		Ok(())
	}
}

fn direction(name: &str, value: &Option<String>, shape: &str) -> Result<ShortcutInput, String> {
	let text = value
		.as_deref()
		.ok_or_else(|| format!("composite {shape} is missing `{name}`"))?;
	ShortcutInput::from_name(&text.trim().to_ascii_lowercase())
		.ok_or_else(|| format!("unknown key or mouse button \"{text}\" for `{name}`"))
}

/// Physical inputs bound to registered actions by name, so the keymap works for any action group.
#[derive(Resource)]
pub struct Keymap {
	button_bindings: Vec<(Shortcut, &'static str)>,
	axis_bindings: Vec<AxisBinding>,
}

impl Keymap {
	pub fn button_bindings(&self) -> &[(Shortcut, &'static str)] {
		&self.button_bindings
	}

	pub fn axis_bindings(&self) -> &[AxisBinding] {
		&self.axis_bindings
	}

	pub fn shortcuts_for(&self, name: &'static str) -> impl Iterator<Item = &Shortcut> {
		self.button_bindings
			.iter()
			.filter(move |(_, bound_name)| *bound_name == name)
			.map(|(shortcut, _)| shortcut)
	}

	/// The actions that `defaults` binds and this keymap leaves without any binding, sorted. A user
	/// may remove a binding on purpose, so this is only reported, never filled in.
	pub(crate) fn actions_unbound_compared_to(&self, defaults: &Keymap) -> Vec<&'static str> {
		let is_bound = |name: &str| {
			self.button_bindings
				.iter()
				.any(|(_, bound_name)| *bound_name == name)
				|| self
					.axis_bindings
					.iter()
					.any(|binding| binding.action == name)
		};

		let mut unbound: Vec<&'static str> = defaults
			.button_bindings
			.iter()
			.map(|(_, name)| *name)
			.chain(defaults.axis_bindings.iter().map(|binding| binding.action))
			.filter(|name| !is_bound(name))
			.collect();
		unbound.sort_unstable();
		unbound.dedup();
		unbound
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
			Ok(keymap) => {
				// A file written before an action existed would leave it silently unbound.
				let defaults = Self::parse_built_in(built_in, actions);
				for name in keymap.actions_unbound_compared_to(&defaults) {
					warn!(
						"{} has no binding for \"{name}\", which the built-in keymap binds",
						path.display()
					);
				}
				keymap
			}
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
		let mut axis_bindings: Vec<AxisBinding> = Vec::new();

		for (action_name, entries) in file.bindings {
			let action = actions
				.registered_name(&action_name)
				.ok_or_else(|| format!("unknown action \"{action_name}\""))?;

			for entry in &entries {
				let parsed = entry
					.parse()
					.map_err(|error| format!("action \"{action_name}\": {error}"))?;

				match parsed {
					ParsedEntry::Axis { input, modifiers } => {
						if actions.kind_of(action) != Some(input.value_kind()) {
							return Err(format!(
								"action \"{action_name}\" cannot be bound to \"{input}\": the kinds do not match"
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
						});
					}
					ParsedEntry::Shortcut(shortcut) => {
						if entry.scale().is_some() {
							return Err(format!(
								"action \"{action_name}\": a shortcut cannot have a scale (\"{shortcut}\")"
							));
						}

						// A shortcut is a button press, so it can only drive a button action.
						if actions.kind_of(action) != Some(ValueKind::Button) {
							return Err(format!(
								"action \"{action_name}\" is not a button, so it cannot be bound to the shortcut \"{shortcut}\""
							));
						}

						if let Some((_, other_action)) = button_bindings
							.iter()
							.find(|(existing, _)| *existing == shortcut)
						{
							return Err(format!(
								"shortcut \"{shortcut}\" is bound to both {other_action} and {action}"
							));
						}

						button_bindings.push((shortcut, action));
					}
				}
			}
		}

		check_direction_conflicts(&button_bindings, &axis_bindings)?;

		Ok(Self {
			button_bindings,
			axis_bindings,
		})
	}
}

/// A button, with its modifiers, may be used by one binding only. Otherwise one press would drive
/// two actions, such as `s` saving while it also moves the camera back. Equal shortcuts and equal
/// composites are already rejected while parsing, so only the directions of composites remain.
fn check_direction_conflicts(
	button_bindings: &[(Shortcut, &'static str)],
	axis_bindings: &[AxisBinding],
) -> Result<(), String> {
	for (index, binding) in axis_bindings.iter().enumerate() {
		for direction in binding.input.directions() {
			let claimed = Shortcut::new(direction, binding.modifiers);

			if let Some((_, other_action)) = button_bindings
				.iter()
				.find(|(shortcut, _)| *shortcut == claimed)
			{
				return Err(format!(
					"\"{claimed}\" is used by both {other_action} and the composite of {}",
					binding.action
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

#[cfg(test)]
#[path = "tests/keymap_tests.rs"]
mod tests;
