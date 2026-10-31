//! The bottom dock: the tools, and the catalog of what the chosen tool places.
//!
//! Door and Window are one tool in the editor (placing an opening) that differ by category,
//! so their buttons also choose the category. Only the standard door and window exist yet.
//! The other catalog entries and Furniture are placeholders that say they are not available.

use bevy::{ecs::system::SystemParam, prelude::*};
use house_document::OpeningCategory;

use crate::{
	actions::ActionSender,
	actions::settings::SettingsAction,
	document::edit::EditNotice,
	tools::active::{ActivateTool, ActiveTool, Tool},
	tools::opening::OpeningSettings,
	ui::{
		icons::{Icon, Icons, icon_image},
		panel::EditorUiSystems,
		theme,
	},
};

const TOOL_TILE_WIDTH: f32 = 88.0;
const CARD_WIDTH: f32 = 100.0;

/// What a dock button stands for. Several can share one editor [`Tool`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DockTool {
	Select,
	Wall,
	Curve,
	Door,
	Window,
	Furniture,
	Delete,
}

impl DockTool {
	const ALL: [DockTool; 7] = [
		Self::Select,
		Self::Wall,
		Self::Curve,
		Self::Door,
		Self::Window,
		Self::Furniture,
		Self::Delete,
	];

	fn label(self) -> &'static str {
		match self {
			Self::Select => "Select",
			Self::Wall => "Wall",
			Self::Curve => "Curve",
			Self::Door => "Door",
			Self::Window => "Window",
			Self::Furniture => "Furniture",
			Self::Delete => "Delete",
		}
	}

	fn icon(self) -> Icon {
		match self {
			Self::Select => Icon::Select,
			Self::Wall => Icon::Wall,
			Self::Curve => Icon::Curve,
			Self::Door => Icon::Door,
			Self::Window => Icon::Window,
			Self::Furniture => Icon::Furniture,
			Self::Delete => Icon::Delete,
		}
	}

	/// The tools that place something have a catalog of what they place.
	fn catalog(self) -> &'static [CatalogEntry] {
		match self {
			Self::Door => &DOORS,
			Self::Window => &WINDOWS,
			Self::Furniture => &FURNITURE,
			_ => &[],
		}
	}

	fn catalog_title(self) -> &'static str {
		match self {
			Self::Door => "Doors",
			Self::Window => "Windows",
			_ => "Furniture",
		}
	}
}

struct CatalogEntry {
	name: &'static str,
	icon: Icon,
	available: bool,
}

const fn entry(name: &'static str, icon: Icon, available: bool) -> CatalogEntry {
	CatalogEntry {
		name,
		icon,
		available,
	}
}

const DOORS: [CatalogEntry; 5] = [
	entry("Standard", Icon::DoorStandard, true),
	entry("Arched", Icon::DoorArched, false),
	entry("Double", Icon::DoorDouble, false),
	entry("Glass", Icon::DoorGlass, false),
	entry("Sliding", Icon::DoorSliding, false),
];

const WINDOWS: [CatalogEntry; 5] = [
	entry("Standard", Icon::WindowStandard, true),
	entry("Round", Icon::WindowRound, false),
	entry("Wide", Icon::WindowWide, false),
	entry("Tall", Icon::WindowTall, false),
	entry("Bay", Icon::WindowBay, false),
];

const FURNITURE: [CatalogEntry; 5] = [
	entry("Sofa", Icon::FurnitureSofa, false),
	entry("Table", Icon::FurnitureTable, false),
	entry("Bed", Icon::FurnitureBed, false),
	entry("Chair", Icon::FurnitureChair, false),
	entry("Wardrobe", Icon::FurnitureWardrobe, false),
];

/// Furniture has no tool yet, so choosing it only opens its catalog.
#[derive(Resource, Default, PartialEq, Eq)]
struct FurnitureBrowsing(bool);

#[derive(Component)]
pub(super) struct DockToolButton(DockTool);

#[derive(Component)]
struct CatalogGroup(DockTool);

#[derive(Component)]
struct CatalogCard {
	group: DockTool,
	entry: &'static CatalogEntry,
}

