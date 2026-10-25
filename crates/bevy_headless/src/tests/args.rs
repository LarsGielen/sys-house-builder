use super::*;

fn parse(arguments: &[&str]) -> Result<CaptureArgs, String> {
	CaptureArgs::parse(arguments.iter().map(|argument| argument.to_string()), false)
}

#[test]
fn defaults_apply_without_arguments() {
	let args = parse(&[]).unwrap();

	assert_eq!(args.out, PathBuf::from("screenshot.png"));
	assert_eq!(args.config, HeadlessConfig::default());
}

#[test]
fn every_option_is_read() {
	let args = parse(&[
		"--out",
		"a.png",
		"--size",
		"640x480",
		"--scale",
		"2",
		"--frames",
		"9",
		"--software",
	])
	.unwrap();

	assert_eq!(args.out, PathBuf::from("a.png"));
	assert_eq!(
		args.config,
		HeadlessConfig::default()
			.with_size(640, 480)
			.with_scale_factor(2.0)
			.with_settle_frames(9)
			.with_software_rendering(true)
	);
}

#[test]
fn environment_default_selects_software_rendering() {
	let args = CaptureArgs::parse([], true).unwrap();

	assert!(args.config.software_rendering);
}

#[test]
fn bad_input_is_reported() {
	assert!(parse(&["--size", "640"]).is_err());
	assert!(parse(&["--scale", "0"]).is_err());
	assert!(parse(&["--frames", "many"]).is_err());
	assert!(parse(&["--out"]).is_err());
	assert!(parse(&["--nope"]).is_err());
}
