use bevy::prelude::{Deref, Resource};
use house_editor_core::InteractionSettings;

/// Drag threshold and snap radii in effect, as a Bevy resource. A host replaces it with values
/// that passed [`InteractionSettings::new`]; the defaults match the editor's standard feel.
#[derive(Resource, Default, Deref)]
pub struct InteractionConfig(pub InteractionSettings);
