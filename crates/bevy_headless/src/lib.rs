//! Renders any Bevy app to images without a window or display server.
//!
//! [`HeadlessApp`] builds an app the way a normal `main` would, but steps it frame by frame
//! and redirects cameras that target the primary window to an off-screen image. That includes
//! Bevy UI, so 2D, 3D and UI screens all render unchanged.
//!
//! ```ignore
//! use bevy_headless::{CaptureArgs, HeadlessApp};
//!
//! let args = CaptureArgs::from_env();
//! let mut app = HeadlessApp::new(args.config, |app| {
//!     // add the same plugins and scene as the windowed main
//! });
//! app.settle();
//! app.screenshot(&args.out).expect("screenshot");
//! ```
//!
//! Adapter choice follows wgpu's environment variables (`WGPU_BACKEND`, `WGPU_ADAPTER_NAME`),
//! and [`HeadlessConfig::software_rendering`] selects the CPU adapter.

mod app;
mod args;
mod config;
mod target;

pub use app::{HeadlessApp, HeadlessError};
pub use args::CaptureArgs;
pub use config::HeadlessConfig;
