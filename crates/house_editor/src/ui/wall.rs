use crate::{
	actions::ActionSender,
	actions::settings::{SettingsAction, SettingsActionPlugin},
	document::edit::{
		ArcProposal, BendProposal, EditCandidate, EditError, MoveProposal, WallDimensionsProposal,
		WallProposal,
	},
	document::house::House,
	input::hints::hint,
	tools::active::Tool,
	tools::arc::ArcModeState,
	tools::drawing::{StepDirection, WallDrawingSettings},
	tools::edit_wall::{ConfirmWallDeletion, DeleteSelectedWall},
	ui::arc_fields::arc_field_controls,
	ui::panel::{ENABLED_BUTTON_COLOR, EditorUiSystems, step_button},
};
use bevy::prelude::*;
use house_document::OpeningCategory;
use house_editor_core::ArcMode;
use wall_graph::{Wall, WallGraph, arc_radius};

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
			hint("Wall drawing: click twice, or drag. {Cancel} cancels."),
			dimension_row(DimensionField::Height),
			dimension_row(DimensionField::Thickness),
		],
	)
}

#[derive(Component)]
struct ArcModeButton(ArcMode);

#[derive(Component)]
struct ArcModeHint;

pub(super) fn arc_drawing_controls() -> impl Bundle {
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
					flex_wrap: FlexWrap::Wrap,
					row_gap: Val::Px(8.0),
					column_gap: Val::Px(8.0),
					..default()
				},
				children![
					(step_button("Free"), ArcModeButton(ArcMode::Free)),
					(step_button("1 tangent"), ArcModeButton(ArcMode::OneTangent)),
					(
						step_button("2 tangents"),
						ArcModeButton(ArcMode::TwoTangents)
					),
				],
			),
			(Text::new(""), ArcModeHint),
			dimension_row(DimensionField::Height),
			dimension_row(DimensionField::Thickness),
			arc_field_controls(),
		],
	)
}

fn choose_arc_mode(
	buttons: Query<(&Interaction, &ArcModeButton), Changed<Interaction>>,
	mut actions: ActionSender,
) {
	for (interaction, button) in &buttons {
		if *interaction == Interaction::Pressed {
			actions.send(SettingsAction::SetArcMode(button.0));
		}
	}
}

fn update_arc_mode_controls(
	mode: Res<ArcModeState>,
	mut buttons: Query<(&ArcModeButton, &mut BackgroundColor)>,
	mut hint: Single<&mut Text, With<ArcModeHint>>,
) {
	for (button, mut background) in &mut buttons {
		let color = if button.0 == mode.0 {
			Color::srgb(0.3, 0.45, 0.7)
		} else {
			ENABLED_BUTTON_COLOR
		};
		background.set_if_neq(BackgroundColor(color));
	}
	hint.0 = match mode.0 {
		ArcMode::Free => {
			"Free arc: place both ends like a wall, then move the pointer to bend it and click."
		}
		ArcMode::OneTangent => {
			"Click a free wall end, then the other end. The arc leaves the wall smoothly."
		}
		ArcMode::TwoTangents => {
			"Click a free wall end, then a second free wall end. Both joins are smooth."
		}
	}
	.to_string();
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
	mut actions: ActionSender,
	buttons: Query<(&Interaction, &DimensionStep), Changed<Interaction>>,
) {
	for (interaction, step) in &buttons {
		if *interaction != Interaction::Pressed {
			continue;
		}
		actions.send(match step.field {
			DimensionField::Height => SettingsAction::StepWallHeight(step.direction),
			DimensionField::Thickness => SettingsAction::StepWallThickness(step.direction),
		});
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
	mut actions: ActionSender,
) {
	for (interaction, button) in &buttons {
		if *interaction != Interaction::Pressed {
			continue;
		}
		match button.0 {
			WallCommand::Delete => {
				actions.send(DeleteSelectedWall);
			}
			WallCommand::ConfirmDelete => {
				actions.send(ConfirmWallDeletion);
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

pub(super) fn describe_arc_proposal(proposal: &ArcProposal, error: Option<&EditError>) -> String {
	let derived =
		proposal.sweep().ok().map(
			|sweep| match arc_radius(proposal.start, proposal.end, sweep) {
				Some(radius) => {
					format!("Radius {radius:.3} m, angle {:.1} deg", sweep.to_degrees())
				}
				None => "Straight".to_string(),
			},
		);
	match (error, derived) {
		(None, Some(derived)) => format!(
			"{derived}\nHeight {:.2} m, thickness {:.2} m",
			proposal.dimensions.height, proposal.dimensions.thickness
		),
		(Some(error), Some(derived)) => format!("Invalid arc ({derived}): {error}"),
		(Some(error), None) => format!("Invalid arc: {error}"),
		(None, None) => String::new(),
	}
}

/// The effect of a drag on the layout, including any smooth join it breaks.
fn describe_reshape(outcome: Result<&EditCandidate, &EditError>, what: &str) -> String {
	match outcome {
		Err(error) => format!("Cannot {what}: {error}"),
		Ok(candidate) => {
			let report = candidate.report();
			let joins = match report.lost_smooth_joins.len() {
				0 => "Smooth joins kept".to_string(),
				count => format!("Smooth join lost at {count} junction(s), marked in orange"),
			};
			format!(
				"{} wall(s) change\n{joins}\nRelease applies, {{Cancel}} cancels",
				report.rebuilt_walls.len()
			)
		}
	}
}

pub(super) fn describe_move(
	_proposal: &MoveProposal,
	outcome: Result<&EditCandidate, &EditError>,
) -> String {
	describe_reshape(outcome, "move this")
}

pub(super) fn describe_bend(
	proposal: &BendProposal,
	outcome: Result<&EditCandidate, &EditError>,
) -> String {
	let bend = format!("Sweep {:.1} deg", proposal.sweep.to_degrees());
	format!("{bend}\n{}", describe_reshape(outcome, "bend this wall"))
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
		None => format!("Wall would be {size}\n{{FieldSubmit}} applies, {{FieldCancel}} reverts"),
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
		_ => "{Confirm} or Confirm deletes, {Cancel} cancels",
	};
	format!(
		"Delete this wall?\nWalls {} -> {}\nRemoves {doors} door(s) and {windows} window(s)\n\
		 {how_to_confirm}",
		committed.graph().walls().count(),
		result.graph().walls().count()
	)
}

pub(super) fn register(app: &mut App) {
	SettingsActionPlugin::ensure(app);
	app.add_systems(
		Update,
		(step_dimensions, request_wall_command, choose_arc_mode).in_set(EditorUiSystems::Settings),
	)
	.add_systems(
		PostUpdate,
		update_dimension_labels
			.run_if(resource_changed::<WallDrawingSettings>)
			.in_set(EditorUiSystems::Readouts),
	)
	.add_systems(
		PostUpdate,
		update_arc_mode_controls.in_set(EditorUiSystems::Readouts),
	);
}
