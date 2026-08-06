// WD-40: shared library for dev artifact scanning and cleaning.
// Used by both the macOS menu bar app and the CLI.
pub mod cleaner;
pub mod config;
pub mod discover;
pub mod extents;
pub mod disk;
pub mod nesting;
pub mod roots;
pub mod scanner;
pub mod size_cache;
pub mod sizes;
pub mod toolchains;
mod walk;
