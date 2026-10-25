/// How a [`HeadlessApp`](crate::HeadlessApp) renders: the size of the virtual window and which
/// adapter draws it.
#[derive(Clone, Debug, PartialEq)]
pub struct HeadlessConfig {
	/// Logical width of the virtual window; the image is `width * scale_factor` pixels wide.
	pub width: u32,
	/// Logical height of the virtual window.
	pub height: u32,
	/// Window scale factor, which Bevy UI uses to size text and nodes.
	pub scale_factor: f32,
	/// Render with the software adapter (llvmpipe on Linux) instead of the GPU. Slower, but the
	/// output is identical across machines.
	pub software_rendering: bool,
	/// Frames [`HeadlessApp::settle`](crate::HeadlessApp::settle) runs so startup systems, asset
	/// loading and layout reach a steady state before a capture.
	pub settle_frames: u32,
}

impl Default for HeadlessConfig {
	fn default() -> Self {
		Self {
			width: 1280,
			height: 720,
			scale_factor: 1.0,
			software_rendering: false,
			settle_frames: 5,
		}
	}
}

impl HeadlessConfig {
	pub fn with_size(mut self, width: u32, height: u32) -> Self {
		self.width = width;
		self.height = height;
		self
	}

	pub fn with_scale_factor(mut self, scale_factor: f32) -> Self {
		self.scale_factor = scale_factor;
		self
	}

	pub fn with_software_rendering(mut self, software_rendering: bool) -> Self {
		self.software_rendering = software_rendering;
		self
	}

	pub fn with_settle_frames(mut self, settle_frames: u32) -> Self {
		self.settle_frames = settle_frames;
		self
	}

	/// Pixel size of the rendered image.
	pub(crate) fn physical_size(&self) -> (u32, u32) {
		let scale = |logical: u32| ((logical as f32 * self.scale_factor).round() as u32).max(1);
		(scale(self.width), scale(self.height))
	}
}

#[cfg(test)]
#[path = "tests/config.rs"]
mod tests;
