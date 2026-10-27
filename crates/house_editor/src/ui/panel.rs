use crate::ui::{opening as opening_ui, wall as wall_ui};
use crate::{
	actions::ActionSender,
	actions::EditorActionPlugin,
	actions::settings::{SettingsAction, SettingsActionPlugin},
	document::edit::{EditNotice, EditPreview, PreviewState, Proposal},
	document::history::HistoryRequest,
	document::house::House,
	input::{ActionState, ActionsPlugin, Bindings, SCROLL_PIXELS_PER_LINE},
	tools::active::{ActiveTool, Tool},
	tools::selection::{SelectedTarget, SelectionTarget},
	ui::icons::{Icon, Icons, icon_image},
	ui::opening_fields::{opening_field_controls, snap_gap_control},
	ui::theme,
	ui::wall_fields::wall_field_controls,
	viewport::snapping::SnapSettings,
};
use bevy::{
	input_focus::InputFocus,
	prelude::*,
	ui::{FocusPolicy, RelativeCursorPosition},
};

/// Height of the top bar, which the floating panels sit below.
const TOP_BAR_HEIGHT: f32 = 56.0;
const FLOATING_MARGIN: f32 = 12.0;

pub struct EditorUiPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditorUiSystems {
	Settings,
	Readouts,
}

impl Plugin for EditorUiPlugin {
	fn build(&self, app: &mut App) {
		EditorActionPlugin::ensure(app);
		ActionsPlugin::ensure(app);
		SettingsActionPlugin::ensure(app);
		crate::ui::opening::register(app);
		crate::ui::wall::register(app);
		crate::ui::project::register(app);
		crate::ui::library::register(app);
		crate::ui::icons::register(app);
		crate::ui::dock::register(app);
		app.add_systems(Startup, setup_editor_panel)
			.add_systems(
				Update,
				(step_grid_spacing, scroll_editor_panel, request_history)
					.in_set(EditorUiSystems::Settings),
			)
			.add_systems(
				PostUpdate,
				(
					update_grid_spacing_label.run_if(resource_changed::<SnapSettings>),
					update_edit_readout,
					crate::input::hints::fill_hints,
					update_selection_readout,
					update_history_controls,
					update_notice_readout,
					update_panel_sections,
					hide_empty_pills,
				)
					.in_set(EditorUiSystems::Readouts),
			);
	}
}

#[derive(Component)]
struct EditorPanel;

#[derive(Component, Clone, Copy)]
enum PanelSection {
	WallDrawing,
	ArcDrawing,
	OpeningPlacement,
	SelectedOpening,
	SelectedWall,
}

#[derive(Component)]
struct EditReadout;

#[derive(Component)]
struct NoticeReadout;

