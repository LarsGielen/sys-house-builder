//! The editor's icons: white line art embedded in the binary and tinted when shown. They are
//! drawn by `icons/generate_icons.py`.

use std::collections::HashMap;

use bevy::{asset::embedded_asset, prelude::*};

use crate::ui::theme;

macro_rules! icons {
	($($name:ident => $file:literal),* $(,)?) => {
		#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
		pub enum Icon {
			$($name),*
		}

		impl Icon {
			const ALL: &[(Icon, &str)] = &[
				$((Icon::$name, concat!("embedded://house_editor/ui/", $file))),*
			];
		}

		fn embed_icons(app: &mut App) {
			$(embedded_asset!(app, $file);)*
		}
	};
}

icons! {
	Select => "icons/select.png",
	Wall => "icons/wall.png",
	Curve => "icons/curve.png",
	Door => "icons/door.png",
	Window => "icons/window.png",
	Furniture => "icons/furniture.png",
	Delete => "icons/delete.png",
	Undo => "icons/undo.png",
	Redo => "icons/redo.png",
	Save => "icons/save.png",
	Home => "icons/home.png",
	Grid => "icons/grid.png",
	Houses => "icons/houses.png",
	Plus => "icons/plus.png",
	Minus => "icons/minus.png",
	Rename => "icons/rename.png",
	Duplicate => "icons/duplicate.png",
	Close => "icons/close.png",
	DoorStandard => "icons/door_standard.png",
	DoorArched => "icons/door_arched.png",
	DoorDouble => "icons/door_double.png",
	DoorGlass => "icons/door_glass.png",
	DoorSliding => "icons/door_sliding.png",
	WindowStandard => "icons/window_standard.png",
	WindowRound => "icons/window_round.png",
	WindowWide => "icons/window_wide.png",
	WindowTall => "icons/window_tall.png",
	WindowBay => "icons/window_bay.png",
	FurnitureSofa => "icons/furniture_sofa.png",
	FurnitureTable => "icons/furniture_table.png",
	FurnitureBed => "icons/furniture_bed.png",
	FurnitureChair => "icons/furniture_chair.png",
	FurnitureWardrobe => "icons/furniture_wardrobe.png",
}

#[derive(Resource, Default)]
pub struct Icons(HashMap<Icon, Handle<Image>>);

impl Icons {
	/// An icon that is not loaded, such as in an app without assets, shows as nothing.
	pub fn get(&self, icon: Icon) -> Handle<Image> {
		self.0.get(&icon).cloned().unwrap_or_default()
	}
}

pub(super) fn register(app: &mut App) {
	if !app.world().contains_resource::<AssetServer>() {
		app.init_resource::<Icons>();
		return;
	}
	embed_icons(app);
	let server = app.world().resource::<AssetServer>().clone();
	let handles = Icon::ALL
		.iter()
		.map(|&(icon, path)| (icon, server.load(path)))
		.collect();
	app.insert_resource(Icons(handles));
}

/// An icon of `size` logical pixels, tinted with the normal text colour.
pub fn icon_image(icons: &Icons, icon: Icon, size: f32) -> impl Bundle {
	(
		Node {
			width: Val::Px(size),
			height: Val::Px(size),
			flex_shrink: 0.0,
			..default()
		},
		ImageNode::new(icons.get(icon)).with_color(theme::TEXT),
	)
}
