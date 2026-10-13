//! nottio — a FigJam-shaped sticky board for Windows.
//!
//! Double-click the canvas to drop a note, drag shapes and arrows between them,
//! and the board saves itself. `src/app.rs` has the entry point; the two modules
//! beside it are the on-disk format and the colour palette.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod storage;
mod theme;
mod ui;

fn main() {
    app::main()
}
