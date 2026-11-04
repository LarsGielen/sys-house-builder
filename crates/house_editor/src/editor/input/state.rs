use std::collections::HashMap;

use bevy::prelude::*;

use crate::editor::input::{
	action::{Action, ActionGroup},
	signal::{
		Signal,
		ValueKind::{self},
	},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceId(u32);

struct ActionSlot {
	value_kind: ValueKind,
	signal: Signal,
	signal_previous: Signal,
	contributions: Vec<(SourceId, Signal)>,
	owner: Option<SourceId>,
}

#[derive(Default, Resource)]
pub struct ActionState {
	slots: HashMap<&'static str, ActionSlot>,
}

#[derive(Debug)]
pub enum ContributionError {
	Unregistered,
	WrongKind,
}

impl ActionState {
	pub fn register(&mut self, action: impl Action) {
		self.slots.insert(
			action.name(),
			ActionSlot {
				value_kind: action.value(),
				signal: action.value().default_signal(),
				signal_previous: action.value().default_signal(),
				contributions: Vec::new(),
				owner: None,
			},
		);
	}

	pub fn register_group<Group: ActionGroup>(&mut self) {
		for &action in Group::all() {
			self.register(action);
		}
	}

	/// The registered name equal to `name`, as the `'static` string that identifies the action.
	pub fn registered_name(&self, name: &str) -> Option<&'static str> {
		self.slots.get_key_value(name).map(|(name, _)| *name)
	}

	pub fn start_frame(&mut self) {
		for slot in self.slots.values_mut() {
			slot.signal_previous = slot.signal;
			slot.contributions.clear();
		}
	}

	pub fn contribute(
		&mut self,
		source: SourceId,
		action: impl Action,
		signal: Signal,
	) -> Result<(), ContributionError> {
		let Some(slot) = self.slots.get_mut(action.name()) else {
			return Err(ContributionError::Unregistered);
		};

		if !slot.value_kind.accepts(signal) {
			return Err(ContributionError::WrongKind);
		}

		slot.contributions.push((source, signal));
		Ok(())
	}

	pub fn resolve(&mut self) {
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
					(values.get(source).copied().unwrap_or_default() != T::default())
						.then_some(*source)
				});
			}

			owner
				.and_then(|source| values.get(&source).copied())
				.unwrap_or_default()
		}

		for slot in self.slots.values_mut() {
			slot.signal = match slot.value_kind {
				ValueKind::Button => Signal::Button(
					slot.contributions
						.iter()
						.any(|(_, signal)| matches!(signal, Signal::Button(true))),
				),
				ValueKind::Axis1D(_) => {
					let mut values = HashMap::<SourceId, f32>::new();

					for (source, signal) in &slot.contributions {
						if let Signal::Axis1D(value) = signal {
							*values.entry(*source).or_default() += value;
						}
					}

					Signal::Axis1D(resolve_owner(&mut slot.owner, &slot.contributions, &values))
				}
				ValueKind::Axis2D(_) => {
					let mut values = HashMap::<SourceId, Vec2>::new();

					for (source, signal) in &slot.contributions {
						if let Signal::Axis2D(value) = signal {
							*values.entry(*source).or_default() += value;
						}
					}

					Signal::Axis2D(resolve_owner(&mut slot.owner, &slot.contributions, &values))
				}
			};
		}
	}

	pub fn pressed(&self, action: impl Action) -> bool {
		let Some(slot) = self.slots.get(action.name()) else {
			return false;
		};

		matches!(
			(slot.value_kind, slot.signal),
			(ValueKind::Button, Signal::Button(true),)
		)
	}

	pub fn just_pressed(&self, action: impl Action) -> bool {
		let Some(slot) = self.slots.get(action.name()) else {
			return false;
		};

		matches!(
			(slot.value_kind, slot.signal, slot.signal_previous),
			(
				ValueKind::Button,
				Signal::Button(true),
				Signal::Button(false)
			)
		)
	}

	pub fn just_released(&self, action: impl Action) -> bool {
		let Some(slot) = self.slots.get(action.name()) else {
			return false;
		};

		matches!(
			(slot.value_kind, slot.signal, slot.signal_previous),
			(
				ValueKind::Button,
				Signal::Button(false),
				Signal::Button(true)
			)
		)
	}

	pub fn axis1d(&self, action: impl Action) -> f32 {
		match self.slots.get(action.name()).map(|slot| slot.signal) {
			Some(Signal::Axis1D(value)) => value,
			_ => 0.0,
		}
	}

	pub fn axis2d(&self, action: impl Action) -> Vec2 {
		match self.slots.get(action.name()).map(|slot| slot.signal) {
			Some(Signal::Axis2D(value)) => value,
			_ => Vec2::ZERO,
		}
	}
}

#[cfg(test)]
#[path = "tests/state_tests.rs"]
mod tests;
