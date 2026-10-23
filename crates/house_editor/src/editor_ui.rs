use bevy::{prelude::*, ui::FocusPolicy};

use crate::{
	edit::{EditError, EditPreview, OpeningProposal, PreviewState, Proposal, WallProposal},
	house::House,
	snapping::{SnapSettings, SnapSource, SnapTarget, SnappedPointer, snapping_disabled},
	wall_tool_active::{ActivateTool, ActiveTool, Tool},
	wall_tool_drawing::{StepDirection, WallDrawingSettings},
	wall_tool_selection::SelectedWall,
};

pub struct EditorUiPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditorUiSystems {
	Settings,
	Readouts,
}

impl Plugin for EditorUiPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Startup, setup_editor_panel)
			.add_systems(
				Update,
				(step_grid_spacing, step_dimensions, request_tool)
					.in_set(EditorUiSystems::Settings),
			)
			.add_systems(
				PostUpdate,
				(
					update_grid_spacing_label.run_if(resource_changed::<SnapSettings>),
					update_dimension_labels.run_if(resource_changed::<WallDrawingSettings>),
					update_snap_readout,
					update_wall_readout,
					update_selection_readout,
					update_active_tool_label.run_if(resource_changed::<ActiveTool>),
				)
					.in_set(EditorUiSystems::Readouts),
			);
	}
}

fn setup_editor_panel(mut commands: Commands) {
	commands
		.spawn((
			Name::new("editor-panel"),
			Node {
				position_type: PositionType::Absolute,
				left: Val::Px(0.0),
				top: Val::Px(0.0),
				width: Val::Px(240.0),
				height: Val::Percent(100.0),
				padding: UiRect::all(Val::Px(12.0)),
				flex_direction: FlexDirection::Column,
				row_gap: Val::Px(12.0),
				..default()
			},
			BackgroundColor(Color::srgba(0.1, 0.1, 0.12, 0.9)),
			// Without `Interaction` the node is never hover-tested, and `Block` stops
			// the pointer from also counting as over anything underneath.
			Interaction::default(),
			FocusPolicy::Block,
		))
		.with_child((
			Button,
			Node {
				padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
				..default()
			},
			BackgroundColor(Color::srgb(0.25, 0.25, 0.3)),
			children![Text::new("Test button")],
		))
		.with_child(tool_controls())
		.with_child(snap_controls())
		.with_child(wall_drawing_controls());
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
					column_gap: Val::Px(8.0),
					..default()
				},
				children![
					(step_button("Select"), ToolButton(Tool::Select)),
					(step_button("Draw wall"), ToolButton(Tool::DrawWall)),
					(step_button("Opening"), ToolButton(Tool::PlaceOpening)),
				],
			),
			(Text::new(""), SelectionReadout),
		],
	)
}

