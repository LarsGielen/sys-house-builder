use bevy::{picking::hover::Hovered, prelude::*};

use crate::{
	editor::{ActionTrigger, Keymap},
	ui::theme::{colors, editor_layout, spacing, typography},
};

const SHOW_DELAY_SECONDS: f32 = 0.5;
const GAP_BELOW_BUTTON: f32 = spacing::TINY;

pub(super) struct TooltipPlugin;

impl Plugin for TooltipPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Startup, spawn_tooltip)
			.add_systems(Update, update_tooltip);
	}
}

#[derive(Component)]
struct Tooltip;

/// Text shown when the entity is hovered. The entity also needs a `Hovered` component.
/// If it has an `ActionTrigger`, the action's shortcut is appended.
#[derive(Component, Default, Clone)]
pub struct TooltipText(pub Option<&'static str>);

#[derive(Default)]
struct HoverTimer {
	target: Option<Entity>,
	seconds: f32,
}

// A single shared tooltip avoids the clipping and draw-order problems of a tooltip child per button.
fn spawn_tooltip(mut commands: Commands) {
	commands.spawn((
		Name::new("Tooltip"),
		Tooltip,
		Text::new(""),
		TextFont {
			font_size: FontSize::Px(typography::SMALL),
			..default()
		},
		TextColor(colors::TEXT),
		Node {
			position_type: PositionType::Absolute,
			padding: UiRect::axes(Val::Px(spacing::COMPACT), Val::Px(spacing::TINY)),
			border: UiRect::all(Val::Px(1.0)),
			border_radius: BorderRadius::all(Val::Px(editor_layout::CONTROL_RADIUS)),
			..default()
		},
		BackgroundColor(colors::OVERLAY),
		BorderColor::all(colors::BORDER),
		GlobalZIndex(100),
		Visibility::Hidden,
		Pickable::IGNORE,
	));
}

fn update_tooltip(
	time: Res<Time>,
	keymap: Res<Keymap>,
	targets: Query<(
		Entity,
		&Hovered,
		&TooltipText,
		Option<&ActionTrigger>,
		&ComputedNode,
		&UiGlobalTransform,
	)>,
	tooltip: Single<(&mut Node, &mut Text, &mut Visibility), With<Tooltip>>,
	mut hover: Local<HoverTimer>,
) {
	let (mut node, mut text, mut visibility) = tooltip.into_inner();

	let hovered_target = targets
		.iter()
		.find(|(_, hovered, text, ..)| hovered.get() && text.0.is_some());
	let Some((entity, _, tooltip_text, trigger, computed, transform)) = hovered_target else {
		*hover = HoverTimer::default();
		visibility.set_if_neq(Visibility::Hidden);
		return;
	};
	let Some(label) = tooltip_text.0 else {
		return;
	};

	if hover.target != Some(entity) {
		*hover = HoverTimer {
			target: Some(entity),
			seconds: 0.0,
		};
	}
	hover.seconds += time.delta_secs();
	if hover.seconds < SHOW_DELAY_SECONDS {
		visibility.set_if_neq(Visibility::Hidden);
		return;
	}

	// Looked up on every show, so the hint follows the keymap if it ever changes at runtime.
	let shortcut = trigger.and_then(|trigger| keymap.shortcuts_for(trigger.0).next());
	let hint_text = match shortcut {
		Some(shortcut) => format!("{label} ({shortcut})"),
		None => label.to_string(),
	};
	if text.0 != hint_text {
		text.0 = hint_text;
	}

	// Layout values are in physical pixels while `Node` positions are logical.
	let size = computed.size() * computed.inverse_scale_factor;
	let top_left = transform.translation * computed.inverse_scale_factor - size / 2.0;
	let left = Val::Px(top_left.x);
	let top = Val::Px(top_left.y + size.y + GAP_BELOW_BUTTON);
	// Compared first so an unchanged tooltip does not trigger a relayout every frame.
	if node.left != left {
		node.left = left;
	}
	if node.top != top {
		node.top = top;
	}
	visibility.set_if_neq(Visibility::Inherited);
}
