# nottio

FigJam-shaped sticky board for Windows. Drop notes on an infinite canvas, draw
shapes, connect them with arrows, and it saves itself.

Built on [GPUI](https://gpui.rs) (the UI framework behind Zed) via
[gpui-whiteboard](https://crates.io/crates/gpui-whiteboard).

## Status

Working first cut. Everything below is verified running on Windows 11:

- Infinite canvas with pan and zoom; a 100% indicator in the corner.
- Pasting notes: the rectangle/round-rect tools, double-click a shape to type a
  label into it (centred, word-wrapped, auto-shrunk to fit).
- Shapes: rectangle, rounded rectangle, ellipse, diamond, triangle, star,
  hexagon; freehand pen, straight lines, arrows.
- Arrows and lines **snap to a shape's connector points** and stay attached when
  the shape moves, resizes or rotates.
- Select / move / resize / rotate, multi-select, marquee, z-order, undo/redo.
- Colour picker with a saved-swatch palette; per-element stroke, fill and text
  colour.
- Mind-map and flowchart seed templates.
- Autosave to `%APPDATA%\nottio\board.json`, plus **File ▸ Save a copy…** and
  **File ▸ Open board…** through native dialogs.
- Light and dark themes; the toolbar can be dragged anywhere on the board and
  remembers where it was left.

## Running it

```sh
cargo run            # development
cargo build --release
```

Requirements are the same as Zed's: a stable Rust toolchain with the
`x86_64-pc-windows-msvc` target, Visual Studio Build Tools with the C++ desktop
workload, and the Windows SDK. No CMake is needed.

## Keyboard

Tool shortcuts are single keys, active whenever the canvas has focus:

| Key | Tool | Key | Tool |
| --- | --- | --- | --- |
| `H` | Pan | `S` | Star |
| `V` | Select | `X` | Hexagon |
| `P` | Pen | `L` | Line |
| `R` | Rectangle | `A` | Arrow |
| `O` | Ellipse | `T` | Text |
| `D` | Diamond | | |
| `G` | Triangle | | |
| `U` | Rounded rectangle | | |

`Ctrl+Z` / `Ctrl+Shift+Z` undo and redo; `Ctrl+C` / `Ctrl+X` / `Ctrl+V` copy,
cut and paste elements; `Ctrl+]` / `Ctrl+[` change z-order; `Ctrl+B` / `Ctrl+I`
/ `Ctrl+U` / `Ctrl+Shift+X` / `Ctrl+Shift+H` apply bold, italic, underline,
strikethrough and highlight to text.

Drag the toolbar's grip to move it, double-click the grip to reset it, and use
the grip's other affordance to flip it between a row and a column.

## Where things live

| Path | What |
| --- | --- |
| `%APPDATA%\nottio\board.json` | The working board. Rewritten on every change. |
| `%APPDATA%\nottio\ui-state.json` | Theme, toolbar position, saved swatches. |

The working board is a cache, not a document — a copy saved through **Save a
copy…** is the thing worth backing up.

## Code layout

```
src/main.rs      entry point
src/app.rs       window, native menu, root view, and every board hook
src/ui.rs        status strip + the theme handle
src/theme.rs     both palettes
src/storage.rs   paths, atomic writes, UI-state file
```

`app.rs` is where the interesting decisions are, and each one is commented where
it lives. The short version:

- **GPUI is pinned to a snapshot.** Zed publishes GPUI to crates.io as the
  `gpui-pre` family, where each version is a different Zed revision, so the app
  and `gpui-whiteboard` must resolve to the same one. `Cargo.toml` pins
  `=0.3.9` exactly; treat a bump as a code change, not a version bump.
- **The theme is a shared handle, not a value.** `WhiteboardView::new` takes the
  style as a closure and never hands it back, so the closure captures a
  `Rc<Cell<ThemeKind>>` and flipping it re-themes the live board — rebuilding the
  view instead would throw away the undo history, camera and selection.
- **Autosave is debounced by generation counter.** Every change bumps the
  counter and schedules a timer; a timer that wakes up stale exits without
  writing. A drag therefore collapses into one disk write, and the last edit of
  a burst is the one that lands.
- **Opening a board rebuilds the entity.** The crate has no `set_scene`, and
  discarding the undo history is the correct behaviour when a different document
  is loaded.

## Tests

```sh
cargo test
cargo clippy --all-targets
```

The suite covers the parts that are easy to get subtly wrong and hard to see by
looking: the UI-state round trip, hex-vs-decimal parsing of saved swatches,
atomic replacement of an existing export, and the clipboard sniffing that
decides whether a paste is board elements or ordinary text.
