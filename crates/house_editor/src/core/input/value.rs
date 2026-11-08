use bevy::math::Vec2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisMode {
	Absolute,
	Relative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
	Button,
	Toggle,
	Axis1D(AxisMode),
	Axis2D(AxisMode),
}

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
