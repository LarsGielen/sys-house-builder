#![doc = include_str!("../README.md")]

mod geometry;
mod graph;

pub use graph::{Wall, WallDimensions, WallError, WallGraph, WallGraphIdMap, WallNodeId};
