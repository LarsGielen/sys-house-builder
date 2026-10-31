use bevy::prelude::*;

use crate::input::Bindings;

/// Text that mentions controls by action, such as `{Cancel}`, shown with the active bindings.
#[derive(Component)]
pub struct HintTemplate(pub &'static str);

/// A text node whose control names follow [`Bindings`].
pub fn hint(template: &'static str) -> impl Bundle {
	(Text::new(""), HintTemplate(template))
}

pub fn fill_hints(bindings: Res<Bindings>, mut hints: Query<(&HintTemplate, &mut Text)>) {
	for (template, mut text) in &mut hints {
		let wanted = bindings.fill(template.0);
		// Writing identical text would still trigger a text layout every frame.
		if text.0 != wanted {
			text.0 = wanted;
		}
	}
}

#[cfg(test)]
#[path = "../tests/input/hints.rs"]
mod tests;
