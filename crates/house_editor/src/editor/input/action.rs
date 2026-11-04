use crate::editor::input::signal::ValueKind;

pub trait Action: Copy {
	// Stable and unique across all groups (ex: camera.orbit)
	fn name(self) -> &'static str;
	fn value(self) -> ValueKind;
}
