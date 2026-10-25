use std::path::PathBuf;

use crate::HeadlessConfig;

/// Command-line options shared by every capture binary, so agents drive any project the same way:
/// `--out <png>`, `--size <W>x<H>`, `--scale <factor>`, `--frames <n>` and `--software`.
/// `HEADLESS_SOFTWARE=1` also selects software rendering.
#[derive(Clone, Debug, PartialEq)]
pub struct CaptureArgs {
	pub out: PathBuf,
	pub config: HeadlessConfig,
}

impl CaptureArgs {
	/// Parses the process arguments, printing the problem and exiting with status 2 on bad input.
	pub fn from_env() -> Self {
		let software_from_env = std::env::var("HEADLESS_SOFTWARE").is_ok_and(|value| value == "1");
		Self::parse(std::env::args().skip(1), software_from_env).unwrap_or_else(|message| {
			eprintln!("{message}");
			eprintln!(
				"usage: --out <png> [--size WxH] [--scale <factor>] [--frames <n>] [--software]"
			);
			std::process::exit(2);
		})
	}

	pub fn parse(
		arguments: impl IntoIterator<Item = String>,
		software_rendering: bool,
	) -> Result<Self, String> {
		let mut out = PathBuf::from("screenshot.png");
		let mut config = HeadlessConfig::default().with_software_rendering(software_rendering);
		let mut arguments = arguments.into_iter();
		while let Some(argument) = arguments.next() {
			let mut value = |name: &str| {
				arguments
					.next()
					.ok_or_else(|| format!("{name} needs a value"))
			};
			match argument.as_str() {
				"--out" => out = PathBuf::from(value("--out")?),
				"--size" => {
					let size = value("--size")?;
					let (width, height) = size
						.split_once('x')
						.and_then(|(width, height)| {
							Some((width.parse().ok()?, height.parse().ok()?))
						})
						.ok_or_else(|| format!("--size expects WxH, got `{size}`"))?;
					config = config.with_size(width, height);
				}
				"--scale" => {
					let scale = value("--scale")?;
					let scale = scale
						.parse::<f32>()
						.ok()
						.filter(|scale| *scale > 0.0)
						.ok_or_else(|| {
							format!("--scale expects a positive number, got `{scale}`")
						})?;
					config = config.with_scale_factor(scale);
				}
				"--frames" => {
					let frames = value("--frames")?;
					let frames = frames
						.parse()
						.map_err(|_| format!("--frames expects a number, got `{frames}`"))?;
					config = config.with_settle_frames(frames);
				}
				"--software" => config = config.with_software_rendering(true),
				unknown => return Err(format!("unknown argument `{unknown}`")),
			}
		}
		Ok(Self { out, config })
	}
}

#[cfg(test)]
#[path = "tests/args.rs"]
mod tests;