fn setup_editor_panel(mut commands: Commands, icons: Res<Icons>) {
	commands.insert_resource(ClearColor(theme::VIEWPORT));
	commands.spawn((
		Name::new("top-bar"),
		chrome_bar(),
		Node {
			position_type: PositionType::Absolute,
			top: Val::Px(0.0),
			left: Val::Px(0.0),
			width: Val::Percent(100.0),
			height: Val::Px(TOP_BAR_HEIGHT),
			padding: UiRect::horizontal(Val::Px(16.0)),
			align_items: AlignItems::Center,
			justify_content: JustifyContent::SpaceBetween,
			..default()
		},
		children![
			crate::ui::project::project_file_controls(&icons),
			(
				Node {
					align_items: AlignItems::Center,
					column_gap: Val::Px(8.0),
					..default()
				},
				children![
					(
						icon_button(&icons, Icon::Undo, None, theme::BUTTON, theme::TEXT),
						HistoryButton(HistoryRequest::Undo)
					),
					(
						icon_button(&icons, Icon::Redo, None, theme::BUTTON, theme::TEXT),
						HistoryButton(HistoryRequest::Redo)
					),
					(Text::new(""), DocumentReadout),
					crate::ui::project::save_button(&icons),
				],
			),
		],
	));

	commands.spawn((
		Name::new("grid-bar"),
		floating_panel(),
		Node {
			position_type: PositionType::Absolute,
			top: Val::Px(TOP_BAR_HEIGHT + FLOATING_MARGIN),
			left: Val::Px(FLOATING_MARGIN),
			..floating_node()
		},
		children![grid_controls(&icons)],
	));

	commands.spawn((
		Name::new("editor-panel"),
		EditorPanel,
		floating_panel(),
		Node {
			position_type: PositionType::Absolute,
			top: Val::Px(TOP_BAR_HEIGHT + FLOATING_MARGIN),
			right: Val::Px(FLOATING_MARGIN),
			width: Val::Px(280.0),
			max_width: Val::Percent(100.0),
			max_height: Val::Percent(60.0),
			overflow: Overflow::scroll_y(),
			..floating_node()
		},
		ScrollPosition::default(),
		RelativeCursorPosition::default(),
		children![(
			Node {
				width: Val::Percent(100.0),
				flex_direction: FlexDirection::Column,
				flex_shrink: 0.0,
				row_gap: Val::Px(12.0),
				..default()
			},
			children![
				(Text::new(""), SelectionReadout),
				section(PanelSection::WallDrawing, wall_ui::wall_drawing_controls()),
				section(PanelSection::ArcDrawing, wall_ui::arc_drawing_controls()),
				section(PanelSection::OpeningPlacement, snap_gap_control()),
				section(
					PanelSection::SelectedOpening,
					(
						Node {
							flex_direction: FlexDirection::Column,
							row_gap: Val::Px(8.0),
							..default()
						},
						children![
							opening_ui::opening_edit_controls(&icons),
							opening_field_controls()
						],
					)
				),
				section(
					PanelSection::SelectedWall,
					(
						Node {
							flex_direction: FlexDirection::Column,
							row_gap: Val::Px(8.0),
							..default()
						},
						children![wall_ui::wall_edit_controls(), wall_field_controls()],
					)
				),
			],
		)],
	));

	// Not blocking the viewport: only the pills themselves are visible, and they are not
	// clickable.
	commands.spawn((
		Name::new("status-line"),
		Node {
			position_type: PositionType::Absolute,
			bottom: Val::Px(DOCK_HEIGHT + FLOATING_MARGIN),
			left: Val::Px(0.0),
			width: Val::Percent(100.0),
			flex_direction: FlexDirection::Column,
			align_items: AlignItems::Center,
			row_gap: Val::Px(8.0),
			..default()
		},
		children![
			crate::ui::project::project_notices(),
			(pill(""), NoticeReadout),
			(pill(""), EditReadout),
		],
	));

	commands.spawn((
		Name::new("tool-dock"),
		chrome_bar(),
		Node {
			position_type: PositionType::Absolute,
			bottom: Val::Px(0.0),
			left: Val::Px(0.0),
			width: Val::Percent(100.0),
			min_height: Val::Px(DOCK_HEIGHT),
			padding: UiRect::all(Val::Px(FLOATING_MARGIN)),
			..default()
		},
		children![crate::ui::dock::dock_controls(&icons)],
	));
}

const DOCK_HEIGHT: f32 = 120.0;

/// Top bar and dock: solid, and blocks the viewport behind it.
fn chrome_bar() -> impl Bundle {
	(
		BackgroundColor(theme::CHROME),
		// Interaction makes the bar block viewport picking, including its blank space.
		Interaction::default(),
		FocusPolicy::Block,
	)
}

/// A rounded panel floating over the viewport, which it blocks like the bars do.
fn floating_panel() -> impl Bundle {
	(
		BackgroundColor(theme::PANEL),
		BorderColor::from(theme::PANEL_BORDER),
		Interaction::default(),
		FocusPolicy::Block,
	)
}

fn floating_node() -> Node {
	Node {
		padding: UiRect::all(Val::Px(12.0)),
		border: UiRect::all(Val::Px(1.0)),
		border_radius: BorderRadius::all(theme::RADIUS),
		..default()
	}
}

fn section(kind: PanelSection, contents: impl Bundle) -> impl Bundle {
	(
		kind,
		Node {
			display: Display::None,
			flex_shrink: 0.0,
			..default()
		},
		children![contents],
	)
}

fn update_panel_sections(
	tool: Res<ActiveTool>,
	selected: Res<SelectedTarget>,
	mut sections: Query<(Entity, &PanelSection, &mut Node), Without<EditorPanel>>,
	mut panel: Single<&mut Node, With<EditorPanel>>,
	parents: Query<&ChildOf>,
	focus: Option<ResMut<InputFocus>>,
) {
	let mut focus = focus;
	// The properties panel only appears while one of its own sections has something to show.
	let mut panel_has_content = false;
	for (entity, section, mut node) in &mut sections {
		let show = match section {
			PanelSection::WallDrawing => tool.0 == Tool::DrawWall,
			PanelSection::ArcDrawing => tool.0 == Tool::DrawArc,
			PanelSection::OpeningPlacement => tool.0 == Tool::PlaceOpening,
			PanelSection::SelectedOpening => tool.0 == Tool::Select && selected.opening().is_some(),
			PanelSection::SelectedWall => tool.0 == Tool::Select && selected.wall().is_some(),
		};
		panel_has_content |= show;
		let display = if show { Display::Flex } else { Display::None };
		if node.display != display {
			node.display = display;
		}
		if !show && let Some(focus) = focus.as_mut() {
			let mut ancestor = focus.get();
			while let Some(current) = ancestor {
				if current == entity {
					focus.clear();
					break;
				}
				ancestor = parents.get(current).ok().map(ChildOf::parent);
			}
		}
	}
	let display = if panel_has_content {
		Display::Flex
	} else {
		Display::None
	};
	if panel.display != display {
		panel.display = display;
	}
}

