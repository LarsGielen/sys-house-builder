use std::collections::HashMap;

use bevy::prelude::*;

use crate::core::input::{
	sources::SourceId,
	state::ActionSlot,
	value::{AxisMode, Signal, ValueKind},
};

impl ActionSlot {
	/// Combines this frame's contributions into the slot's signal.
	pub(super) fn resolve(&mut self) {
		// Within one source, bindings of an absolute axis add up (W cancels S), but never beyond the
		// strongest single binding: WASD and the arrow keys held together are still one stick at full
		// tilt, not two. Relative axes (mouse motion) are deltas, so they do sum.
		self.signal = match self.value_kind {
			ValueKind::Button => Signal::Button(
				self.contributions
					.iter()
					.any(|(_, signal)| matches!(signal, Signal::Button(true))),
			),
			ValueKind::Toggle => {
				let pressed = self
					.contributions
					.iter()
					.any(|(_, signal)| matches!(signal, Signal::Button(true)));
				// Flips on the press, whichever sources press it, and not again while held.
				if pressed && !self.was_pressed {
					self.toggled = !self.toggled;
				}
				self.was_pressed = pressed;
				Signal::Button(self.toggled)
			}
			ValueKind::Axis1D(mode) => {
				let mut values = HashMap::<SourceId, f32>::new();
				let mut limits = HashMap::<SourceId, f32>::new();

				for (source, signal) in &self.contributions {
					if let Signal::Axis1D(value) = signal {
						*values.entry(*source).or_default() += value;
						let limit = limits.entry(*source).or_default();
						*limit = limit.max(value.abs());
					}
				}

				if mode == AxisMode::Absolute {
					for (source, value) in &mut values {
						*value = value.clamp(-limits[source], limits[source]);
					}
				}

				Signal::Axis1D(resolve_owner(&mut self.owner, &self.contributions, &values))
			}
			ValueKind::Axis2D(mode) => {
				let mut values = HashMap::<SourceId, Vec2>::new();
				let mut limits = HashMap::<SourceId, f32>::new();

				for (source, signal) in &self.contributions {
					if let Signal::Axis2D(value) = signal {
						*values.entry(*source).or_default() += value;
						let limit = limits.entry(*source).or_default();
						*limit = limit.max(value.length());
					}
				}

				if mode == AxisMode::Absolute {
					for (source, value) in &mut values {
						*value = value.clamp_length_max(limits[source]);
					}
				}

				Signal::Axis2D(resolve_owner(&mut self.owner, &self.contributions, &values))
			}
		};
	}
}

fn resolve_owner<T: Copy + Default + PartialEq>(
	owner: &mut Option<SourceId>,
	contributions: &[(SourceId, Signal)],
	values: &HashMap<SourceId, T>,
) -> T {
	if let Some(current) = owner
		&& values.get(current).copied().unwrap_or_default() == T::default()
	{
		*owner = None;
	}

	if owner.is_none() {
		*owner = contributions.iter().find_map(|(source, _)| {
			(values.get(source).copied().unwrap_or_default() != T::default()).then_some(*source)
		});
	}

	owner
		.and_then(|source| values.get(&source).copied())
		.unwrap_or_default()
}
