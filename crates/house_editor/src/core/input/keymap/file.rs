use std::collections::BTreeMap;

use serde::Deserialize;

use crate::core::input::{
	axis_input::{AxisInput, Scale},
	button_combo::{ButtonCombo, PhysicalButton},
	modifiers::ModifierSet,
};

// Unknown fields are rejected so that a file from before contexts (a top-level `[bindings]`) is
// reported instead of being read as empty.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct KeymapFile {
	pub(super) contexts: BTreeMap<String, ContextSection>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ContextSection {
	pub(super) bindings: BTreeMap<String, Vec<Entry>>,
}

/// One binding in the keymap file: plain text (`"ctrl+z"`), or a table for the bindings that need
/// more than an input:
///
/// - `{ input = "scroll", scale = -1.0 }`
/// - `{ input = "e", interpret = "released" }`
/// - `{ composite = "2d", up = "w", down = "s", left = "a", right = "d", modifiers = ["shift"] }`
/// - `{ composite = "1d", up = "space", down = "c" }`
#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum Entry {
	Text(String),
	Table(EntryTable),
}

// One struct with optional fields instead of a variant per shape, so that mistakes get a message
// that names the problem instead of "data did not match any variant". Unknown fields are rejected
// so that a misspelled option is an error, not silently ignored.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EntryTable {
	input: Option<String>,
	composite: Option<String>,
	up: Option<String>,
	down: Option<String>,
	left: Option<String>,
	right: Option<String>,
	modifiers: Option<Vec<String>>,
	scale: Option<Scale>,
	interpret: Option<String>,
}

/// What an entry binds, before it is checked against the action it is bound to.
pub(super) enum ParsedEntry {
	ButtonCombo(ButtonCombo),
	Axis {
		input: AxisInput,
		modifiers: ModifierSet,
	},
}

impl Entry {
	pub(super) fn scale(&self) -> Option<Scale> {
		match self {
			Self::Text(_) => None,
			Self::Table(table) => table.scale,
		}
	}

	pub(super) fn interpret(&self) -> Option<&str> {
		match self {
			Self::Text(_) => None,
			Self::Table(table) => table.interpret.as_deref(),
		}
	}

	pub(super) fn parse(&self) -> Result<ParsedEntry, String> {
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

	text.parse().map(ParsedEntry::ButtonCombo)
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
		let modifiers: ModifierSet = self
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
		// A direction that is a modifier key is judged without itself, like a shortcut on that key,
		// so the composite cannot also require it as a modifier.
		for direction in input.directions() {
			if let PhysicalButton::Modifier(modifier) = direction
				&& modifiers.contains(modifier)
			{
				return Err(format!(
					"a composite cannot use \"{}\" as a direction and as a modifier",
					direction.name().unwrap_or_default()
				));
			}
		}
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

fn direction(name: &str, value: &Option<String>, shape: &str) -> Result<PhysicalButton, String> {
	let text = value
		.as_deref()
		.ok_or_else(|| format!("composite {shape} is missing `{name}`"))?;
	PhysicalButton::from_name(&text.trim().to_ascii_lowercase())
		.ok_or_else(|| format!("unknown key or mouse button \"{text}\" for `{name}`"))
}
