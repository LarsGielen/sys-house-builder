//! Reading and writing project files, kept apart from the format so a path chosen in the UI
//! never reaches the parser and validator.
use std::{
	fmt, fs, io,
	io::Write,
	path::{Path, PathBuf},
};

use crate::{
	edit::EditCandidate,
	project_file::{ImportError, ProjectFile},
};

#[derive(Debug)]
pub enum ProjectError {
	Read { path: PathBuf, source: io::Error },
	Write { path: PathBuf, source: io::Error },
	Import { path: PathBuf, error: ImportError },
}

impl fmt::Display for ProjectError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Read { path, source } => write!(f, "Cannot read {}: {source}", path.display()),
			Self::Write { path, source } => {
				write!(f, "Cannot save {}: {source}", path.display())
			}
			Self::Import { path, error } => write!(f, "Cannot open {}: {error}", path.display()),
		}
	}
}

impl std::error::Error for ProjectError {}

/// Reads and validates a whole project, including its mesh. The current document is not
/// touched, so a failure anywhere leaves it intact.
pub fn open_project(path: &Path) -> Result<EditCandidate, ProjectError> {
	let text = fs::read_to_string(path).map_err(|source| ProjectError::Read {
		path: path.to_path_buf(),
		source,
	})?;
	ProjectFile::parse(&text)
		.and_then(|file| file.to_candidate())
		.map_err(|error| ProjectError::Import {
			path: path.to_path_buf(),
			error,
		})
}

/// Writes the whole document to a temporary file beside `path` and renames it into place,
/// so a failed save leaves the previous file as it was.
pub fn save_project(path: &Path, document: &EditCandidate) -> Result<(), ProjectError> {
	let text = ProjectFile::from_candidate(document).to_text();
	write_atomically(path, text.as_bytes()).map_err(|source| ProjectError::Write {
		path: path.to_path_buf(),
		source,
	})
}

fn write_atomically(path: &Path, bytes: &[u8]) -> io::Result<()> {
	let mut temporary_name = path
		.file_name()
		.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "the path has no file name"))?
		.to_os_string();
	temporary_name.push(".saving");
	// The same directory keeps the rename on one filesystem.
	let temporary = path.with_file_name(temporary_name);

	let result = (|| {
		let mut file = fs::File::create(&temporary)?;
		file.write_all(bytes)?;
		file.sync_all()?;
		drop(file);
		fs::rename(&temporary, path)
	})();
	if result.is_err() {
		// The original error is the one worth reporting; a leftover temporary is only litter.
		let _ = fs::remove_file(&temporary);
	}
	result
}

#[cfg(test)]
#[path = "tests/project_io.rs"]
mod tests;
