//! The house library screen: every saved house as a card, and a card for adding one.
//!
//! The screen is an opaque overlay above the editor. Because it blocks the pointer, the
//! viewport and its tools ignore input while it is shown, like they do behind any other panel.

use std::time::SystemTime;

use bevy::{
	input_focus::{FocusCause, InputFocus},
	prelude::*,
	ui::{FocusPolicy, RelativeCursorPosition},
};
use house_document::HouseEntry;

use crate::{
	actions::ActionSender,
	document::project::{Houses, OpenHouse, ProjectPath, ProjectRequest},
	input::{Action, ActionState, SCROLL_PIXELS_PER_LINE},
	ui::{
		fields::{FocusedField, wide_text_field},
		icons::{Icon, Icons},
		panel::{EditorUiSystems, accent_button, icon_button, step_button},
		theme,
	},
};

const CARD_WIDTH: f32 = 260.0;
const SECONDS_PER_MINUTE: u64 = 60;
const SECONDS_PER_HOUR: u64 = 60 * SECONDS_PER_MINUTE;
const SECONDS_PER_DAY: u64 = 24 * SECONDS_PER_HOUR;

/// Which full-window view is shown. The library is first, because there is no house to edit
/// until one is chosen.
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
	#[default]
	Library,
	Editor,
}

/// What the library screen shows, and what the user is in the middle of doing to it.
#[derive(Resource)]
struct LibraryView {
	houses: Vec<HouseEntry>,
	/// The house whose name is being edited.
	renaming: Option<String>,
	/// The house waiting for the second click that deletes it.
	confirming_delete: Option<String>,
	message: String,
	/// The folder must be read again. Set when the screen opens and after every change to it.
	reload: bool,
	/// The cards no longer match the fields above. Not set by `message`, so reporting a
	/// problem does not rebuild the cards and discard what is being typed.
	rebuild: bool,
}

impl Default for LibraryView {
	fn default() -> Self {
		Self {
			houses: Vec::new(),
			renaming: None,
			confirming_delete: None,
			message: String::new(),
			reload: true,
			rebuild: true,
		}
	}
}

impl LibraryView {
	fn reset(&mut self) {
		self.renaming = None;
		self.confirming_delete = None;
		self.message.clear();
		self.reload = true;
	}
}

#[derive(Component)]
struct LibraryScreen;

#[derive(Component)]
struct CardGrid;

#[derive(Component)]
struct BackButton;

#[derive(Component)]
struct LibraryMessage;

#[derive(Component, Clone, Copy)]
struct RenameField;

#[derive(Component, Clone)]
pub(super) enum LibraryButton {
	ShowLibrary,
	BackToEditor,
	NewHouse,
	Open(HouseEntry),
	Rename(String),
	AskDelete(String),
	ConfirmDelete(String),
	CancelDelete,
}

pub(super) fn register(app: &mut App) {
	app.init_resource::<Screen>()
		.init_resource::<LibraryView>()
		.add_systems(Startup, setup_library_screen)
		.add_systems(
			Update,
			(
				handle_library_buttons,
				open_library_on_shortcut,
				submit_rename,
				cancel_rename,
				scroll_library,
			)
				.in_set(EditorUiSystems::Settings),
		)
		.add_systems(
			PostUpdate,
			(
				update_library_visibility.run_if(resource_changed::<Screen>),
				reload_library,
				rebuild_cards,
				update_library_message,
				update_back_button,
			)
				.chain()
				.in_set(EditorUiSystems::Readouts),
		);
}

/// The button that returns to the library, for the top bar.
pub(super) fn library_button(icons: &Icons) -> impl Bundle {
	(
		icon_button(
			icons,
			Icon::Houses,
			Some("Houses"),
			theme::BUTTON,
			theme::TEXT,
		),
		LibraryButton::ShowLibrary,
	)
}

