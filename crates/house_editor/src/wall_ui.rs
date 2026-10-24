use crate::{
	edit::{EditCandidate, EditError, WallDimensionsProposal, WallProposal},
	editor_ui::{EditorUiSystems, step_button},
	house::House,
	opening_category::OpeningCategory,
	wall_tool_active::Tool,
	wall_tool_drawing::{StepDirection, WallDrawingSettings},
	wall_tool_edit_wall::{ConfirmWallDeletion, DeleteSelectedWall},
};
use bevy::prelude::*;
use wall_graph::{Wall, WallGraph};

pub(super) fn describe_wall(graph: &WallGraph, wall: Wall) -> Option<String> {
	let dimensions = graph.wall_dimensions(wall)?;
	let length = graph.wall_length(wall)?;
	let start = graph.node_position(wall.origin())?;
	let end = graph.node_position(wall.destination())?;
	Some(format!(
		"Selected wall\nLength {length:.3} m\nHeight {:.2} m, thickness {:.2} m\n\
		 From x {:.3} m, y {:.3} m\nTo x {:.3} m, y {:.3} m",
		dimensions.height, dimensions.thickness, start.x, start.y, end.x, end.y
	))
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

pub(super) fn wall_drawing_controls() -> impl Bundle {
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

#[derive(Clone, Copy)]
enum WallCommand {
	Delete,
	ConfirmDelete,
}

#[derive(Component)]
struct WallCommandButton(WallCommand);

pub(super) fn wall_edit_controls() -> impl Bundle {
	(
		Node {
			flex_direction: FlexDirection::Row,
			column_gap: Val::Px(8.0),
			..default()
		},
		children![
			(
				step_button("Delete wall"),
				WallCommandButton(WallCommand::Delete)
			),
			(
				step_button("Confirm"),
				WallCommandButton(WallCommand::ConfirmDelete)
			),
		],
	)
}

fn request_wall_command(
	buttons: Query<(&Interaction, &WallCommandButton), Changed<Interaction>>,
	mut deletes: MessageWriter<DeleteSelectedWall>,
	mut confirmations: MessageWriter<ConfirmWallDeletion>,
) {
	for (interaction, button) in &buttons {
		if *interaction != Interaction::Pressed {
			continue;
		}
		match button.0 {
			WallCommand::Delete => {
				deletes.write(DeleteSelectedWall);
			}
			WallCommand::ConfirmDelete => {
				confirmations.write(ConfirmWallDeletion);
			}
		}
	}
}

pub(super) fn describe_wall_proposal(proposal: &WallProposal, error: Option<&EditError>) -> String {
	let length = proposal.start.distance(proposal.end);
	match error {
		None => format!(
			"Length {length:.3} m\nHeight {:.2} m, thickness {:.2} m",
			proposal.dimensions.height, proposal.dimensions.thickness
		),
		Some(error) => format!("Invalid wall ({length:.3} m): {error}"),
	}
}

pub(super) fn describe_wall_dimensions(
	proposal: &WallDimensionsProposal,
	error: Option<&EditError>,
) -> String {
	let size = format!(
		"{:.2} m high, {:.2} m thick",
		proposal.dimensions.height, proposal.dimensions.thickness
	);
	match error {
		None => format!("Wall would be {size}\nEnter applies, Esc reverts"),
		Some(error) => format!("Invalid wall change ({size}): {error}"),
	}
}

/// What deleting the wall takes with it, so it can be judged before it is confirmed.
pub(super) fn describe_wall_removal(
	house: &House,
	wall: Wall,
	result: Result<&EditCandidate, &EditError>,
	tool: Tool,
) -> String {
	let committed = house.committed();
	let mut doors = 0;
	let mut windows = 0;
	for (id, opening) in committed.graph().openings() {
		if opening.wall != wall {
			continue;
		}
		match committed.categories().get(id) {
			Some(OpeningCategory::Window) => windows += 1,
			_ => doors += 1,
		}
	}
	let result = match result {
		Ok(result) => result,
		Err(error) => return format!("This wall cannot be deleted: {error}"),
	};
	let how_to_confirm = match tool {
		Tool::Delete => "Click deletes",
		_ => "Enter or Confirm deletes, Esc cancels",
	};
	format!(
		"Delete this wall?\nWalls {} -> {}\nRemoves {doors} door(s) and {windows} window(s)\n\
		 {how_to_confirm}",
		committed.graph().walls().count(),
		result.graph().walls().count()
	)
}

pub(super) fn register(app: &mut App) {
	app.add_systems(
		Update,
		(step_dimensions, request_wall_command).in_set(EditorUiSystems::Settings),
	)
	.add_systems(
		PostUpdate,
		update_dimension_labels
			.run_if(resource_changed::<WallDrawingSettings>)
			.in_set(EditorUiSystems::Readouts),
	);
}