pub(super) fn register(app: &mut App) {
	app.init_resource::<FurnitureBrowsing>()
		.add_systems(
			Update,
			(press_tool_buttons, press_catalog_cards).in_set(EditorUiSystems::Settings),
		)
		.add_systems(PostUpdate, update_dock.in_set(EditorUiSystems::Readouts));
}

/// The tool buttons, then the catalog of the chosen tool.
pub(super) fn dock_controls(icons: &Icons) -> impl Bundle {
	(
		Node {
			align_items: AlignItems::Center,
			column_gap: Val::Px(20.0),
			flex_wrap: FlexWrap::Wrap,
			row_gap: Val::Px(8.0),
			..default()
		},
		children![
			(
				Node {
					column_gap: Val::Px(8.0),
					row_gap: Val::Px(8.0),
					flex_wrap: FlexWrap::Wrap,
					..default()
				},
				Children::spawn(SpawnIter(
					DockTool::ALL
						.into_iter()
						.map(|tool| tool_button(icons, tool))
						.collect::<Vec<_>>()
						.into_iter()
				)),
			),
			(
				Node {
					width: Val::Px(1.0),
					align_self: AlignSelf::Stretch,
					..default()
				},
				BackgroundColor(theme::PANEL_BORDER),
			),
			catalog_group(icons, DockTool::Door),
			catalog_group(icons, DockTool::Window),
			catalog_group(icons, DockTool::Furniture),
		],
	)
}

fn tool_button(icons: &Icons, tool: DockTool) -> impl Bundle {
	(
		Button,
		DockToolButton(tool),
		Node {
			width: Val::Px(TOOL_TILE_WIDTH),
			padding: UiRect::axes(Val::Px(4.0), Val::Px(8.0)),
			flex_direction: FlexDirection::Column,
			align_items: AlignItems::Center,
			row_gap: Val::Px(4.0),
			border_radius: BorderRadius::all(theme::RADIUS),
			..default()
		},
		BackgroundColor(theme::BUTTON),
		children![
			icon_image(icons, tool.icon(), 32.0),
			(Text::new(tool.label()), TextFont::from_font_size(15.0)),
		],
	)
}

fn catalog_group(icons: &Icons, tool: DockTool) -> impl Bundle {
	(
		CatalogGroup(tool),
		Node {
			display: Display::None,
			flex_direction: FlexDirection::Column,
			row_gap: Val::Px(6.0),
			..default()
		},
		children![
			Text::new(tool.catalog_title()),
			(
				Node {
					column_gap: Val::Px(8.0),
					flex_wrap: FlexWrap::Wrap,
					row_gap: Val::Px(8.0),
					..default()
				},
				Children::spawn(SpawnIter(
					tool.catalog()
						.iter()
						.map(|entry| catalog_card(icons, tool, entry))
						.collect::<Vec<_>>()
						.into_iter()
				)),
			),
		],
	)
}

fn catalog_card(icons: &Icons, group: DockTool, entry: &'static CatalogEntry) -> impl Bundle {
	(
		Button,
		CatalogCard { group, entry },
		Node {
			width: Val::Px(CARD_WIDTH),
			padding: UiRect::axes(Val::Px(4.0), Val::Px(8.0)),
			flex_direction: FlexDirection::Column,
			align_items: AlignItems::Center,
			row_gap: Val::Px(4.0),
			border: UiRect::all(Val::Px(2.0)),
			border_radius: BorderRadius::all(theme::RADIUS),
			..default()
		},
		BackgroundColor(theme::BUTTON),
		BorderColor::from(Color::NONE),
		children![
			icon_image(icons, entry.icon, 40.0),
			(Text::new(entry.name), TextFont::from_font_size(15.0)),
		],
	)
}

fn press_tool_buttons(
	buttons: Query<(&Interaction, &DockToolButton), Changed<Interaction>>,
	mut furniture: ResMut<FurnitureBrowsing>,
	mut actions: ActionSender,
) {
	for (interaction, button) in &buttons {
		if *interaction != Interaction::Pressed {
			continue;
		}
		furniture.0 = button.0 == DockTool::Furniture;
		match button.0 {
			DockTool::Select | DockTool::Furniture => actions.send(ActivateTool(Tool::Select)),
			DockTool::Wall => actions.send(ActivateTool(Tool::DrawWall)),
			DockTool::Curve => actions.send(ActivateTool(Tool::DrawArc)),
			DockTool::Delete => actions.send(ActivateTool(Tool::Delete)),
			DockTool::Door => {
				actions.send(SettingsAction::SetOpeningCategory(OpeningCategory::Door));
				actions.send(ActivateTool(Tool::PlaceOpening));
			}
			DockTool::Window => {
				actions.send(SettingsAction::SetOpeningCategory(OpeningCategory::Window));
				actions.send(ActivateTool(Tool::PlaceOpening));
			}
		}
	}
}

