//! nottio — a FigJam-shaped sticky board for Windows and macOS.
//!
//! Press `N` or the sticky icon next to Shapes, then click to place a note.
//! Type immediately, Escape to finish, drag to move, double-click to edit.
//! Selected notes and shapes can be linked through snapping arrow connectors.
//!
//! `src/app.rs` has the entry point. The modules beside it are the on-disk
//! format, the colour palette, the host chrome, and sticky mode.

#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod app;
mod images;
mod storage;
mod theme;
mod ui;

fn main() {
    app::main()
}
