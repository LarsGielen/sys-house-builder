//! The validated house document: committed walls and openings with their generated mesh,
//! the proposals that change them, undo history, and the project file format.
//!
//! Nothing here knows about an engine, a window, or input. Every change goes through a
//! proposal that is validated completely before a [`House`] accepts it.

mod candidate;
mod document;
mod library;
mod opening_category;
mod project;

pub use candidate::{
	ArcDefinition, ArcProposal, BendProposal, EditCandidate, EditError, MoveProposal,
	OpeningEditProposal, OpeningProposal, Proposal, WallDimensionsProposal, WallProposal,
	opening_bounds, propose_arc, propose_bend, propose_move, propose_opening,
	propose_opening_duplicate, propose_opening_edit, propose_opening_removal, propose_wall,
	propose_wall_dimensions, propose_wall_removal,
};
pub use document::House;
pub use library::{HouseEntry, HouseLibrary, InvalidName, LibraryError, validate_name};
pub use opening_category::{OpeningCategories, OpeningCategory};
pub use project::io::{ProjectError, open_project, save_project};
