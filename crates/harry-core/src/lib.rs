//! Game model and simulation of Harry the Handsome Executive.
//!
//! This crate does no I/O. Routines are ported one by one from the
//! original's PowerPC build (names and addresses in the doc comments refer
//! to it; see NOTES.md). One game frame is one pass of the original's main
//! loop, 30 per second.

pub mod bounce;
pub mod mask;
pub mod moves;
pub mod physics;

pub use physics::{Physics, Speed};
