use bevy::prelude::*;

macro_rules! define_icons {
	($($name:ident => $file_stem:literal),* $(,)?) => {
		#[derive(Debug, Clone, Copy, PartialEq, Eq)]
		pub enum Icon {
			$($name),*
		}

		impl Icon {
			const ALL: &'static [Icon] = &[$(Icon::$name),*];

			const fn asset_path(self) -> &'static str {
				match self {
					$(Icon::$name => concat!("icons/", $file_stem, ".png")),*
				}
			}
		}
	};
}

define_icons!(
	House => "house",
	Undo => "undo",
	Redo => "redo",
	Save => "save",
	Open => "open",
);

#[derive(Resource)]
pub struct Icons(Vec<Handle<Image>>);

impl Icons {
	pub fn get(&self, icon: Icon) -> Handle<Image> {
		self.0[icon as usize].clone()
	}
}

pub struct IconsPlugin;

impl Plugin for IconsPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<Icons>();
	}
}

impl FromWorld for Icons {
	fn from_world(world: &mut World) -> Self {
		let asset_server = world.resource::<AssetServer>();
		Icons(
			Icon::ALL
				.iter()
				.map(|&icon| asset_server.load(icon.asset_path()))
				.collect(),
		)
	}
}
