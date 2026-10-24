//! The project file format: a versioned, line-based text description of the editable
//! document. It knows nothing about files on disk or Bevy; `project_io` moves the text.
//!
//! ```text
//! house-editor-project 1
//! # Units are metres. x and y are floor-plan coordinates, z is elevation above the floor.
//! # A positive sweep is a counterclockwise arc, in radians. Tangency is not stored.
//! wall w1: from 0 0 to 4 0 height 2.5 thickness 0.2
//! wall w2: from 4 0 to 4 3 sweep 1.5707964 height 2.5 thickness 0.2
//! opening o1: wall w1 center 2 width 0.9 bottom 0 height 2.1 category door
//! ```
//!
//! IDs (`w1`, `o1`) are local to the file. An opening names its wall by that ID and measures
//! `center` along it from the wall's `from` end, so it stays attached however the graph
//! numbers its own walls when the file is read back.
use std::fmt;

use bevy::prelude::Vec2;
use wall_graph::{
	OpeningSpec, Wall, WallCurve, WallDimensions, WallError, WallGraph, arc_midpoint,
};
use wall_mesh::MeshError;

use crate::{
	edit::{EditCandidate, EditError},
	opening_category::{OpeningCategories, OpeningCategory},
};

pub const FORMAT_VERSION: u32 = 1;
const HEADER: &str = "house-editor-project";
/// Positions closer than this are the same place, matching the graph's own tolerance.
const POSITION_TOLERANCE_METRES: f32 = 1e-3;

#[derive(Debug, Clone, PartialEq)]
pub struct FileWall {
	pub id: String,
	pub start: Vec2,
	pub end: Vec2,
	/// `None` for a straight wall.
	pub sweep: Option<f32>,
	pub dimensions: WallDimensions,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileOpening {
	pub id: String,
	pub wall: String,
	pub spec: OpeningSpec,
	pub category: OpeningCategory,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProjectFile {
	pub walls: Vec<FileWall>,
	pub openings: Vec<FileOpening>,
}

/// Why a project file could not be turned into a document. Each names where to look.
#[derive(Debug, Clone, PartialEq)]
pub enum ImportError {
	Syntax {
		line: usize,
		message: String,
	},
	MissingHeader,
	UnsupportedVersion {
		found: String,
	},
	DuplicateId {
		id: String,
		line: usize,
	},
	UnknownWall {
		opening: String,
		wall: String,
	},
	Wall {
		id: String,
		error: WallError,
	},
	Opening {
		id: String,
		error: WallError,
	},
	/// The wall an opening names is not present as one wall in the rebuilt graph.
	HostNotFound {
		opening: String,
		wall: String,
	},
	Mesh(MeshError),
}

impl fmt::Display for ImportError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Syntax { line, message } => write!(f, "line {line}: {message}"),
			Self::MissingHeader => write!(f, "not a project file: expected '{HEADER} <version>'"),
			Self::UnsupportedVersion { found } => write!(
				f,
				"unsupported project version {found}; this editor reads version {FORMAT_VERSION}"
			),
			Self::DuplicateId { id, line } => write!(f, "line {line}: '{id}' is already used"),
			Self::UnknownWall { opening, wall } => {
				write!(f, "opening {opening}: there is no wall '{wall}'")
			}
			Self::Wall { id, error } => write!(f, "wall {id}: {error}"),
			Self::Opening { id, error } => write!(f, "opening {id}: {error}"),
			Self::HostNotFound { opening, wall } => write!(
				f,
				"opening {opening}: wall '{wall}' does not exist as one wall once the walls are joined"
			),
			Self::Mesh(error) => write!(f, "the house cannot be built: {error}"),
		}
	}
}

impl std::error::Error for ImportError {}

fn syntax(line: usize, message: impl Into<String>) -> ImportError {
	ImportError::Syntax {
		line,
		message: message.into(),
	}
}

