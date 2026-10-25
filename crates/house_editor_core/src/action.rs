use std::collections::HashSet;

use glam::Vec2;

/// What the user wants to do, independent of how they asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
	/// Primary pointer action: begin, hold, and end the active tool's gesture.
	ToolPress,
	/// Abandon the current gesture or selection. Not produced while typing in a field.
	Cancel,
	/// Confirm the pending operation, such as a wall deletion. Not produced while typing.
	Confirm,
	/// Delete the selected target. Not produced while typing.
	Delete,
	/// Leave the focused field and drop the draft it was building.
	FieldCancel,
	/// Apply the value in the focused field.
	FieldSubmit,
	/// While held, positions are not snapped.
	SuppressSnapping,
	Undo,
	Redo,
	Save,
	Open,
	/// While held, pointer motion orbits the camera.
	Orbit,
	/// While held, pointer motion pans the camera.
	Pan,
	CameraReset,
	PanLeft,
	PanRight,
	PanForward,
	PanBack,
}

impl Action {
	/// Every action, for listing and for finding one by name.
	pub const ALL: [Action; 18] = [
		Action::ToolPress,
		Action::Cancel,
		Action::Confirm,
		Action::Delete,
		Action::FieldCancel,
		Action::FieldSubmit,
		Action::SuppressSnapping,
		Action::Undo,
		Action::Redo,
		Action::Save,
		Action::Open,
		Action::Orbit,
		Action::Pan,
		Action::CameraReset,
		Action::PanLeft,
		Action::PanRight,
		Action::PanForward,
		Action::PanBack,
	];
}

/// What an [`Action`] sees of the current frame.
#[derive(Default)]
pub struct ActionState {
	pressed: HashSet<Action>,
	just_pressed: HashSet<Action>,
	just_released: HashSet<Action>,
	pointer_motion: Vec2,
	scroll_lines: f32,
}

impl ActionState {
	/// Starts a new frame: nothing is pressed or moving until the host says so.
	pub fn clear(&mut self) {
		*self = Self::default();
	}

	/// An action begins this frame and is held until the next [`Self::clear`].
	pub fn press(&mut self, action: Action) {
		self.pressed.insert(action);
		self.just_pressed.insert(action);
	}

	/// An action is held, having begun in an earlier frame.
	pub fn hold(&mut self, action: Action) {
		self.pressed.insert(action);
	}

	/// An action ends this frame.
	pub fn release(&mut self, action: Action) {
		self.just_released.insert(action);
	}

	pub fn set_pointer_motion(&mut self, motion: Vec2) {
		self.pointer_motion = motion;
	}

	pub fn set_scroll_lines(&mut self, lines: f32) {
		self.scroll_lines = lines;
	}

	pub fn pressed(&self, action: Action) -> bool {
		self.pressed.contains(&action)
	}

	pub fn just_pressed(&self, action: Action) -> bool {
		self.just_pressed.contains(&action)
	}

	pub fn just_released(&self, action: Action) -> bool {
		self.just_released.contains(&action)
	}

	/// Pointer movement since the last frame, in pixels.
	pub fn pointer_motion(&self) -> Vec2 {
		self.pointer_motion
	}

	/// Scrolling since the last frame, with up positive.
	pub fn scroll_lines(&self) -> f32 {
		self.scroll_lines
	}
}