fn update_active_tool_label(
	active_tool: Res<ActiveTool>,
	mut label: Single<&mut Text, With<ActiveToolLabel>>,
) {
	label.0 = match active_tool.0 {
		Tool::Select => "Tool: Select",
		Tool::DrawWall => "Tool: Draw wall",
		Tool::PlaceOpening => "Tool: Place opening",
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
	selected: Res<SelectedWall>,
	mut readout: Single<&mut Text, With<SelectionReadout>>,
) {
	let graph = house.graph();
	// Each query fails for a stale handle, so a stale selection shows nothing.
	let description = selected
		.0
		.and_then(|wall| {
			let dimensions = graph.wall_dimensions(wall)?;
			let length = graph.wall_length(wall)?;
			let start = graph.node_position(wall.origin())?;
			let end = graph.node_position(wall.destination())?;
			Some(format!(
				"Selected wall\nLength {length:.3} m\nHeight {:.2} m, thickness {:.2} m\n\
				 From x {:.3} m, y {:.3} m\nTo x {:.3} m, y {:.3} m",
				dimensions.height, dimensions.thickness, start.x, start.y, end.x, end.y
			))
		})
		.unwrap_or_default();

	// Writing identical text would still trigger a text layout every frame.
	if readout.0 != description {
		readout.0 = description;
	}
}

#[derive(Clone, Copy)]
enum DimensionField {
	Height,
	Thickness,
}

#[derive(Component)]
struct DimensionStep {
	field: DimensionField,
	direction: StepDirection,
}

#[derive(Component)]
struct DimensionLabel(DimensionField);

#[derive(Component)]
struct WallReadout;

fn wall_drawing_controls() -> impl Bundle {
	(
		Node {
			flex_direction: FlexDirection::Column,
			row_gap: Val::Px(8.0),
			..default()
		},
		children![
			Text::new("Wall drawing: click twice, or drag. Esc cancels."),
			dimension_row(DimensionField::Height),
			dimension_row(DimensionField::Thickness),
			(Text::new(""), WallReadout),
		],
	)
}

fn dimension_row(field: DimensionField) -> impl Bundle {
	(
		Node {
			flex_direction: FlexDirection::Row,
			align_items: AlignItems::Center,
			column_gap: Val::Px(8.0),
			..default()
		},
		children![
			(Text::new(""), DimensionLabel(field)),
			dimension_button("-", field, StepDirection::Decrease),
			dimension_button("+", field, StepDirection::Increase),
		],
	)
}

fn dimension_button(label: &str, field: DimensionField, direction: StepDirection) -> impl Bundle {
	(step_button(label), DimensionStep { field, direction })
}

fn step_dimensions(
	mut settings: ResMut<WallDrawingSettings>,
	buttons: Query<(&Interaction, &DimensionStep), Changed<Interaction>>,
) {
	for (interaction, step) in &buttons {
		if *interaction != Interaction::Pressed {
			continue;
		}
		match step.field {
			DimensionField::Height => settings.step_height(step.direction),
			DimensionField::Thickness => settings.step_thickness(step.direction),
		}
	}
}

fn update_dimension_labels(
	settings: Res<WallDrawingSettings>,
	mut labels: Query<(&DimensionLabel, &mut Text)>,
) {
	for (label, mut text) in &mut labels {
		text.0 = match label.0 {
			DimensionField::Height => format!("Height: {:.2} m", settings.dimensions().height),
			DimensionField::Thickness => {
				format!("Thickness: {:.2} m", settings.dimensions().thickness)
			}
		};
	}
}

fn update_wall_readout(
	preview: Res<EditPreview>,
	mut readout: Single<&mut Text, With<WallReadout>>,
) {
	let description = match &preview.state {
		None => String::new(),
		Some(PreviewState {
			proposal: Proposal::Wall(proposal),
			outcome,
			..
		}) => describe_wall_proposal(proposal, outcome.as_ref().err()),
		Some(PreviewState {
			proposal: Proposal::Opening(proposal),
			outcome,
			..
		}) => describe_opening_proposal(proposal, outcome.as_ref().err()),
	};

	// Writing identical text would still trigger a text layout every frame.
	if readout.0 != description {
		readout.0 = description;
	}
}

fn describe_wall_proposal(proposal: &WallProposal, error: Option<&EditError>) -> String {
	let length = proposal.start.distance(proposal.end);
	match error {
		None => format!(
			"Length {length:.3} m\nHeight {:.2} m, thickness {:.2} m",
			proposal.dimensions.height, proposal.dimensions.thickness
		),
		Some(error) => format!("Invalid wall ({length:.3} m): {error}"),
	}
}

fn describe_opening_proposal(proposal: &OpeningProposal, error: Option<&EditError>) -> String {
	let spec = proposal.spec;
	let size = format!(
		"{:.2} m wide, {:.2} m high, bottom {:.2} m",
		spec.width, spec.height, spec.bottom
	);
	match error {
		None => format!(
			"Opening {size}\n{:.3} m along the wall",
			spec.center_distance
		),
		Some(error) => format!(
			"Invalid opening at {:.3} m along the wall\n{size}: {error}",
			spec.center_distance
		),
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
			(
				Node {
					flex_direction: FlexDirection::Row,
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

fn step_button(label: &str) -> impl Bundle {
	(
		Button,
		Node {
			padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)),
			..default()
		},
		BackgroundColor(Color::srgb(0.25, 0.25, 0.3)),
		children![Text::new(label)],
	)
}
