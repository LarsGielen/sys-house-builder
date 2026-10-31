//! The folder of saved houses: one project file per house, named after the house.
//!
//! The folder is the only source of truth, so there is no index to fall out of sync. Houses
//! are addressed by name rather than by path, so no caller can make the library touch a file
//! outside its folder. Names are validated against the rules of every supported platform, so
//! a library folder can be copied between them.

use std::{
	fmt, fs, io,
	path::{Path, PathBuf},
	time::SystemTime,
};

use crate::{
	candidate::EditCandidate,
	project::io::{ProjectError, save_project},
};

const PROJECT_EXTENSION: &str = "project";
const MAX_NAME_CHARACTERS: usize = 64;
const NEW_HOUSE_PREFIX: &str = "House";
const FORBIDDEN_CHARACTERS: &str = "<>:\"/\\|?*";
/// Device names Windows refuses as file names, with or without an extension.
const RESERVED_NAMES: [&str; 22] = [
	"con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
	"com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HouseEntry {
	pub name: String,
	pub path: PathBuf,
	pub modified: Option<SystemTime>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvalidName {
	Empty,
	TooLong,
	ForbiddenCharacter(char),
	/// Windows silently drops trailing dots, so the file would get another name.
	TrailingDot,
	Reserved,
}

#[derive(Debug)]
pub enum LibraryError {
	InvalidName(InvalidName),
	NameTaken(String),
	UnknownHouse(String),
	Io { path: PathBuf, source: io::Error },
	Save(ProjectError),
}

impl fmt::Display for InvalidName {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Empty => write!(f, "The name is empty"),
			Self::TooLong => write!(
				f,
				"The name is longer than {MAX_NAME_CHARACTERS} characters"
			),
			Self::ForbiddenCharacter(character) => {
				write!(f, "The name cannot contain {character:?}")
			}
			Self::TrailingDot => write!(f, "The name cannot end with a dot"),
			Self::Reserved => write!(f, "The name is reserved by the operating system"),
		}
	}
}

impl fmt::Display for LibraryError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::InvalidName(reason) => reason.fmt(f),
			Self::NameTaken(name) => write!(f, "There is already a house named {name}"),
			Self::UnknownHouse(name) => write!(f, "There is no house named {name}"),
			Self::Io { path, source } => write!(f, "Cannot access {}: {source}", path.display()),
			Self::Save(error) => error.fmt(f),
		}
	}
}

impl std::error::Error for InvalidName {}
impl std::error::Error for LibraryError {}

/// Checks `name` as it would be stored, and returns it trimmed.
pub fn validate_name(name: &str) -> Result<&str, InvalidName> {
	let name = name.trim();
	if name.is_empty() {
		return Err(InvalidName::Empty);
	}
	if name.chars().count() > MAX_NAME_CHARACTERS {
		return Err(InvalidName::TooLong);
	}
	if let Some(character) = name
		.chars()
		.find(|&character| character.is_control() || FORBIDDEN_CHARACTERS.contains(character))
	{
		return Err(InvalidName::ForbiddenCharacter(character));
	}
	if name.ends_with('.') {
		return Err(InvalidName::TrailingDot);
	}
	let device = name.split('.').next().unwrap_or(name).trim_end();
	if RESERVED_NAMES.contains(&device.to_lowercase().as_str()) {
		return Err(InvalidName::Reserved);
	}
	Ok(name)
}

/// Names are compared without case because Windows and macOS file systems do.
fn same_name(first: &str, second: &str) -> bool {
	first.to_lowercase() == second.to_lowercase()
}

pub struct HouseLibrary {
	directory: PathBuf,
}

impl HouseLibrary {
	/// The folder need not exist yet; it is created with the first house.
	pub fn new(directory: PathBuf) -> Self {
		Self { directory }
	}

	pub fn directory(&self) -> &Path {
		&self.directory
	}

