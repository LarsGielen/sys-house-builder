use std::{
	fmt,
	path::{Path, PathBuf},
	sync::{Arc, Mutex},
};

use bevy::{
	app::{PluginGroupBuilder, PluginsState, SubApps},
	asset::RenderAssetUsages,
	prelude::*,
	render::{
		RenderPlugin,
		pipelined_rendering::PipelinedRenderingPlugin,
		render_resource::{Extent3d, PollType, TextureDimension, TextureFormat, TextureUsages},
		renderer::RenderDevice,
		settings::{RenderCreation, WgpuSettings},
		view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk},
	},
	tasks::tick_global_task_pools_on_main_thread,
	window::{ExitCondition, WindowResolution},
	winit::WinitPlugin,
};

use crate::{
	HeadlessConfig,
	target::{HeadlessTarget, HeadlessTargetPlugin},
};

/// Frames a capture may take before it counts as lost. GPU readback arrives a few frames after
/// the request.
const CAPTURE_TIMEOUT_FRAMES: u32 = 30;

/// A Bevy app with no OS window that the caller steps frame by frame and captures to images.
///
/// Cameras that render to the primary window are redirected to an off-screen image, so the app
/// is built exactly as it would be for a window. Stepping is synchronous: when [`step`](Self::step)
/// returns, the frame has been rendered.
pub struct HeadlessApp {
	sub_apps: SubApps,
	config: HeadlessConfig,
	target: Handle<Image>,
}

#[derive(Debug)]
pub enum HeadlessError {
	/// The awaited condition or capture did not happen within the given number of frames.
	Timeout { frames: u32 },
	/// A capture arrived but the file at the path was not written.
	SaveFailed(PathBuf),
}

impl fmt::Display for HeadlessError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Timeout { frames } => write!(formatter, "timed out after {frames} frames"),
			Self::SaveFailed(path) => write!(formatter, "could not save {}", path.display()),
		}
	}
}

impl std::error::Error for HeadlessError {}

impl HeadlessApp {
	/// Builds the app with Bevy's default plugins, minus the window event loop. `build` adds the
	/// application's own plugins and scene to the `App` it receives. It must not add
	/// `DefaultPlugins` itself; use [`with_default_plugins`](Self::with_default_plugins) to adjust
	/// them.
	///
	/// Panics if no graphics adapter is available.
	pub fn new(config: HeadlessConfig, build: impl FnOnce(&mut App)) -> Self {
		Self::with_default_plugins(config, |plugins| plugins, build)
	}

	/// Like [`new`](Self::new), with `customize` applied to the default plugins, for apps that
	/// configure them (asset paths, image sampling) in their normal `main`.
	pub fn with_default_plugins(
		config: HeadlessConfig,
		customize: impl FnOnce(PluginGroupBuilder) -> PluginGroupBuilder,
		build: impl FnOnce(&mut App),
	) -> Self {
		let mut app = App::new();
		app.add_plugins(customize(headless_default_plugins(&config)));

		let target = app
			.world_mut()
			.resource_mut::<Assets<Image>>()
			.add(target_image(&config));
		app.insert_resource(HeadlessTarget {
			image: target.clone(),
			scale_factor: config.scale_factor,
		})
		.add_plugins(HeadlessTargetPlugin);
		build(&mut app);

		// There is no runner to wait for the renderer to initialize or to finish plugin setup.
		while app.plugins_state() == PluginsState::Adding {
			tick_global_task_pools_on_main_thread();
		}
		app.finish();
		app.cleanup();

		Self {
			sub_apps: std::mem::take(app.sub_apps_mut()),
			config,
			target,
		}
	}

	pub fn config(&self) -> &HeadlessConfig {
		&self.config
	}

	pub fn world(&self) -> &World {
		self.sub_apps.main.world()
	}

	pub fn world_mut(&mut self) -> &mut World {
		self.sub_apps.main.world_mut()
	}

	/// Runs one frame and waits for the GPU to finish it.
	pub fn step(&mut self) {
		self.sub_apps.update();
		self.world()
			.resource::<RenderDevice>()
			.wgpu_device()
			.poll(PollType::Wait {
				submission_index: None,
				timeout: None,
			})
			.expect("waiting for the GPU to finish the frame");
	}