fn setup_library_screen(mut commands: Commands, icons: Res<Icons>) {
	commands.spawn((
		Name::new("library-screen"),
		LibraryScreen,
		GlobalZIndex(10),
		Node {
			position_type: PositionType::Absolute,
			width: Val::Percent(100.0),
			height: Val::Percent(100.0),
			flex_direction: FlexDirection::Column,
			padding: UiRect::all(Val::Px(32.0)),
			row_gap: Val::Px(16.0),
			..default()
		},
		BackgroundColor(theme::VIEWPORT),
		// Interaction makes the screen block viewport picking, including its blank space.
		Interaction::default(),
		FocusPolicy::Block,
		children![
			(
				Node {
					justify_content: JustifyContent::SpaceBetween,
					align_items: AlignItems::Center,
					..default()
				},
				children![
					(Text::new("Your houses"), TextFont::from_font_size(28.0)),
					(
						icon_button(
							&icons,
							Icon::Close,
							Some("Back to editor"),
							theme::BUTTON,
							theme::TEXT
						),
						LibraryButton::BackToEditor,
						BackButton
					),
				],
			),
			(Text::new(""), LibraryMessage),
			(
				Node {
					flex_wrap: FlexWrap::Wrap,
					align_content: AlignContent::FlexStart,
					row_gap: Val::Px(16.0),
					column_gap: Val::Px(16.0),
					overflow: Overflow::scroll_y(),
					..default()
				},
				ScrollPosition::default(),
				RelativeCursorPosition::default(),
				CardGrid,
			),
		],
	));
}

fn update_library_visibility(
	screen: Res<Screen>,
	mut library: Single<&mut Node, With<LibraryScreen>>,
) {
	library.display = match *screen {
		Screen::Library => Display::Flex,
		Screen::Editor => Display::None,
	};
}

fn handle_library_buttons(
	buttons: Query<(&Interaction, &LibraryButton), Changed<Interaction>>,
	houses: Res<Houses>,
	mut view: ResMut<LibraryView>,
	mut screen: ResMut<Screen>,
	project_path: Res<ProjectPath>,
	mut open_requests: MessageWriter<OpenHouse>,
	mut actions: ActionSender,
) {
	for (interaction, button) in &buttons {
		if *interaction != Interaction::Pressed {
			continue;
		}
		match button.clone() {
			LibraryButton::ShowLibrary => {
				view.reset();
				*screen = Screen::Library;
			}
			LibraryButton::BackToEditor => *screen = Screen::Editor,
			LibraryButton::NewHouse => {
				actions.send(ProjectRequest::New);
				// The unsaved-changes question, if any, is asked in the editor.
				*screen = Screen::Editor;
			}
			LibraryButton::Open(house) => {
				// Opening the open house again would only ask about its own unsaved changes.
				if project_path.0.as_deref() != Some(house.path.as_path()) {
					open_requests.write(OpenHouse(house.path));
				}
				*screen = Screen::Editor;
			}
			LibraryButton::Rename(name) => {
				view.renaming = Some(name);
				view.confirming_delete = None;
				view.rebuild = true;
			}
			LibraryButton::AskDelete(name) => {
				view.confirming_delete = Some(name);
				view.renaming = None;
				view.rebuild = true;
			}
			LibraryButton::CancelDelete => {
				view.confirming_delete = None;
				view.rebuild = true;
			}
			LibraryButton::ConfirmDelete(name) => {
				let Some(library) = &houses.0 else {
					continue;
				};
				match library.delete(&name) {
					Ok(()) => view.message.clear(),
					Err(error) => view.message = error.to_string(),
				}
				view.confirming_delete = None;
				view.reload = true;
			}
		}
	}
}

fn open_library_on_shortcut(
	actions: Res<ActionState>,
	mut view: ResMut<LibraryView>,
	mut screen: ResMut<Screen>,
) {
	if actions.just_pressed(Action::Open) && *screen != Screen::Library {
		view.reset();
		*screen = Screen::Library;
	}
}

