use crate::{
	edit::{EditNotice, EditPreview, PreviewState, Proposal},
	history::HistoryRequest,
	house::House,
	opening_fields::{opening_field_controls, snap_gap_control},
	opening_ui,
	snapping::{SnapSettings, SnapSource, SnapTarget, SnappedPointer, snapping_disabled},
	wall_fields::wall_field_controls,
	wall_tool_active::{ActivateTool, ActiveTool, Tool},
	wall_tool_selection::{SelectedTarget, SelectionTarget},
	wall_ui,
};
use bevy::{
	input::mouse::{AccumulatedMouseScroll, MouseScrollUnit},
	input_focus::InputFocus,
	prelude::*,
	ui::{FocusPolicy, RelativeCursorPosition},
};

pub(crate) const ENABLED_BUTTON_COLOR: Color = Color::srgb(0.25, 0.25, 0.3);
const DISABLED_BUTTON_COLOR: Color = Color::srgb(0.15, 0.15, 0.17);

pub struct EditorUiPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditorUiSystems {
	Settings,
	Readouts,
}

impl Plugin for EditorUiPlugin {
	fn build(&self, app: &mut App) {
		crate::opening_ui::register(app);
		crate::wall_ui::register(app);
		crate::project_ui::register(app);
		app.add_systems(Startup, setup_editor_panel)
			.add_systems(
				Update,
				(
					step_grid_spacing,
					scroll_editor_panel,
					request_tool,
					request_history,
				)
					.in_set(EditorUiSystems::Settings),
			)
			.add_systems(
				PostUpdate,
				(
					update_grid_spacing_label.run_if(resource_changed::<SnapSettings>),
					update_snap_readout,
					update_edit_readout,
					update_selection_readout,
					update_history_controls,
					update_notice_readout,
					update_panel_sections,
					update_active_tool_label.run_if(resource_changed::<ActiveTool>),
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

fn setup_editor_panel(mut commands: Commands) {
	commands.spawn((
		Name::new("editor-panel"),
		EditorPanel,
		Node {
			position_type: PositionType::Absolute,
			left: Val::Px(0.0),
			top: Val::Px(0.0),
			width: Val::Px(240.0),
			max_width: Val::Percent(100.0),
			height: Val::Percent(100.0),
			padding: UiRect::all(Val::Px(12.0)),
			overflow: Overflow::scroll_y(),
			..default()
		},
		ScrollPosition::default(),
		RelativeCursorPosition::default(),
		BackgroundColor(Color::srgba(0.1, 0.1, 0.12, 0.9)),
		// Interaction makes the panel block viewport picking, including its blank space.
		Interaction::default(),
		FocusPolicy::Block,
		children![(
			Node {
				width: Val::Percent(100.0),
				flex_direction: FlexDirection::Column,
				flex_shrink: 0.0,
				row_gap: Val::Px(12.0),
				..default()
			},
			children![
				crate::project_ui::project_controls(),
				tool_controls(),
				history_controls(),
				snap_controls(),
				(Text::new(""), EditReadout),
				(Text::new(""), NoticeReadout),
				section(PanelSection::WallDrawing, wall_ui::wall_drawing_controls()),
				section(PanelSection::ArcDrawing, wall_ui::arc_drawing_controls()),
				section(
					PanelSection::OpeningPlacement,
					opening_ui::opening_controls()
				),
				section(
					PanelSection::SelectedOpening,
					(
						Node {
							flex_direction: FlexDirection::Column,
							row_gap: Val::Px(8.0),
							..default()
						},
						children![
							opening_ui::opening_edit_controls(),
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
	mut sections: Query<(Entity, &PanelSection, &mut Node)>,
	parents: Query<&ChildOf>,
	focus: Option<ResMut<InputFocus>>,
) {
	let mut focus = focus;
	for (entity, section, mut node) in &mut sections {
		let show = match section {
			PanelSection::WallDrawing => tool.0 == Tool::DrawWall,
			PanelSection::ArcDrawing => tool.0 == Tool::DrawArc,
			PanelSection::OpeningPlacement => tool.0 == Tool::PlaceOpening,
			PanelSection::SelectedOpening => tool.0 == Tool::Select && selected.opening().is_some(),
			PanelSection::SelectedWall => tool.0 == Tool::Select && selected.wall().is_some(),
		};
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
}

fn scroll_editor_panel(
	scroll: Res<AccumulatedMouseScroll>,
	mut panel: Single<
		(&RelativeCursorPosition, &ComputedNode, &mut ScrollPosition),
		With<EditorPanel>,
	>,
) {
	let (cursor, computed, position) = &mut *panel;
	if !cursor.cursor_over() {
		return;
	}
	let delta = match scroll.unit {
		MouseScrollUnit::Line => scroll.delta.y * 40.0,
		MouseScrollUnit::Pixel => scroll.delta.y,
	};
	let maximum = ((computed.content_size().y - computed.size().y)
		* computed.inverse_scale_factor())
	.max(0.0);
	position.y = (position.y - delta).clamp(0.0, maximum);
}

#[derive(Component)]
struct ToolButton(Tool);

#[derive(Component)]
struct SelectionReadout;

#[derive(Component)]
struct ActiveToolLabel;

fn tool_controls() -> impl Bundle {
	(
		Node {
			flex_direction: FlexDirection::Column,
			row_gap: Val::Px(8.0),
			..default()
		},
		children![
			(Text::new(""), ActiveToolLabel),
			(
				Node {
					flex_direction: FlexDirection::Row,
					flex_wrap: FlexWrap::Wrap,
					row_gap: Val::Px(8.0),
					column_gap: Val::Px(8.0),
					..default()
				},
				children![
					(step_button("Select"), ToolButton(Tool::Select)),
					(step_button("Draw wall"), ToolButton(Tool::DrawWall)),
					(step_button("Arc"), ToolButton(Tool::DrawArc)),
					(step_button("Opening"), ToolButton(Tool::PlaceOpening)),
					(step_button("Delete"), ToolButton(Tool::Delete)),
				],
			),
			(Text::new(""), SelectionReadout),
		],
	)
}

#[derive(Component)]
struct HistoryButton(HistoryRequest);

#[derive(Component)]
struct DocumentReadout;

fn history_controls() -> impl Bundle {
	(
		Node {
			flex_direction: FlexDirection::Column,
			row_gap: Val::Px(8.0),
			..default()
		},
		children![
			(
				Node {
					flex_direction: FlexDirection::Row,
					column_gap: Val::Px(8.0),
					..default()
				},
				children![
					(step_button("Undo"), HistoryButton(HistoryRequest::Undo)),
					(step_button("Redo"), HistoryButton(HistoryRequest::Redo)),
				],
			),
			(Text::new(""), DocumentReadout),
		],
	)
}

fn request_history(
	buttons: Query<(&Interaction, &HistoryButton), Changed<Interaction>>,
	mut messages: MessageWriter<HistoryRequest>,
) {
	for (interaction, button) in &buttons {
		if *interaction == Interaction::Pressed {
			messages.write(button.0);
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
			ENABLED_BUTTON_COLOR
		} else {
			DISABLED_BUTTON_COLOR
		};
		background.set_if_neq(BackgroundColor(color));
	}

	let description = if house.is_modified() { "Modified" } else { "" };
	// Writing identical text would still trigger a text layout every frame.
	if readout.0 != description {
		readout.0 = description.to_string();
	}
}

fn update_active_tool_label(
	active_tool: Res<ActiveTool>,
	mut label: Single<&mut Text, With<ActiveToolLabel>>,
) {
	label.0 = match active_tool.0 {
		Tool::Select => "Tool: Select",
		Tool::DrawWall => "Tool: Draw wall",
		Tool::DrawArc => "Tool: Draw arc",
		Tool::PlaceOpening => "Tool: Place opening",
		Tool::Delete => "Tool: Delete",
	}
	.to_string();
}

fn request_tool(
	buttons: Query<(&Interaction, &ToolButton), Changed<Interaction>>,
	mut messages: MessageWriter<ActivateTool>,
) {
	for (interaction, button) in &buttons {
		if *interaction == Interaction::Pressed {
			messages.write(ActivateTool(button.0));
		}
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
struct SnapReadout;

#[derive(Component)]
struct GridSpacingStep(isize);

fn snap_controls() -> impl Bundle {
	(
		Node {
			flex_direction: FlexDirection::Column,
			row_gap: Val::Px(8.0),
			..default()
		},
		children![
			Text::new("Snap priority: junction, wall, grid. Hold Shift to disable."),
			snap_gap_control(),
			(
				Node {
					flex_direction: FlexDirection::Row,
					flex_wrap: FlexWrap::Wrap,
					row_gap: Val::Px(8.0),
					align_items: AlignItems::Center,
					column_gap: Val::Px(8.0),
					..default()
				},
				children![
					(Text::new(""), GridSpacingLabel),
					grid_spacing_button("-", -1),
					grid_spacing_button("+", 1),
				],
			),
			(Text::new(""), SnapReadout),
		],
	)
}

fn grid_spacing_button(label: &str, step: isize) -> impl Bundle {
	(step_button(label), GridSpacingStep(step))
}

fn step_grid_spacing(
	mut settings: ResMut<SnapSettings>,
	buttons: Query<(&Interaction, &GridSpacingStep), Changed<Interaction>>,
) {
	for (interaction, step) in &buttons {
		if *interaction == Interaction::Pressed {
			settings.step_grid_spacing(step.0);
		}
	}
}

fn update_grid_spacing_label(
	settings: Res<SnapSettings>,
	mut label: Single<&mut Text, With<GridSpacingLabel>>,
) {
	label.0 = format!("Grid: {} m", settings.grid_spacing());
}

fn update_snap_readout(
	snapped_pointer: Res<SnappedPointer>,
	settings: Res<SnapSettings>,
	keyboard: Res<ButtonInput<KeyCode>>,
	mut readout: Single<&mut Text, With<SnapReadout>>,
) {
	let description = match snapped_pointer.target {
		None => "Snap: pointer is not on the ground".to_string(),
		Some(SnapTarget { position, source }) => {
			let source = match source {
				SnapSource::Junction(_) => "junction".to_string(),
				SnapSource::Wall(_) => "wall".to_string(),
				SnapSource::Grid => format!("grid ({} m)", settings.grid_spacing()),
				SnapSource::Free if snapping_disabled(&keyboard) => {
					"free (snapping off)".to_string()
				}
				SnapSource::Free => "free".to_string(),
			};
			format!(
				"Snap: {source}\nx {:.3} m, y {:.3} m",
				position.x, position.y
			)
		}
	};

	// Writing identical text would still trigger a text layout every frame.
	if readout.0 != description {
		readout.0 = description;
	}
}

pub(super) fn step_button(label: &str) -> impl Bundle {
	(
		Button,
		Node {
			padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)),
			..default()
		},
		BackgroundColor(ENABLED_BUTTON_COLOR),
		children![Text::new(label)],
	)
}

#[cfg(test)]
#[path = "tests/editor_ui.rs"]
mod tests;