	pub fn step_frames(&mut self, frames: u32) {
		for _ in 0..frames {
			self.step();
		}
	}

	/// Runs the configured number of frames so startup, asset loading and layout can finish.
	pub fn settle(&mut self) {
		self.step_frames(self.config.settle_frames);
	}

	/// Steps until `condition` holds, for scenes that load assets or build meshes over several
	/// frames. Returns the frames it took.
	pub fn run_until(
		&mut self,
		mut condition: impl FnMut(&World) -> bool,
		max_frames: u32,
	) -> Result<u32, HeadlessError> {
		for frame in 0..max_frames {
			if condition(self.world()) {
				return Ok(frame);
			}
			self.step();
		}
		Err(HeadlessError::Timeout { frames: max_frames })
	}

	/// Captures the next frame as an image in memory.
	pub fn capture(&mut self) -> Result<Image, HeadlessError> {
		self.capture_with_save(None)
	}

	/// Captures the next frame and writes it to `path`; the extension picks the format.
	pub fn screenshot(&mut self, path: impl AsRef<Path>) -> Result<(), HeadlessError> {
		let path = path.as_ref();
		// A leftover file from an earlier run would otherwise look like a successful save.
		let _ = std::fs::remove_file(path);
		self.capture_with_save(Some(path.to_path_buf()))?;
		if path.exists() {
			Ok(())
		} else {
			Err(HeadlessError::SaveFailed(path.to_path_buf()))
		}
	}

	fn capture_with_save(&mut self, path: Option<PathBuf>) -> Result<Image, HeadlessError> {
		let captured = Arc::new(Mutex::new(None));
		let captured_in_observer = Arc::clone(&captured);
		let target = self.target.clone();

		let mut screenshot = self.world_mut().spawn(Screenshot::image(target));
		if let Some(path) = path {
			screenshot.observe(save_to_disk(path));
		}
		screenshot.observe(move |event: On<ScreenshotCaptured>| {
			*captured_in_observer.lock().expect("capture slot poisoned") =
				Some(event.image.clone());
		});

		for _ in 0..CAPTURE_TIMEOUT_FRAMES {
			self.step();
			if let Some(image) = captured.lock().expect("capture slot poisoned").take() {
				return Ok(image);
			}
		}
		Err(HeadlessError::Timeout {
			frames: CAPTURE_TIMEOUT_FRAMES,
		})
	}
}

fn headless_default_plugins(config: &HeadlessConfig) -> PluginGroupBuilder {
	let render_creation = if config.software_rendering {
		RenderCreation::Automatic(Box::new(WgpuSettings {
			force_fallback_adapter: true,
			..default()
		}))
	} else {
		RenderCreation::default()
	};

	DefaultPlugins
		.build()
		// The window still exists as an entity, so systems that read it keep working.
		.set(WindowPlugin {
			primary_window: Some(Window {
				resolution: WindowResolution::new(config.width, config.height)
					.with_scale_factor_override(config.scale_factor),
				..default()
			}),
			exit_condition: ExitCondition::DontExit,
			close_when_requested: false,
			..default()
		})
		.set(RenderPlugin {
			render_creation,
			// A capture must show every material, not the frames before shaders finish compiling.
			synchronous_pipeline_compilation: true,
			..default()
		})
		// Winit needs a display server; stepping by hand replaces its event loop.
		.disable::<WinitPlugin>()
		// Pipelining overlaps frames, which would make a capture lag the world it was asked in.
		.disable::<PipelinedRenderingPlugin>()
}

fn target_image(config: &HeadlessConfig) -> Image {
	let (width, height) = config.physical_size();
	let mut image = Image::new_uninit(
		Extent3d {
			width,
			height,
			depth_or_array_layers: 1,
		},
		TextureDimension::D2,
		TextureFormat::Rgba8UnormSrgb,
		RenderAssetUsages::RENDER_WORLD,
	);
	image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC;
	image
}

#[cfg(test)]
#[path = "tests/app.rs"]
mod tests;