fn scroll_editor_panel(
	actions: Res<ActionState>,
	mut panel: Single<
		(&RelativeCursorPosition, &ComputedNode, &mut ScrollPosition),
		With<EditorPanel>,
	>,
) {
	let (cursor, computed, position) = &mut *panel;
	if !cursor.cursor_over() {
		return;
	}
	let delta = actions.scroll_lines() * SCROLL_PIXELS_PER_LINE;
	let maximum = ((computed.content_size().y - computed.size().y)
		* computed.inverse_scale_factor())
	.max(0.0);
	position.y = (position.y - delta).clamp(0.0, maximum);
}

#[derive(Component)]
struct SelectionReadout;

#[derive(Component)]
struct HistoryButton(HistoryRequest);

#[derive(Component)]
struct DocumentReadout;

fn request_history(
	buttons: Query<(&Interaction, &HistoryButton), Changed<Interaction>>,
	mut messages: ActionSender,
) {
	for (interaction, button) in &buttons {
		if *interaction == Interaction::Pressed {
			messages.send(button.0);
		}
	}
}

/// Buttons with no history to step through are dimmed; pressing one does nothing.
fn update_history_controls(
	house: Res<House>,
	mut buttons: Query<(&HistoryButton, &mut BackgroundColor)>,
	mut readout: Single<&mut Text, With<DocumentReadout>>,
) {
	for (button, mut background) in &mut buttons {
		let available = match button.0 {
			HistoryRequest::Undo => house.can_undo(),
			HistoryRequest::Redo => house.can_redo(),
		};
		let color = if available {
			theme::BUTTON
		} else {
			theme::BUTTON_DISABLED
		};
		background.set_if_neq(BackgroundColor(color));
	}

	let description = if house.is_modified() { "Modified" } else { "" };
	// Writing identical text would still trigger a text layout every frame.
	if readout.0 != description {
		readout.0 = description.to_string();
	}
}

fn update_selection_readout(
	house: Res<House>,
	selected: Res<SelectedTarget>,
	mut readout: Single<&mut Text, With<SelectionReadout>>,
) {
	let description = selected
		.0
		.and_then(|target| match target {
			SelectionTarget::Wall(wall) => wall_ui::describe_wall(house.graph(), wall),
			SelectionTarget::Opening(opening) => opening_ui::describe_opening(&house, opening),
		})
		.unwrap_or_default();

	// Writing identical text would still trigger a text layout every frame.
	if readout.0 != description {
		readout.0 = description;
	}
}

fn update_edit_readout(
	preview: Res<EditPreview>,
	house: Res<House>,
	active_tool: Res<ActiveTool>,
	bindings: Res<Bindings>,
	mut readout: Single<&mut Text, With<EditReadout>>,
) {
	let description = match &preview.state {
		None => String::new(),
		Some(PreviewState {
			proposal: Proposal::Wall(proposal),
			outcome,
			..
		}) => wall_ui::describe_wall_proposal(proposal, outcome.as_ref().err()),
		Some(PreviewState {
			proposal: Proposal::Arc(proposal),
			outcome,
			..
		}) => wall_ui::describe_arc_proposal(proposal, outcome.as_ref().err()),
		Some(PreviewState {
			proposal: Proposal::Move(proposal),
			outcome,
			..
		}) => wall_ui::describe_move(proposal, outcome.as_ref()),
		Some(PreviewState {
			proposal: Proposal::Bend(proposal),
			outcome,
			..
		}) => wall_ui::describe_bend(proposal, outcome.as_ref()),
		Some(PreviewState {
			proposal: Proposal::Opening(proposal),
			outcome,
			..
		}) => opening_ui::describe_opening_proposal(proposal, outcome.as_ref().err()),
		Some(PreviewState {
			proposal: Proposal::OpeningEdit(proposal),
			outcome,
			..
		}) => opening_ui::describe_opening_edit(&house, proposal, outcome.as_ref().err()),
		Some(PreviewState {
			proposal: Proposal::WallDimensions(proposal),
			outcome,
			..
		}) => wall_ui::describe_wall_dimensions(proposal, outcome.as_ref().err()),
		Some(PreviewState {
			proposal: Proposal::WallRemoval(wall),
			outcome,
			..
		}) => wall_ui::describe_wall_removal(&house, *wall, outcome.as_ref(), active_tool.0),
		Some(PreviewState {
			proposal: Proposal::OpeningRemoval(opening),
			outcome,
			..
		}) => opening_ui::describe_opening_removal(&house, *opening, outcome.as_ref().err()),
	};
	let description = bindings.fill(&description);

	// Writing identical text would still trigger a text layout every frame.
	if readout.0 != description {
		readout.0 = description;
	}
}