fn press_catalog_cards(
	cards: Query<(&Interaction, &CatalogCard), Changed<Interaction>>,
	mut notice: ResMut<EditNotice>,
) {
	for (interaction, card) in &cards {
		if *interaction == Interaction::Pressed && !card.entry.available {
			notice.message = Some(format!("{} is not available yet", card.entry.name));
		}
	}
}

/// The dock button that stands for what is going on now.
fn current_tool(tool: Tool, category: OpeningCategory, furniture: bool) -> DockTool {
	match tool {
		Tool::Select if furniture => DockTool::Furniture,
		Tool::Select => DockTool::Select,
		Tool::DrawWall => DockTool::Wall,
		Tool::DrawArc => DockTool::Curve,
		Tool::Delete => DockTool::Delete,
		Tool::PlaceOpening => match category {
			OpeningCategory::Door => DockTool::Door,
			OpeningCategory::Window => DockTool::Window,
		},
	}
}

fn update_dock(
	active_tool: Res<ActiveTool>,
	opening_settings: Res<OpeningSettings>,
	mut furniture: ResMut<FurnitureBrowsing>,
	mut buttons: Query<(&DockToolButton, &mut BackgroundColor, &Children), Without<CatalogCard>>,
	mut groups: Query<(&CatalogGroup, &mut Node)>,
	mut cards: Query<
		(
			&CatalogCard,
			&mut BackgroundColor,
			&mut BorderColor,
			&Children,
		),
		Without<DockToolButton>,
	>,
	mut tint: Tint,
) {
	// Another way of changing the tool, such as a shortcut, ends browsing furniture.
	if active_tool.0 != Tool::Select && furniture.0 {
		furniture.0 = false;
	}
	let current = current_tool(active_tool.0, opening_settings.category(), furniture.0);

	for (button, mut background, children) in &mut buttons {
		let active = button.0 == current;
		let (color, content) = if active {
			(theme::ACCENT, theme::TEXT_ON_ACCENT)
		} else {
			(theme::BUTTON, theme::TEXT)
		};
		background.set_if_neq(BackgroundColor(color));
		tint.apply(children, content);
	}

	for (group, mut node) in &mut groups {
		let display = if group.0 == current {
			Display::Flex
		} else {
			Display::None
		};
		if node.display != display {
			node.display = display;
		}
	}

	for (card, mut background, mut border, children) in &mut cards {
		// The standard door and window are what placing uses, so they show as chosen.
		let chosen = card.entry.available && card.group == current;
		let (color, content, outline) = match (card.entry.available, chosen) {
			(false, _) => (
				theme::BUTTON_DISABLED,
				theme::TEXT.with_alpha(0.45),
				Color::NONE,
			),
			(true, true) => (theme::BUTTON, theme::TEXT, theme::ACCENT),
			(true, false) => (theme::BUTTON, theme::TEXT, Color::NONE),
		};
		background.set_if_neq(BackgroundColor(color));
		border.set_if_neq(BorderColor::from(outline));
		tint.apply(children, content);
	}
}

/// Recolours the text and icons directly inside a button.
#[derive(SystemParam)]
struct Tint<'w, 's> {
	texts: Query<'w, 's, &'static mut TextColor>,
	icons: Query<'w, 's, &'static mut ImageNode>,
}

impl Tint<'_, '_> {
	fn apply(&mut self, children: &Children, color: Color) {
		for child in children {
			if let Ok(mut text) = self.texts.get_mut(*child) {
				text.set_if_neq(TextColor(color));
			}
			if let Ok(mut image) = self.icons.get_mut(*child)
				&& image.color != color
			{
				image.color = color;
			}
		}
	}
}

#[cfg(test)]
#[path = "../tests/ui/dock.rs"]
mod tests;
