//! Game model and simulation of Harry the Handsome Executive.
//!
//! This crate does no I/O. Routines are ported one by one from the
//! original's PowerPC build (names and addresses in the doc comments refer
//! to it; see NOTES.md). One game frame is one pass of the original's main
//! loop, 30 per second.

pub mod air;
pub mod blob;
pub mod boss;
pub mod boulder;
pub mod bounce;
pub mod button;
pub mod collider;
pub mod coworker;
pub mod dart_enemy;
pub mod death;
pub mod effect;
pub mod enemy_shot;
pub mod firepipe;
pub mod frame;
pub mod furniture;
pub mod game;
pub mod ghost;
pub mod grate;
pub mod inventory;
pub mod keys;
pub mod kinds;
pub mod level;
pub mod mailguy;
pub mod mask;
pub mod moves;
pub mod physics;
pub mod player;
pub mod props;
pub mod ramp;
pub mod random;
pub mod reversor;
pub mod robot;
pub mod sat;
pub mod scientist;
pub mod shots;
pub mod sound;
pub mod sprite;
pub mod stairs;
pub mod tiles;
pub mod text;
pub mod touch;
pub mod turret;
pub mod view;
pub mod words;
pub mod world;

pub use level::Level;
pub use physics::{Physics, Speed};
