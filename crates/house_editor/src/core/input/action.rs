use crate::core::input::value::ValueKind;

/// Something that can happen in the editor, such as orbiting the camera. An action says what
/// happens; which inputs cause it is up to the keymap.
pub trait Action: Copy {
	/// The name used in the keymap file. Stable and unique across all groups (ex: camera.orbit)
	fn name(self) -> &'static str;
	/// What kind of value the action carries, which decides the inputs it can be bound to.
	fn value(self) -> ValueKind;
}

/// A fixed set of actions, normally one enum, that is registered together.
pub trait ActionGroup: Action + 'static {
	/// Every action of the group. Registration uses it, so an action left out cannot be bound in
	/// the keymap.
	fn all() -> &'static [Self];
}