impl ProjectFile {
	/// Describes the document's current walls and openings, in a stable order.
	pub fn from_candidate(candidate: &EditCandidate) -> Self {
		let graph = candidate.graph();
		let mut walls: Vec<(Wall, Vec2, Vec2, Option<f32>)> = graph
			.walls()
			.filter_map(|wall| {
				let start = graph.node_position(wall.origin())?;
				let end = graph.node_position(wall.destination())?;
				let sweep = match graph.wall_curve(wall)? {
					WallCurve::Straight { .. } => None,
					WallCurve::CircularArc { sweep, .. } => Some(sweep as f32),
				};
				Some((wall, start, end, sweep))
			})
			.collect();
		walls.sort_by(|first, second| {
			let key = |entry: &(Wall, Vec2, Vec2, Option<f32>)| {
				[
					entry.1.x,
					entry.1.y,
					entry.2.x,
					entry.2.y,
					entry.3.unwrap_or(0.0),
				]
			};
			key(first)
				.iter()
				.zip(key(second))
				.map(|(a, b)| a.total_cmp(&b))
				.find(|order| order.is_ne())
				.unwrap_or(std::cmp::Ordering::Equal)
		});

		let file_walls: Vec<FileWall> = walls
			.iter()
			.enumerate()
			.map(|(index, &(wall, start, end, sweep))| FileWall {
				id: format!("w{}", index + 1),
				start,
				end,
				sweep,
				dimensions: graph
					.wall_dimensions(wall)
					.expect("a listed wall is current"),
			})
			.collect();

		let mut openings: Vec<(usize, FileOpening)> = graph
			.openings()
			.filter_map(|(id, opening)| {
				let index = walls.iter().position(|entry| entry.0 == opening.wall)?;
				Some((
					index,
					FileOpening {
						id: String::new(),
						wall: file_walls[index].id.clone(),
						spec: opening.spec,
						category: candidate.categories().get(id).unwrap_or_default(),
					},
				))
			})
			.collect();
		openings.sort_by(|(first_wall, first), (second_wall, second)| {
			first_wall.cmp(second_wall).then(
				first
					.spec
					.center_distance
					.total_cmp(&second.spec.center_distance),
			)
		});
		let openings = openings
			.into_iter()
			.enumerate()
			.map(|(index, (_, mut opening))| {
				opening.id = format!("o{}", index + 1);
				opening
			})
			.collect();

		Self {
			walls: file_walls,
			openings,
		}
	}

	pub fn to_text(&self) -> String {
		let mut text = format!(
			"{HEADER} {FORMAT_VERSION}\n\
			 # Units are metres. x and y are floor-plan coordinates; z is elevation above the floor.\n\
			 # A positive sweep is a counterclockwise arc in radians. Tangency is not stored.\n"
		);
		for wall in &self.walls {
			text += &format!(
				"wall {}: from {} {} to {} {}",
				wall.id, wall.start.x, wall.start.y, wall.end.x, wall.end.y
			);
			if let Some(sweep) = wall.sweep {
				text += &format!(" sweep {sweep}");
			}
			text += &format!(
				" height {} thickness {}\n",
				wall.dimensions.height, wall.dimensions.thickness
			);
		}
		for opening in &self.openings {
			let category = match opening.category {
				OpeningCategory::Door => "door",
				OpeningCategory::Window => "window",
			};
			text += &format!(
				"opening {}: wall {} center {} width {} bottom {} height {} category {category}\n",
				opening.id,
				opening.wall,
				opening.spec.center_distance,
				opening.spec.width,
				opening.spec.bottom,
				opening.spec.height
			);
		}
		text
	}

