//! Game model and simulation of Harry the Handsome Executive.
//!
//! This crate does no I/O. Routines are ported one by one from the
//! original's PowerPC build (names and addresses in the doc comments refer
//! to it; see NOTES.md). One game frame is one pass of the original's main
//! loop, 30 per second.

pub mod air;
pub mod bounce;
pub mod frame;
pub mod inventory;
pub mod keys;
pub mod kinds;
pub mod level;
pub mod mask;
pub mod moves;
pub mod physics;
pub mod player;
pub mod random;
pub mod sat;
pub mod shots;
pub mod sprite;
pub mod tiles;
pub mod touch;

pub use level::Level;
pub use physics::{Physics, Speed};
