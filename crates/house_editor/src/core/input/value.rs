use bevy::math::Vec2;

/// What the value of an axis means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisMode {
	/// A position, such as which directions are held. Bindings of one source never add up beyond
	/// the strongest one, so WASD and the arrow keys together are still one stick at full tilt.
	Absolute,
	/// A change since last frame, such as mouse motion. Bindings of one source add up.
	Relative,
}

/// The kind of value an action carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
	/// Held or not.
	Button,
	/// A button whose value flips on every press and is kept between frames.
	Toggle,
	Axis1D(AxisMode),
	Axis2D(AxisMode),
}

/// What one source contributes to an action in one frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Signal {
	Button(bool),
	Axis1D(f32),
	Axis2D(Vec2),
}

impl ValueKind {
	pub(crate) fn default_signal(self) -> Signal {
		match self {
			ValueKind::Button | ValueKind::Toggle => Signal::Button(false),
			ValueKind::Axis1D(_) => Signal::Axis1D(0.0),
			ValueKind::Axis2D(_) => Signal::Axis2D(Vec2::ZERO),
		}
	}

	pub(crate) fn accepts(self, signal: Signal) -> bool {
		matches!(
			(self, signal),
			(ValueKind::Button | ValueKind::Toggle, Signal::Button(_))
				| (ValueKind::Axis1D(_), Signal::Axis1D(_))
				| (ValueKind::Axis2D(_), Signal::Axis2D(_))
		)
	}
}
