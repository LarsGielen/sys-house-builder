use crate::{
	edit::{EditError, OpeningEditProposal, OpeningProposal},
	editor_ui::{EditorUiSystems, step_button},
	house::House,
	opening_category::OpeningCategory,
	wall_tool_edit_opening::{DeleteSelectedOpening, DuplicateSelectedOpening},
	wall_tool_opening::OpeningSettings,
};
use bevy::prelude::*;
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

#[derive(Component)]
struct OpeningCategoryButton(OpeningCategory);

#[derive(Component)]
struct OpeningCategoryLabel;

pub(super) fn category_name(category: OpeningCategory) -> &'static str {
	match category {
		OpeningCategory::Door => "Door",
		OpeningCategory::Window => "Window",
	}
}

pub(super) fn opening_controls() -> impl Bundle {
	(
		Node {
			flex_direction: FlexDirection::Column,
			row_gap: Val::Px(8.0),
			..default()
		},
		children![
			(Text::new(""), OpeningCategoryLabel),
			(
				Node {
					flex_direction: FlexDirection::Row,
					column_gap: Val::Px(8.0),
					..default()
				},
				children![
					(
						step_button(category_name(OpeningCategory::Door)),
						OpeningCategoryButton(OpeningCategory::Door)
					),
					(
						step_button(category_name(OpeningCategory::Window)),
						OpeningCategoryButton(OpeningCategory::Window)
					),
				],
			),
		],
	)
}

#[derive(Clone, Copy)]
enum OpeningCommand {
	Duplicate,
	Delete,
}

#[derive(Component)]
struct OpeningCommandButton(OpeningCommand);

pub(super) fn opening_edit_controls() -> impl Bundle {
	(
		Node {
			flex_direction: FlexDirection::Column,
			row_gap: Val::Px(8.0),
			..default()
		},
		children![(
			Node {
				flex_direction: FlexDirection::Row,
				column_gap: Val::Px(8.0),
				..default()
			},
			children![
				(
					step_button("Duplicate"),
					OpeningCommandButton(OpeningCommand::Duplicate)
				),
				(
					step_button("Delete"),
					OpeningCommandButton(OpeningCommand::Delete)
				),
			],
		),],
	)
}

fn request_opening_command(
	buttons: Query<(&Interaction, &OpeningCommandButton), Changed<Interaction>>,
	mut duplicates: MessageWriter<DuplicateSelectedOpening>,
	mut deletes: MessageWriter<DeleteSelectedOpening>,
) {
	for (interaction, button) in &buttons {
		if *interaction != Interaction::Pressed {
			continue;
		}
		match button.0 {
			OpeningCommand::Duplicate => {
				duplicates.write(DuplicateSelectedOpening);
			}
			OpeningCommand::Delete => {
				deletes.write(DeleteSelectedOpening);
			}
		}
	}
}

fn request_opening_category(
	buttons: Query<(&Interaction, &OpeningCategoryButton), Changed<Interaction>>,
	mut settings: ResMut<OpeningSettings>,
) {
	for (interaction, button) in &buttons {
		if *interaction == Interaction::Pressed {
			settings.set_category(button.0);
		}
	}
}

fn update_opening_category_label(
	settings: Res<OpeningSettings>,
	mut label: Single<&mut Text, With<OpeningCategoryLabel>>,
) {
	label.0 = format!("Opening type: {}", category_name(settings.category()));
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
		(request_opening_category, request_opening_command).in_set(EditorUiSystems::Settings),
	)
	.add_systems(
		PostUpdate,
		update_opening_category_label
			.run_if(resource_changed::<OpeningSettings>)
			.in_set(EditorUiSystems::Readouts),
	);
}
