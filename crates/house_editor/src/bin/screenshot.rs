//! Renders the editor to a PNG without a window, for agents and regression checks:
//! `cargo run -p house_editor --bin screenshot -- --out editor.png [--project house.json]
//! [--tool select|wall|curve|door|delete]`
//! plus the shared options of `bevy_headless::CaptureArgs`.

use bevy::prelude::*;
use bevy_headless::{CaptureArgs, HeadlessApp};
use house_document::open_project;
use house_editor::{ActiveTool, House, HouseEditorPlugin, Screen, Tool, setup_scene};

fn main() {
	let mut arguments: Vec<String> = std::env::args().skip(1).collect();
	let project_path = take_option(&mut arguments, "--project");
	let tool = take_option(&mut arguments, "--tool").map(|name| parse_tool(&name));
	let software_from_env = std::env::var("HEADLESS_SOFTWARE").is_ok_and(|value| value == "1");
	let args = CaptureArgs::parse(arguments, software_from_env).unwrap_or_else(|message| {
		eprintln!("{message}");
		std::process::exit(2);
	});

	let mut app = HeadlessApp::new(args.config, |app| {
		app.add_plugins(HouseEditorPlugin)
			.add_systems(Startup, setup_scene);
	});

	if let Some(path) = project_path {
		let document = open_project(path.as_ref()).unwrap_or_else(|error| {
			eprintln!("{error}");
			std::process::exit(1);
		});
		app.world_mut()
			.resource_mut::<House>()
			.replace_document(document);
		// A project is shown in the editor, not behind the house library.
		*app.world_mut().resource_mut::<Screen>() = Screen::Editor;
	}

	if let Some(tool) = tool {
		app.world_mut().resource_mut::<ActiveTool>().0 = tool;
		*app.world_mut().resource_mut::<Screen>() = Screen::Editor;
	}

	app.settle();
	app.screenshot(&args.out).expect("screenshot");
}

/// Removes `name <value>` from the arguments and returns the value.
fn take_option(arguments: &mut Vec<String>, name: &str) -> Option<String> {
	let position = arguments.iter().position(|argument| argument == name)?;
	if position + 1 >= arguments.len() {
		eprintln!("{name} needs a value");
		std::process::exit(2);
	}
	let value = arguments.remove(position + 1);
	arguments.remove(position);
	Some(value)
}

fn parse_tool(name: &str) -> Tool {
	match name {
		"select" => Tool::Select,
		"wall" => Tool::DrawWall,
		"curve" => Tool::DrawArc,
		"door" => Tool::PlaceOpening,
		"delete" => Tool::Delete,
		_ => {
			eprintln!("unknown tool {name}");
			std::process::exit(2);
		}
	}
}
