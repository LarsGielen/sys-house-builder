use super::*;
use crate::project::io::open_project;

/// A library in a fresh directory that is removed afterwards.
struct TestLibrary {
	library: HouseLibrary,
}

impl TestLibrary {
	fn new(test: &str) -> Self {
		let directory = std::env::temp_dir().join(format!(
			"house_document_library_{test}_{}",
			std::process::id()
		));
		let _ = fs::remove_dir_all(&directory);
		Self {
			library: HouseLibrary::new(directory),
		}
	}

	fn names(&self) -> Vec<String> {
		let mut names: Vec<_> = self
			.library
			.list()
			.unwrap()
			.into_iter()
			.map(|house| house.name)
			.collect();
		names.sort();
		names
	}
}

impl Drop for TestLibrary {
	fn drop(&mut self) {
		let _ = fs::remove_dir_all(self.library.directory());
	}
}

#[test]
fn a_missing_folder_is_an_empty_library() {
	let test = TestLibrary::new("missing");
	assert_eq!(test.library.list().unwrap(), Vec::new());
}

#[test]
fn new_houses_take_the_first_free_number_and_are_valid_projects() {
	let test = TestLibrary::new("create");
	let first = test.library.create_empty().unwrap();
	let second = test.library.create_empty().unwrap();
	assert_eq!(
		(first.name.as_str(), second.name.as_str()),
		("House 1", "House 2")
	);
	assert!(open_project(&first.path).is_ok());

	test.library.delete("House 1").unwrap();
	assert_eq!(test.library.create_empty().unwrap().name, "House 1");
	assert_eq!(test.names(), ["House 1", "House 2"]);
}

#[test]
fn listing_skips_other_files_and_puts_the_newest_first() {
	let test = TestLibrary::new("list");
	test.library.create_empty().unwrap();
	let newest = test.library.create_empty().unwrap();
	fs::write(test.library.directory().join("notes.txt"), "x").unwrap();
	fs::create_dir(test.library.directory().join("folder.project")).unwrap();
	let file = fs::File::options().write(true).open(&newest.path).unwrap();
	file.set_modified(SystemTime::now() + std::time::Duration::from_secs(60))
		.unwrap();

	let names: Vec<_> = test
		.library
		.list()
		.unwrap()
		.into_iter()
		.map(|house| house.name)
		.collect();
	assert_eq!(names, ["House 2", "House 1"]);
}

#[test]
fn renaming_moves_the_file() {
	let test = TestLibrary::new("rename");
	test.library.create_empty().unwrap();
	let renamed = test
		.library
		.rename("House 1", "  Courtyard house ")
		.unwrap();
	assert_eq!(renamed.name, "Courtyard house");
	assert!(open_project(&renamed.path).is_ok());
	assert_eq!(test.names(), ["Courtyard house"]);
}

#[test]
fn renaming_onto_another_house_is_refused_without_case_sensitivity() {
	let test = TestLibrary::new("collision");
	test.library.create_empty().unwrap();
	test.library.create_empty().unwrap();
	let result = test.library.rename("House 1", "house 2");
	assert!(matches!(result, Err(LibraryError::NameTaken(_))));
	assert_eq!(test.names(), ["House 1", "House 2"]);
}

#[test]
fn a_house_may_change_only_the_case_of_its_own_name() {
	let test = TestLibrary::new("case");
	test.library.create_empty().unwrap();
	test.library.rename("House 1", "HOUSE 1").unwrap();
	assert_eq!(test.names(), ["HOUSE 1"]);
}

#[test]
fn names_that_some_platform_refuses_are_rejected() {
	for (name, reason) in [
		("", InvalidName::Empty),
		("   ", InvalidName::Empty),
		("a/b", InvalidName::ForbiddenCharacter('/')),
		("a\\b", InvalidName::ForbiddenCharacter('\\')),
		("what?", InvalidName::ForbiddenCharacter('?')),
		("tab\there", InvalidName::ForbiddenCharacter('\t')),
		("..", InvalidName::TrailingDot),
		("house.", InvalidName::TrailingDot),
		("CON", InvalidName::Reserved),
		("nul.backup", InvalidName::Reserved),
		("Com1", InvalidName::Reserved),
		(&"x".repeat(MAX_NAME_CHARACTERS + 1), InvalidName::TooLong),
	] {
		assert_eq!(validate_name(name), Err(reason), "name {name:?}");
	}
	assert_eq!(validate_name("Courtyard house 2"), Ok("Courtyard house 2"));
	assert_eq!(validate_name("v1.2"), Ok("v1.2"));
}

#[test]
fn a_rejected_rename_leaves_the_library_unchanged() {
	let test = TestLibrary::new("rejected");
	test.library.create_empty().unwrap();
	assert!(matches!(
		test.library.rename("House 1", "a/b"),
		Err(LibraryError::InvalidName(_))
	));
	assert!(matches!(
		test.library.rename("Missing", "Other"),
		Err(LibraryError::UnknownHouse(_))
	));
	assert_eq!(test.names(), ["House 1"]);
}

#[test]
fn deleting_removes_only_the_named_house() {
	let test = TestLibrary::new("delete");
	test.library.create_empty().unwrap();
	test.library.create_empty().unwrap();
	test.library.delete("House 1").unwrap();
	assert_eq!(test.names(), ["House 2"]);
	assert!(matches!(
		test.library.delete("House 1"),
		Err(LibraryError::UnknownHouse(_))
	));
	assert!(matches!(
		test.library.delete("../House 2"),
		Err(LibraryError::InvalidName(_))
	));
	assert_eq!(test.names(), ["House 2"]);
}