	/// All houses, most recently modified first. Files that are not project files, and names
	/// that are not valid text, are not houses and are skipped.
	pub fn list(&self) -> Result<Vec<HouseEntry>, LibraryError> {
		let entries = match fs::read_dir(&self.directory) {
			Ok(entries) => entries,
			Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
			Err(source) => return Err(self.io_error(&self.directory, source)),
		};
		let mut houses = Vec::new();
		for entry in entries {
			let entry = entry.map_err(|source| self.io_error(&self.directory, source))?;
			let path = entry.path();
			let is_project = path
				.extension()
				.is_some_and(|extension| extension == PROJECT_EXTENSION);
			let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) else {
				continue;
			};
			let Ok(metadata) = entry.metadata() else {
				continue;
			};
			if !is_project || !metadata.is_file() {
				continue;
			}
			houses.push(HouseEntry {
				name: name.to_string(),
				modified: metadata.modified().ok(),
				path,
			});
		}
		houses.sort_by(|first, second| {
			second
				.modified
				.cmp(&first.modified)
				.then_with(|| first.name.cmp(&second.name))
		});
		Ok(houses)
	}

	/// The file a house with this name is stored in.
	pub fn path_for(&self, name: &str) -> Result<PathBuf, LibraryError> {
		let name = validate_name(name).map_err(LibraryError::InvalidName)?;
		Ok(self.directory.join(format!("{name}.{PROJECT_EXTENSION}")))
	}

	/// Saves an empty house under the first free name, "House 1", "House 2", and so on.
	pub fn create_empty(&self) -> Result<HouseEntry, LibraryError> {
		fs::create_dir_all(&self.directory)
			.map_err(|source| self.io_error(&self.directory, source))?;
		let existing = self.list()?;
		let name = (1..)
			.map(|number| format!("{NEW_HOUSE_PREFIX} {number}"))
			.find(|name| !existing.iter().any(|house| same_name(&house.name, name)))
			.expect("an unbounded range always yields a free name");
		let path = self.path_for(&name)?;
		save_project(&path, &EditCandidate::default()).map_err(LibraryError::Save)?;
		Ok(self.entry_for(name, path))
	}

	/// Renames a house. Only a change of case is allowed to match the house's own old name.
	pub fn rename(&self, name: &str, new_name: &str) -> Result<HouseEntry, LibraryError> {
		let old_path = self.existing_path(name)?;
		let new_name = validate_name(new_name).map_err(LibraryError::InvalidName)?;
		let taken = self
			.list()?
			.iter()
			.any(|house| same_name(&house.name, new_name) && !same_name(&house.name, name));
		if taken {
			return Err(LibraryError::NameTaken(new_name.to_string()));
		}
		let new_path = self.path_for(new_name)?;
		fs::rename(&old_path, &new_path).map_err(|source| self.io_error(&old_path, source))?;
		Ok(self.entry_for(new_name.to_string(), new_path))
	}

	pub fn delete(&self, name: &str) -> Result<(), LibraryError> {
		let path = self.existing_path(name)?;
		fs::remove_file(&path).map_err(|source| self.io_error(&path, source))
	}

	fn existing_path(&self, name: &str) -> Result<PathBuf, LibraryError> {
		let path = self.path_for(name)?;
		if path.is_file() {
			Ok(path)
		} else {
			Err(LibraryError::UnknownHouse(name.to_string()))
		}
	}

	fn entry_for(&self, name: String, path: PathBuf) -> HouseEntry {
		let modified = fs::metadata(&path)
			.and_then(|metadata| metadata.modified())
			.ok();
		HouseEntry {
			name,
			path,
			modified,
		}
	}

	fn io_error(&self, path: &Path, source: io::Error) -> LibraryError {
		LibraryError::Io {
			path: path.to_path_buf(),
			source,
		}
	}
}

#[cfg(test)]
#[path = "tests/library.rs"]
mod tests;
