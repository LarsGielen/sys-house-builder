use crate::{
	actions::ActionSender,
	document::edit::{EditError, OpeningEditProposal, OpeningProposal},
	document::house::House,
	tools::edit_opening::{DeleteSelectedOpening, DuplicateSelectedOpening},
	ui::icons::{Icon, Icons},
	ui::panel::{EditorUiSystems, icon_button},
	ui::theme,
};
use bevy::prelude::*;
use house_document::OpeningCategory;
use wall_graph::OpeningId;

pub(super) fn describe_opening(house: &House, id: OpeningId) -> Option<String> {
	let graph = house.graph();
	let opening = graph.opening(id)?;
	let position = graph.opening_position(id)?;
	let category = house.committed().categories().get(id)?;
	Some(format!(
		"Selected {}\nAt x {:.3} m, y {:.3} m\n{:.3} m along the wall\n\
		 {:.2} m wide, {:.2} m high, bottom {:.2} m",
		category_name(category).to_lowercase(),
		position.x,
		position.y,
		opening.spec.center_distance,
		opening.spec.width,
		opening.spec.height,
		opening.spec.bottom
	))
}

pub(super) fn category_name(category: OpeningCategory) -> &'static str {
	match category {
		OpeningCategory::Door => "Door",
		OpeningCategory::Window => "Window",
	}
}

#[derive(Clone, Copy)]
enum OpeningCommand {
	Duplicate,
	Delete,
}

#[derive(Component)]
struct OpeningCommandButton(OpeningCommand);

pub(super) fn opening_edit_controls(icons: &Icons) -> impl Bundle {
	(
		Node {
			column_gap: Val::Px(8.0),
			..default()
		},
		children![
			(
				icon_button(
					icons,
					Icon::Duplicate,
					Some("Duplicate"),
					theme::BUTTON,
					theme::TEXT
				),
				OpeningCommandButton(OpeningCommand::Duplicate)
			),
			(
				icon_button(
					icons,
					Icon::Delete,
					Some("Delete"),
					theme::BUTTON,
					theme::TEXT
				),
				OpeningCommandButton(OpeningCommand::Delete)
			),
		],
	)
}

fn request_opening_command(
	buttons: Query<(&Interaction, &OpeningCommandButton), Changed<Interaction>>,
	mut actions: ActionSender,
) {
	for (interaction, button) in &buttons {
		if *interaction != Interaction::Pressed {
			continue;
		}
		match button.0 {
			OpeningCommand::Duplicate => {
				actions.send(DuplicateSelectedOpening);
			}
			OpeningCommand::Delete => {
				actions.send(DeleteSelectedOpening);
			}
		}
	}
}

pub(super) fn describe_opening_removal(
	house: &House,
	opening: OpeningId,
	error: Option<&EditError>,
) -> String {
	let category = house
		.committed()
		.categories()
		.get(opening)
		.map_or("opening", category_name)
		.to_lowercase();
	match error {
		None => format!("Delete this {category}?\nClick deletes"),
		Some(error) => format!("This {category} cannot be deleted: {error}"),
	}
}

pub(super) fn describe_opening_edit(
	house: &House,
	proposal: &OpeningEditProposal,
	error: Option<&EditError>,
) -> String {
	let category = house
		.committed()
		.categories()
		.get(proposal.opening)
		.map_or("opening", category_name)
		.to_lowercase();
	let spec = proposal.spec;
	let size = format!(
		"{:.2} m wide, {:.2} m high, bottom {:.2} m",
		spec.width, spec.height, spec.bottom
	);
	match error {
		None => format!(
			"Edited {category}\n{:.3} m along the wall\n{size}",
			spec.center_distance
		),
		Some(error) => format!(
			"Invalid {category} edit at {:.3} m along the wall\n{size}: {error}",
			spec.center_distance
		),
	}
}

pub(super) fn describe_opening_proposal(
	proposal: &OpeningProposal,
	error: Option<&EditError>,
) -> String {
	let spec = proposal.spec;
	let size = format!(
		"{:.2} m wide, {:.2} m high, bottom {:.2} m",
		spec.width, spec.height, spec.bottom
	);
	let category = category_name(proposal.category);
	match error {
		None => format!(
			"{category} {size}\n{:.3} m along the wall",
			spec.center_distance
		),
		Some(error) => format!(
			"Invalid {} at {:.3} m along the wall\n{size}: {error}",
			category.to_lowercase(),
			spec.center_distance
		),
	}
}

pub(super) fn register(app: &mut App) {
	app.add_systems(
		Update,
		request_opening_command.in_set(EditorUiSystems::Settings),
	);
}
