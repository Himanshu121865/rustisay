//! Library core shared by the CLI binary and the WebAssembly build.
//!
//! The CLI binary additionally declares the terminal-only modules (`cli`,
//! `player`, `term`) for itself; this library exposes only the parts that
//! are pure and wasm-safe. The `wasm` module is compiled only for wasm32.

pub mod alphabet;
pub mod color;
pub mod convert;
pub mod font;
pub mod gif;
pub mod render;

#[cfg(target_arch = "wasm32")]
pub mod wasm;
