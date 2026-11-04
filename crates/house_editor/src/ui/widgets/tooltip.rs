use bevy::{ecs::query::QueryData, picking::hover::Hovered, prelude::*};

use crate::{
	editor::{controls::ActionTrigger, input::Keymap},
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

#[derive(QueryData)]
struct TooltipQuery {
	entity: Entity,
	hovered: &'static Hovered,
	text: &'static TooltipText,
	action_trigger: Option<&'static ActionTrigger>,
	computed_node: &'static ComputedNode,
	ui_transform: &'static UiGlobalTransform,
}

fn update_tooltip(
	time: Res<Time>,
	keymap: Res<Keymap>,
	targets: Query<TooltipQuery>,
	tooltip: Single<(&mut Node, &mut Text, &mut Visibility), With<Tooltip>>,
	mut hover: Local<HoverTimer>,
) {
	let (mut node, mut text, mut visibility) = tooltip.into_inner();

	let hovered_target = targets
		.iter()
		.find(|target| target.hovered.get() && target.text.0.is_some());
	let Some(target) = hovered_target else {
		*hover = HoverTimer::default();
		visibility.set_if_neq(Visibility::Hidden);
		return;
	};
	let Some(label) = target.text.0 else {
		return;
	};

	if hover.target != Some(target.entity) {
		*hover = HoverTimer {
			target: Some(target.entity),
			seconds: 0.0,
		};
	}
	hover.seconds += time.delta_secs();
	if hover.seconds < SHOW_DELAY_SECONDS {
		visibility.set_if_neq(Visibility::Hidden);
		return;
	}

	// Looked up on every show, so the hint follows the keymap if it ever changes at runtime.
	let shortcut = target
		.action_trigger
		.and_then(|trigger| trigger.0)
		.and_then(|action| keymap.shortcuts_for(action).next());
	let hint_text = match shortcut {
		Some(shortcut) => format!("{label} ({shortcut})"),
		None => label.to_string(),
	};
	if text.0 != hint_text {
		text.0 = hint_text;
	}

	// Layout values are in physical pixels while `Node` positions are logical.
	let computed = target.computed_node;
	let size = computed.size() * computed.inverse_scale_factor;
	let top_left = target.ui_transform.translation * computed.inverse_scale_factor - size / 2.0;
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
