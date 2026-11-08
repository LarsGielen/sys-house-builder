use bevy::{ecs::query::QueryData, picking::hover::Hovered, prelude::*};

use crate::{
	core::input::{Availability, InputHints, UiAction},
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

/// The shortcut part of the tooltip, a span of its own so it can be dimmed on its own.
#[derive(Component)]
struct TooltipShortcut;

/// Text shown when the entity is hovered. The entity also needs a `Hovered` component.
/// If it has a `UiAction`, the action's shortcut is appended.
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
		children![(
			TooltipShortcut,
			TextSpan::new(""),
			TextFont {
				font_size: FontSize::Px(typography::SMALL),
				..default()
			},
			TextColor(colors::TEXT),
		)],
	));
}

#[derive(QueryData)]
struct TooltipQuery {
	entity: Entity,
	hovered: &'static Hovered,
	text: &'static TooltipText,
	ui_action: Option<&'static UiAction>,
	computed_node: &'static ComputedNode,
	ui_transform: &'static UiGlobalTransform,
}

fn update_tooltip(
	time: Res<Time>,
	hints: InputHints,
	targets: Query<TooltipQuery>,
	tooltip: Single<(&mut Node, &mut Text, &mut Visibility), With<Tooltip>>,
	shortcut_span: Single<(&mut TextSpan, &mut TextColor), With<TooltipShortcut>>,
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

	// Looked up on every show, so the hint follows the keymap and the active contexts.
	let hint = target
		.ui_action
		.and_then(|action| action.0)
		.and_then(|name| hints.for_action_named(name));
	if text.0 != label {
		text.0 = label.to_string();
	}
	let (mut span, mut span_color) = shortcut_span.into_inner();
	let (shortcut_text, color) = match hint {
		Some(hint) => (
			format!(" ({})", hint.shortcut),
			match hint.availability {
				Availability::Available => colors::TEXT,
				// Inactive and blocked look the same for now; they are told apart in the hint.
				Availability::ContextInactive | Availability::Blocked => colors::TEXT_FAINT,
			},
		),
		None => (String::new(), colors::TEXT),
	};
	if span.0 != shortcut_text {
		span.0 = shortcut_text;
	}
	span_color.set_if_neq(TextColor(color));

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
