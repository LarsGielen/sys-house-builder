use super::*;
use crate::edit::{WallProposal, propose_wall};
use bevy::prelude::Vec2;

/// A fresh directory for one test, removed afterwards.
struct TestDirectory(PathBuf);

impl TestDirectory {
	fn new(name: &str) -> Self {
		let path = std::env::temp_dir().join(format!("house_editor_{name}_{}", std::process::id()));
		let _ = fs::remove_dir_all(&path);
		fs::create_dir_all(&path).unwrap();
		Self(path)
	}

	fn path(&self, file: &str) -> PathBuf {
		self.0.join(file)
	}
}

impl Drop for TestDirectory {
	fn drop(&mut self) {
		let _ = fs::remove_dir_all(&self.0);
	}
}

fn house() -> EditCandidate {
	propose_wall(
		&EditCandidate::default(),
		&WallProposal {
			start: Vec2::ZERO,
			end: Vec2::new(4.0, 0.0),
			dimensions: Default::default(),
		},
	)
	.unwrap()
}

#[test]
fn a_saved_project_opens_again() {
	let directory = TestDirectory::new("roundtrip");
	let path = directory.path("house.project");
	save_project(&path, &house()).unwrap();
	let reopened = open_project(&path).unwrap();
	assert_eq!(reopened.graph().walls().count(), 1);
	assert!(
		!directory.path("house.project.saving").exists(),
		"no temporary file is left behind"
	);
}

#[test]
fn saving_replaces_an_existing_file_completely() {
	let directory = TestDirectory::new("replace");
	let path = directory.path("house.project");
	fs::write(
		&path,
		"old contents that are much longer than the new project text? ".repeat(200),
	)
	.unwrap();
	save_project(&path, &EditCandidate::default()).unwrap();
	let text = fs::read_to_string(&path).unwrap();
	assert!(text.starts_with("house-editor-project 1"));
	assert!(!text.contains("old contents"));
}

#[test]
fn a_failed_save_reports_an_error_leaves_the_destination_alone_and_cleans_up() {
	let directory = TestDirectory::new("failed");
	// A directory cannot be replaced by a file, so the final rename fails after the
	// temporary file was written completely.
	let destination = directory.path("house.project");
	fs::create_dir(&destination).unwrap();
	fs::write(destination.join("keep.txt"), "kept").unwrap();

	let error = save_project(&destination, &house()).unwrap_err();

	assert!(matches!(error, ProjectError::Write { .. }));
	assert!(error.to_string().contains("house.project"));
	assert_eq!(
		fs::read_to_string(destination.join("keep.txt")).unwrap(),
		"kept"
	);
	assert!(!directory.path("house.project.saving").exists());
}

#[test]
fn saving_into_a_missing_directory_fails_visibly() {
	let directory = TestDirectory::new("missing");
	let error = save_project(&directory.path("nowhere/house.project"), &house()).unwrap_err();
	assert!(matches!(error, ProjectError::Write { .. }));
}

#[test]
fn opening_a_missing_or_invalid_file_is_an_error() {
	let directory = TestDirectory::new("open_errors");
	assert!(matches!(
		open_project(&directory.path("absent.project")),
		Err(ProjectError::Read { .. })
	));
	let broken = directory.path("broken.project");
	fs::write(&broken, "house-editor-project 9\n").unwrap();
	let error = open_project(&broken).err().unwrap();
	assert!(matches!(error, ProjectError::Import { .. }));
	assert!(error.to_string().contains("unsupported project version 9"));
}
