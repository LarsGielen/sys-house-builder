use crate::editor_core::input::signal::ValueKind;

pub trait Action: Copy {
	// Stable and unique across all groups (ex: camera.orbit)
	fn name(self) -> &'static str;
	fn value(self) -> ValueKind;
}

/// A fixed set of actions, normally one enum, that is registered together.
pub trait ActionGroup: Action + 'static {
	fn all() -> &'static [Self];
}