	/// Reads the whole text before anything is built from it.
	pub fn parse(text: &str) -> Result<Self, ImportError> {
		let mut lines = text
			.lines()
			.enumerate()
			.map(|(index, line)| (index + 1, line.trim()))
			.filter(|(_, line)| !line.is_empty() && !line.starts_with('#'));

		let (header_line, header) = lines.next().ok_or(ImportError::MissingHeader)?;
		let mut header_words = header.split_whitespace();
		if header_words.next() != Some(HEADER) {
			return Err(ImportError::MissingHeader);
		}
		let version = header_words
			.next()
			.ok_or_else(|| syntax(header_line, "the header needs a version"))?;
		if version != FORMAT_VERSION.to_string() {
			return Err(ImportError::UnsupportedVersion {
				found: version.to_string(),
			});
		}

		let mut file = Self::default();
		for (line, content) in lines {
			let mut words = Words {
				words: content.split_whitespace().peekable(),
				line,
			};
			match words.next_word("a wall or opening")? {
				"wall" => {
					let wall = words.wall()?;
					if file.walls.iter().any(|existing| existing.id == wall.id) {
						return Err(ImportError::DuplicateId { id: wall.id, line });
					}
					file.walls.push(wall);
				}
				"opening" => {
					let opening = words.opening()?;
					if file
						.openings
						.iter()
						.any(|existing| existing.id == opening.id)
					{
						return Err(ImportError::DuplicateId {
							id: opening.id,
							line,
						});
					}
					file.openings.push(opening);
				}
				other => {
					return Err(syntax(
						line,
						format!("expected 'wall' or 'opening', found '{other}'"),
					));
				}
			}
		}
		Ok(file)
	}

	/// Builds a fresh document through the graph's own operations, so every rule applies,
	/// then generates its mesh. Nothing is returned unless all of it is valid.
	pub fn to_candidate(&self) -> Result<EditCandidate, ImportError> {
		let mut graph = WallGraph::new();
		for wall in &self.walls {
			let result = match wall.sweep {
				None => graph.add_wall_with_dimensions(wall.start, wall.end, wall.dimensions),
				Some(sweep) => {
					graph.add_arc_with_dimensions(wall.start, wall.end, sweep, wall.dimensions)
				}
			};
			result.map_err(|error| ImportError::Wall {
				id: wall.id.clone(),
				error,
			})?;
		}

		let mut categories = OpeningCategories::default();
		for opening in &self.openings {
			let host = self
				.walls
				.iter()
				.find(|wall| wall.id == opening.wall)
				.ok_or_else(|| ImportError::UnknownWall {
					opening: opening.id.clone(),
					wall: opening.wall.clone(),
				})?;
			// Building the graph may have split or merged walls, so the host is found by its
			// geometry rather than by any handle.
			let (wall, reversed) =
				find_wall(&graph, host).ok_or_else(|| ImportError::HostNotFound {
					opening: opening.id.clone(),
					wall: opening.wall.clone(),
				})?;
			let mut spec = opening.spec;
			if reversed {
				let length = graph.wall_length(wall).unwrap_or_default();
				spec.center_distance = length - spec.center_distance;
			}
			let id = graph
				.add_opening(wall, spec)
				.map_err(|error| ImportError::Opening {
					id: opening.id.clone(),
					error,
				})?;
			categories.insert(id, opening.category);
		}

		EditCandidate::build(graph, categories).map_err(|error| match error {
			EditError::Mesh(error) => ImportError::Mesh(error),
			EditError::Graph(error) => ImportError::Wall {
				id: "(whole house)".to_string(),
				error,
			},
			EditError::Arc(_) => unreachable!("building a candidate solves no arc"),
		})
	}
}

/// The wall in `graph` that has the same ends and curve as `wanted`, and whether it runs the
/// other way.
fn find_wall(graph: &WallGraph, wanted: &FileWall) -> Option<(Wall, bool)> {
	let near = |first: Vec2, second: Vec2| first.distance(second) <= POSITION_TOLERANCE_METRES;
	let middle = arc_midpoint(wanted.start, wanted.end, wanted.sweep.unwrap_or(0.0));
	graph.walls().find_map(|wall| {
		let start = graph.node_position(wall.origin())?;
		let end = graph.node_position(wall.destination())?;
		let reversed = if near(start, wanted.start) && near(end, wanted.end) {
			false
		} else if near(start, wanted.end) && near(end, wanted.start) {
			true
		} else {
			return None;
		};
		let on_curve = graph
			.wall_position(wall, 0.5)
			.is_some_and(|point| near(point, middle));
		on_curve.then_some((wall, reversed))
	})
}

