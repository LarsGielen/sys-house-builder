#![doc = include_str!("../README.md")]

mod geometry;
mod graph;

pub use graph::{Wall, WallError, WallGraph, WallGraphIdMap, WallNodeId};