fn submit_rename(
	actions: Res<ActionState>,
	focused: FocusedField<RenameField>,
	houses: Res<Houses>,
	mut view: ResMut<LibraryView>,
	mut project_path: ResMut<ProjectPath>,
) {
	if !actions.just_pressed(Action::FieldSubmit) {
		return;
	}
	let (Some((_, text)), Some(old_name), Some(library)) =
		(focused.get(), view.renaming.clone(), &houses.0)
	else {
		return;
	};
	let old_path = library.path_for(&old_name).ok();
	match library.rename(&old_name, &text) {
		Ok(renamed) => {
			// The open house keeps being saved to its file, which now has another name.
			if old_path.is_some() && project_path.0 == old_path {
				project_path.0 = Some(renamed.path);
			}
			view.message.clear();
			view.renaming = None;
			view.reload = true;
		}
		Err(error) => view.message = error.to_string(),
	}
}

fn cancel_rename(actions: Res<ActionState>, mut view: ResMut<LibraryView>) {
	if actions.just_pressed(Action::FieldCancel) && view.renaming.is_some() {
		view.renaming = None;
		view.message.clear();
		view.rebuild = true;
	}
}

fn scroll_library(
	actions: Res<ActionState>,
	mut grid: Single<(&RelativeCursorPosition, &ComputedNode, &mut ScrollPosition), With<CardGrid>>,
) {
	let (cursor, computed, position) = &mut *grid;
	if !cursor.cursor_over() {
		return;
	}
	let delta = actions.scroll_lines() * SCROLL_PIXELS_PER_LINE;
	let maximum = ((computed.content_size().y - computed.size().y)
		* computed.inverse_scale_factor())
	.max(0.0);
	position.y = (position.y - delta).clamp(0.0, maximum);
}

fn reload_library(houses: Res<Houses>, mut view: ResMut<LibraryView>) {
	if !view.reload {
		return;
	}
	view.reload = false;
	view.rebuild = true;
	let Some(library) = &houses.0 else {
		view.houses.clear();
		view.message = "There is nowhere to keep houses".to_string();
		return;
	};
	match library.list() {
		Ok(list) => view.houses = list,
		Err(error) => {
			view.houses.clear();
			view.message = error.to_string();
		}
	}
}

fn rebuild_cards(
	mut commands: Commands,
	mut view: ResMut<LibraryView>,
	project_path: Res<ProjectPath>,
	icons: Res<Icons>,
	grid: Single<Entity, With<CardGrid>>,
	focus: Option<ResMut<InputFocus>>,
) {
	if !view.rebuild {
		return;
	}
	view.rebuild = false;

	let current = project_path.0.as_deref();
	let mut rename_field = None;
	commands.entity(*grid).despawn_children();
	commands.entity(*grid).with_children(|grid| {
		grid.spawn((new_house_card(&icons), LibraryButton::NewHouse));
		for house in &view.houses {
			let is_current = current == Some(house.path.as_path());
			let card = grid.spawn(card_node()).id();
			let renaming = view.renaming.as_deref() == Some(house.name.as_str());
			if renaming {
				rename_field = Some(
					grid.commands()
						.spawn((wide_text_field(RenameField), ChildOf(card)))
						.id(),
				);
			}
			let confirming = view.confirming_delete.as_deref() == Some(house.name.as_str());
			grid.commands().entity(card).with_children(|card| {
				fill_card(card, &icons, house, is_current, renaming, confirming)
			});
			if !renaming {
				grid.commands()
					.entity(card)
					.insert((Button, LibraryButton::Open(house.clone())));
			}
		}
	});

	if let (Some(mut focus), Some(field)) = (focus, rename_field) {
		focus.set(field, FocusCause::Navigated);
	}
}

fn card_node() -> impl Bundle {
	(
		Node {
			width: Val::Px(CARD_WIDTH),
			min_height: Val::Px(120.0),
			flex_direction: FlexDirection::Column,
			justify_content: JustifyContent::SpaceBetween,
			padding: UiRect::all(Val::Px(14.0)),
			row_gap: Val::Px(8.0),
			border: UiRect::all(Val::Px(1.0)),
			border_radius: BorderRadius::all(theme::RADIUS),
			..default()
		},
		BackgroundColor(theme::PANEL),
		BorderColor::from(theme::PANEL_BORDER),
	)
}

