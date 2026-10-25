use bevy::prelude::*;
use house_editor_core::UnsavedChoice;

use crate::{
	actions::ActionSender,
	document::project::{
		PathField, PendingReplacement, ProjectPath, ProjectRequest, ProjectStatus, UnsavedDecision,
	},
	input::hints::hint,
	ui::fields::path_field,
	ui::panel::{EditorUiSystems, step_button},
};

#[derive(Component)]
struct ProjectButton(ProjectRequest);

#[derive(Component)]
struct DecisionButton(UnsavedChoice);

#[derive(Component)]
struct ProjectStatusReadout;

#[derive(Component)]
struct UnsavedPrompt;

#[derive(Component)]
struct UnsavedPromptText;

pub(super) fn project_controls() -> impl Bundle {
	(
		Node {
			flex_direction: FlexDirection::Column,
			row_gap: Val::Px(8.0),
			..default()
		},
		children![
			hint("Project file ({Save} saves, {Open} opens)"),
			path_field(PathField),
			(
				Node {
					flex_direction: FlexDirection::Row,
					flex_wrap: FlexWrap::Wrap,
					row_gap: Val::Px(8.0),
					column_gap: Val::Px(8.0),
					..default()
				},
				children![
					(step_button("New"), ProjectButton(ProjectRequest::New)),
					(step_button("Open"), ProjectButton(ProjectRequest::Open)),
					(step_button("Save"), ProjectButton(ProjectRequest::Save)),
				],
			),
			(Text::new(""), ProjectStatusReadout),
			(
				Node {
					display: Display::None,
					flex_direction: FlexDirection::Column,
					row_gap: Val::Px(8.0),
					..default()
				},
				UnsavedPrompt,
				children![
					(Text::new("There are unsaved changes."), UnsavedPromptText),
					(
						Node {
							flex_direction: FlexDirection::Row,
							flex_wrap: FlexWrap::Wrap,
							row_gap: Val::Px(8.0),
							column_gap: Val::Px(8.0),
							..default()
						},
						children![
							(step_button("Save"), DecisionButton(UnsavedChoice::Save)),
							(
								step_button("Discard"),
								DecisionButton(UnsavedChoice::Discard)
							),
							(step_button("Cancel"), DecisionButton(UnsavedChoice::Cancel)),
						],
					),
				],
			),
		],
	)
}

fn request_project(
	buttons: Query<(&Interaction, &ProjectButton), Changed<Interaction>>,
	mut messages: ActionSender,
) {
	for (interaction, button) in &buttons {
		if *interaction == Interaction::Pressed {
			messages.send(button.0);
		}
	}
}

fn decide_unsaved(
	buttons: Query<(&Interaction, &DecisionButton), Changed<Interaction>>,
	mut messages: ActionSender,
) {
	for (interaction, button) in &buttons {
		if *interaction == Interaction::Pressed {
			messages.send(UnsavedDecision(button.0));
		}
	}
}

fn update_project_readouts(
	status: Res<ProjectStatus>,
	path: Res<ProjectPath>,
	pending: Res<PendingReplacement>,
	mut readout: Single<&mut Text, With<ProjectStatusReadout>>,
	mut prompt: Single<(&mut Node, &Children), With<UnsavedPrompt>>,
	mut prompt_text: Single<&mut Text, (With<UnsavedPromptText>, Without<ProjectStatusReadout>)>,
) {
	let description = match (&status.0[..], &path.0) {
		("", Some(path)) => format!("File: {}", path.display()),
		(message, _) => message.to_string(),
	};
	// Writing identical text would still trigger a text layout every frame.
	if readout.0 != description {
		readout.0 = description;
	}

	let (display, question) = match pending.pending_request() {
		None => (Display::None, ""),
		Some(ProjectRequest::New) => (Display::Flex, "Unsaved changes. Start a new project?"),
		Some(ProjectRequest::Open) => (Display::Flex, "Unsaved changes. Open another project?"),
		Some(ProjectRequest::Close) => (Display::Flex, "Unsaved changes. Close the editor?"),
		Some(ProjectRequest::Save) => (Display::None, ""),
	};
	if prompt.0.display != display {
		prompt.0.display = display;
	}
	if !question.is_empty() && prompt_text.0 != question {
		prompt_text.0 = question.to_string();
	}
}

pub(super) fn register(app: &mut App) {
	app.add_systems(
		Update,
		(request_project, decide_unsaved).in_set(EditorUiSystems::Settings),
	)
	.add_systems(
		PostUpdate,
		update_project_readouts.in_set(EditorUiSystems::Readouts),
	);
}