fn update_notice_readout(
	notice: Res<EditNotice>,
	mut readout: Single<&mut Text, With<NoticeReadout>>,
) {
	let description = notice.message.clone().unwrap_or_default();
	// Writing identical text would still trigger a text layout every frame.
	if readout.0 != description {
		readout.0 = description;
	}
}

#[derive(Component)]
struct GridSpacingLabel;

#[derive(Component)]
struct GridSpacingStep(isize);

fn grid_controls(icons: &Icons) -> impl Bundle {
	(
		Node {
			align_items: AlignItems::Center,
			column_gap: Val::Px(8.0),
			..default()
		},
		children![
			icon_image(icons, Icon::Grid, 24.0),
			Text::new("Grid"),
			(Text::new(""), GridSpacingLabel),
			grid_spacing_button(icons, Icon::Minus, -1),
			grid_spacing_button(icons, Icon::Plus, 1),
		],
	)
}

fn grid_spacing_button(icons: &Icons, icon: Icon, step: isize) -> impl Bundle {
	(
		icon_button(icons, icon, None, theme::BUTTON, theme::TEXT),
		GridSpacingStep(step),
	)
}

fn step_grid_spacing(
	mut actions: ActionSender,
	buttons: Query<(&Interaction, &GridSpacingStep), Changed<Interaction>>,
) {
	for (interaction, step) in &buttons {
		if *interaction == Interaction::Pressed {
			actions.send(SettingsAction::StepGridSpacing(step.0));
		}
	}
}

fn update_grid_spacing_label(
	settings: Res<SnapSettings>,
	mut label: Single<&mut Text, With<GridSpacingLabel>>,
) {
	label.0 = format!("{} m", settings.grid_spacing());
}

/// A text with a rounded backdrop that disappears while the text is empty.
#[derive(Component)]
pub(super) struct Pill;

pub(super) fn pill(text: impl Into<String>) -> impl Bundle {
	(
		Text::new(text),
		Pill,
		Node {
			padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
			border: UiRect::all(Val::Px(1.0)),
			border_radius: BorderRadius::all(Val::Px(20.0)),
			..default()
		},
		BackgroundColor(theme::PANEL),
		BorderColor::from(theme::PANEL_BORDER),
	)
}

fn hide_empty_pills(mut pills: Query<(&Text, &mut Node), With<Pill>>) {
	for (text, mut node) in &mut pills {
		let display = if text.0.is_empty() {
			Display::None
		} else {
			Display::Flex
		};
		if node.display != display {
			node.display = display;
		}
	}
}

pub(super) fn step_button(label: &str) -> impl Bundle {
	button(label, theme::BUTTON, theme::TEXT)
}

/// The primary action of a bar, such as saving.
pub(super) fn accent_button(label: &str) -> impl Bundle {
	button(label, theme::ACCENT, theme::TEXT_ON_ACCENT)
}

fn button(label: &str, background: Color, text: Color) -> impl Bundle {
	(
		Button,
		Node {
			padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
			border_radius: BorderRadius::all(theme::RADIUS),
			..default()
		},
		BackgroundColor(background),
		children![(Text::new(label), TextColor(text))],
	)
}

/// A button showing an icon, and beside it a label unless it is `None`.
pub(super) fn icon_button(
	icons: &Icons,
	icon: Icon,
	label: Option<&str>,
	background: Color,
	content: Color,
) -> impl Bundle {
	(
		Button,
		Node {
			padding: UiRect::axes(Val::Px(10.0), Val::Px(8.0)),
			align_items: AlignItems::Center,
			column_gap: Val::Px(8.0),
			border_radius: BorderRadius::all(theme::RADIUS),
			..default()
		},
		BackgroundColor(background),
		children![
			(
				Node {
					width: Val::Px(22.0),
					height: Val::Px(22.0),
					flex_shrink: 0.0,
					..default()
				},
				ImageNode::new(icons.get(icon)).with_color(content),
			),
			(
				Text::new(label.unwrap_or_default()),
				TextColor(content),
				Node {
					display: if label.is_some() {
						Display::Flex
					} else {
						Display::None
					},
					..default()
				},
			)
		],
	)
}

#[cfg(test)]
#[path = "../tests/ui/panel.rs"]
mod tests;