fn new_house_card(icons: &Icons) -> impl Bundle {
	(
		Button,
		Node {
			width: Val::Px(CARD_WIDTH),
			min_height: Val::Px(120.0),
			flex_direction: FlexDirection::Column,
			row_gap: Val::Px(6.0),
			justify_content: JustifyContent::Center,
			align_items: AlignItems::Center,
			border_radius: BorderRadius::all(theme::RADIUS),
			..default()
		},
		BackgroundColor(theme::ACCENT),
		children![
			(
				Node {
					width: Val::Px(40.0),
					height: Val::Px(40.0),
					..default()
				},
				ImageNode::new(icons.get(Icon::Plus)).with_color(theme::TEXT_ON_ACCENT),
			),
			(
				Text::new("New house"),
				TextFont::from_font_size(22.0),
				TextColor(theme::TEXT_ON_ACCENT),
			),
		],
	)
}

fn fill_card(
	card: &mut ChildSpawnerCommands,
	icons: &Icons,
	house: &HouseEntry,
	is_current: bool,
	renaming: bool,
	confirming: bool,
) {
	if !renaming {
		card.spawn((
			Text::new(house.name.clone()),
			TextFont::from_font_size(22.0),
		));
	}
	let modified = match house.modified {
		Some(time) => format!("Modified {}", describe_age(time)),
		None => String::new(),
	};
	let detail = match (is_current, renaming) {
		(_, true) => "Enter renames, Esc cancels".to_string(),
		(true, _) => format!("Open now. {modified}"),
		_ => modified,
	};
	card.spawn((Text::new(detail), TextColor(theme::TEXT.with_alpha(0.7))));
	if renaming {
		return;
	}
	card.spawn(Node {
		column_gap: Val::Px(8.0),
		..default()
	})
	.with_children(|row| {
		if confirming {
			row.spawn((
				accent_button("Delete?"),
				LibraryButton::ConfirmDelete(house.name.clone()),
			));
			row.spawn((step_button("Cancel"), LibraryButton::CancelDelete));
		} else {
			row.spawn((
				step_button("Rename"),
				LibraryButton::Rename(house.name.clone()),
			));
			// The open house is the one being saved to, so it cannot be deleted from under it.
			if !is_current {
				row.spawn((
					icon_button(
						icons,
						Icon::Delete,
						Some("Delete"),
						theme::BUTTON,
						theme::TEXT,
					),
					LibraryButton::AskDelete(house.name.clone()),
				));
			}
		}
	});
}

/// "just now", "5 min ago", "3 h ago", "2 days ago".
fn describe_age(time: SystemTime) -> String {
	let seconds = SystemTime::now()
		.duration_since(time)
		.map_or(0, |age| age.as_secs());
	match seconds {
		0..SECONDS_PER_MINUTE => "just now".to_string(),
		SECONDS_PER_MINUTE..SECONDS_PER_HOUR => format!("{} min ago", seconds / SECONDS_PER_MINUTE),
		SECONDS_PER_HOUR..SECONDS_PER_DAY => format!("{} h ago", seconds / SECONDS_PER_HOUR),
		_ => format!("{} days ago", seconds / SECONDS_PER_DAY),
	}
}

/// Without an open house there is nothing in the editor to go back to.
fn update_back_button(
	project_path: Res<ProjectPath>,
	mut button: Single<&mut Node, With<BackButton>>,
) {
	let display = if project_path.0.is_some() {
		Display::Flex
	} else {
		Display::None
	};
	if button.display != display {
		button.display = display;
	}
}

fn update_library_message(
	view: Res<LibraryView>,
	mut message: Single<&mut Text, With<LibraryMessage>>,
) {
	// Writing identical text would still trigger a text layout every frame.
	if message.0 != view.message {
		message.0 = view.message.clone();
	}
}
