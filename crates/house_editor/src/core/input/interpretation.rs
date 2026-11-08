use std::{fmt, str::FromStr};

/// When a shortcut counts as active. Every shortcut starts as `Pressed`; the keymap can switch one
/// with `interpret = "released"`, so the action's code does not change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Interpretation {
	/// Active while the input is held, starting on the press.
	#[default]
	Pressed,
	/// Active for one frame, when the input is released. A shortcut that ends because its context
	/// went away was cancelled, so it does not fire.
	Released,
}

impl FromStr for Interpretation {
	type Err = String;

	fn from_str(text: &str) -> Result<Self, String> {
		match text.trim().to_ascii_lowercase().as_str() {
			"pressed" => Ok(Self::Pressed),
			"released" => Ok(Self::Released),
			other => Err(format!(
				"unknown interpretation \"{other}\", expected \"pressed\" or \"released\""
			)),
		}
	}
}

impl fmt::Display for Interpretation {
	fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
		formatter.write_str(match self {
			Self::Pressed => "pressed",
			Self::Released => "released",
		})
	}
}
