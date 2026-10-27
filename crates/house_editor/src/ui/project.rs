use bevy::prelude::*;
use house_editor_core::UnsavedChoice;

use crate::{
	actions::ActionSender,
	document::project::{
		PendingReplacement, ProjectPath, ProjectRequest, ProjectStatus, UnsavedDecision,
	},
	ui::icons::{Icon, Icons, icon_image},
	ui::panel::{EditorUiSystems, icon_button, pill, step_button},
	ui::theme,
};

#[derive(Component)]
struct ProjectButton(ProjectRequest);

#[derive(Component)]
struct DecisionButton(UnsavedChoice);

#[derive(Component)]
struct ProjectStatusReadout;

#[derive(Component)]
struct HouseNameReadout;

#[derive(Component)]
struct UnsavedPrompt;

#[derive(Component)]
struct UnsavedPromptText;

/// The app's title and the open house, shown at the left of the top bar.
pub(super) fn project_file_controls(icons: &Icons) -> impl Bundle {
	(
		Node {
			flex_direction: FlexDirection::Row,
			align_items: AlignItems::Center,
			column_gap: Val::Px(12.0),
			..default()
		},
		children![
			icon_image(icons, Icon::Home, 28.0),
			(Text::new("House editor"), TextFont::from_font_size(20.0)),
			(Text::new(""), HouseNameReadout),
			crate::ui::library::library_button(icons),
		],
	)
}

pub(super) fn save_button(icons: &Icons) -> impl Bundle {
	(
		icon_button(
			icons,
			Icon::Save,
			Some("Save"),
			theme::ACCENT,
			theme::TEXT_ON_ACCENT,
		),
		ProjectButton(ProjectRequest::Save),
	)
}

/// File status and the question asked before unsaved changes are replaced.
pub(super) fn project_notices() -> impl Bundle {
	(
		Node {
			flex_direction: FlexDirection::Column,
			align_items: AlignItems::Center,
			row_gap: Val::Px(8.0),
			..default()
		},
		children![
			(pill(""), ProjectStatusReadout),
			(
				Node {
					display: Display::None,
					flex_direction: FlexDirection::Column,
					row_gap: Val::Px(8.0),
					padding: UiRect::all(Val::Px(12.0)),
					border: UiRect::all(Val::Px(1.0)),
					border_radius: BorderRadius::all(theme::RADIUS),
					..default()
				},
				BackgroundColor(theme::PANEL),
				BorderColor::from(theme::PANEL_BORDER),
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

type PromptTextFilter = (
	With<UnsavedPromptText>,
	Without<ProjectStatusReadout>,
	Without<HouseNameReadout>,
);

fn update_project_readouts(
	status: Res<ProjectStatus>,
	path: Res<ProjectPath>,
	pending: Res<PendingReplacement>,
	mut readout: Single<&mut Text, With<ProjectStatusReadout>>,
	mut house_name: Single<&mut Text, (With<HouseNameReadout>, Without<ProjectStatusReadout>)>,
	mut prompt: Single<(&mut Node, &Children), With<UnsavedPrompt>>,
	mut prompt_text: Single<&mut Text, PromptTextFilter>,
) {
	let name = path
		.0
		.as_deref()
		.and_then(|path| path.file_stem())
		.map(|name| name.to_string_lossy().into_owned())
		.unwrap_or_default();
	if house_name.0 != name {
		house_name.0 = name;
	}
	let description = status.0.clone();
	// Writing identical text would still trigger a text layout every frame.
	if readout.0 != description {
		readout.0 = description;
	}

	let (display, question) = match pending.pending_request() {
		None => (Display::None, ""),
		Some(ProjectRequest::New) => (Display::Flex, "Unsaved changes. Create a new house?"),
		Some(ProjectRequest::Open) => (Display::Flex, "Unsaved changes. Open another house?"),
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

/// How long a file message stays after the file action that caused it.
const STATUS_SECONDS: f32 = 4.0;

/// Clears the status message once it has been shown for a while, so a past save does not stay
/// on screen for the rest of the session.
fn expire_status(
	time: Res<Time>,
	mut status: ResMut<ProjectStatus>,
	mut shown: Local<(String, f32)>,
) {
	if status.0 != shown.0 {
		*shown = (status.0.clone(), time.elapsed_secs());
	} else if !status.0.is_empty() && time.elapsed_secs() - shown.1 >= STATUS_SECONDS {
		status.0.clear();
	}
}

pub(super) fn register(app: &mut App) {
	app.add_systems(
		Update,
		(request_project, decide_unsaved, expire_status).in_set(EditorUiSystems::Settings),
	)
	.add_systems(
		PostUpdate,
		update_project_readouts.in_set(EditorUiSystems::Readouts),
	);
}
