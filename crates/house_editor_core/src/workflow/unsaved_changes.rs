use house_document::House;

/// What the user chose when asked about unsaved changes before the document is replaced or
/// closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsavedChoice {
	Save,
	Discard,
	Cancel,
}

/// What a replacement or close does next. Only `Keep` leaves the document, selection, and
/// history alone, so a caller that performs the replacement only for the other two cannot
/// lose work by cancelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplacementStep {
	Keep,
	SaveThenReplace,
	Replace,
}

/// Whether the user must be asked before `house` is replaced.
pub fn needs_confirmation(house: &House) -> bool {
	house.is_modified()
}

impl UnsavedChoice {
	pub fn step(self) -> ReplacementStep {
		match self {
			Self::Save => ReplacementStep::SaveThenReplace,
			Self::Discard => ReplacementStep::Replace,
			Self::Cancel => ReplacementStep::Keep,
		}
	}
}

#[cfg(test)]
#[path = "../tests/workflow/unsaved_changes.rs"]
mod tests;