struct Words<'a> {
	words: std::iter::Peekable<std::str::SplitWhitespace<'a>>,
	line: usize,
}

impl<'a> Words<'a> {
	fn next_word(&mut self, expected: &str) -> Result<&'a str, ImportError> {
		self.words.next().ok_or_else(|| {
			syntax(
				self.line,
				format!("expected {expected}, found the end of the line"),
			)
		})
	}

	fn keyword(&mut self, keyword: &str) -> Result<(), ImportError> {
		match self.next_word(&format!("'{keyword}'"))? {
			found if found == keyword => Ok(()),
			found => Err(syntax(
				self.line,
				format!("expected '{keyword}', found '{found}'"),
			)),
		}
	}

	fn number(&mut self, what: &str) -> Result<f64, ImportError> {
		let word = self.next_word(what)?;
		match word.parse::<f64>() {
			Ok(value) if value.is_finite() => Ok(value),
			_ => Err(syntax(
				self.line,
				format!("{what} '{word}' is not a finite number"),
			)),
		}
	}

	fn position(&mut self, what: &str) -> Result<Vec2, ImportError> {
		Ok(Vec2::new(
			self.number(&format!("{what} x"))? as f32,
			self.number(&format!("{what} y"))? as f32,
		))
	}

	/// A declaration like `wall w1:`, giving `w1`.
	fn declared_id(&mut self) -> Result<String, ImportError> {
		let word = self.next_word("an ID followed by ':'")?;
		word.strip_suffix(':')
			.filter(|id| !id.is_empty())
			.map(str::to_string)
			.ok_or_else(|| {
				syntax(
					self.line,
					format!("expected an ID ending in ':', found '{word}'"),
				)
			})
	}

	fn finish(&mut self) -> Result<(), ImportError> {
		match self.words.next() {
			None => Ok(()),
			Some(extra) => Err(syntax(self.line, format!("unexpected '{extra}'"))),
		}
	}

	fn wall(&mut self) -> Result<FileWall, ImportError> {
		let id = self.declared_id()?;
		self.keyword("from")?;
		let start = self.position("from")?;
		self.keyword("to")?;
		let end = self.position("to")?;
		let sweep = if self.words.peek() == Some(&"sweep") {
			self.words.next();
			Some(self.number("sweep")? as f32)
		} else {
			None
		};
		self.keyword("height")?;
		let height = self.number("height")? as f32;
		self.keyword("thickness")?;
		let thickness = self.number("thickness")? as f32;
		self.finish()?;
		Ok(FileWall {
			id,
			start,
			end,
			sweep,
			dimensions: WallDimensions { thickness, height },
		})
	}

	fn opening(&mut self) -> Result<FileOpening, ImportError> {
		let id = self.declared_id()?;
		self.keyword("wall")?;
		let wall = self.next_word("a wall ID")?.to_string();
		self.keyword("center")?;
		let center_distance = self.number("center")?;
		self.keyword("width")?;
		let width = self.number("width")? as f32;
		self.keyword("bottom")?;
		let bottom = self.number("bottom")? as f32;
		self.keyword("height")?;
		let height = self.number("height")? as f32;
		self.keyword("category")?;
		let category = match self.next_word("'door' or 'window'")? {
			"door" => OpeningCategory::Door,
			"window" => OpeningCategory::Window,
			other => {
				return Err(syntax(
					self.line,
					format!("category must be 'door' or 'window', found '{other}'"),
				));
			}
		};
		self.finish()?;
		Ok(FileOpening {
			id,
			wall,
			spec: OpeningSpec {
				center_distance,
				width,
				bottom,
				height,
			},
			category,
		})
	}
}

#[cfg(test)]
#[path = "tests/project_file.rs"]
mod tests;
