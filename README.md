# nottio

FigJam-shaped sticky board for Windows. Drop notes on an infinite canvas, draw
shapes, connect them with arrows, and it saves itself.

Built on [GPUI](https://gpui.rs) (the UI framework behind Zed) via
[gpui-whiteboard](https://crates.io/crates/gpui-whiteboard).

## Status

Local Windows prototype. Interaction regressions run through GPUI's rendered
element tree; a native Windows launch is also smoke-tested. Not a collaborative
FigJam replacement or a production-hardened release.

- Infinite canvas with pan and zoom; a 100% indicator in the corner.
- A separate **sticky object**, not a rectangle pretending to be a note.
  Drag the folded-note icon beside Shapes onto the canvas to place a 180 × 140
  note. A preview follows the pointer; release places it and starts typing.
  Alternatively click the icon (or press N), then click the canvas.
  Escape finishes text editing; drag to move the entire note, double-click to
  edit its text. Text is centred, wrapped, and shrunk to fit the note.
- Shapes: rectangle, rounded rectangle, ellipse, diamond, triangle, star,
  hexagon; freehand pen, straight lines, arrows.
- With the Arrow tool (A), drag from one sticky/shape body to another to connect
  their facing sides. Arrows and lines also **snap to connector points** and stay
  attached when the object moves, resizes or rotates. Select an endpoint and drag
  it to another object's body or side to reconnect it.
- The selected connector's **直線 / 折れ線 / 曲線** buttons switch between a direct
  line, orthogonal legs with rounded corners, and a smooth quadratic curve.
- Select an arrow/line and drag its **middle circular handle** to make a bend.
  **＋ 折り目** adds more independently draggable handles to a bent connector
  (up to 33). Double-click a handle to remove that fold; **− 折り目** removes the
  last one. Removing the only handle straightens the connector. **直線** clears
  every manual fold. Shift-drag snaps a handle to the grid. Curved mode has one
  on-path handle; returning to bent mode retains the additional saved handles.
- The selected connector's toolbar has **← 始点** and **終点 →** toggles.
  Add/remove arrowheads independently: no heads, either end, or both ends.
  Bends, heads, text and endpoint bindings persist with the board and undo/redo.
- **Automatic detours** avoid sticky notes and shapes with a padded rectangular
  clearance. Moving an object recomputes the path; camera changes do not.
  The connector's **迂回** button toggles this mode. Manual elbow edits take
  precedence; **直線** and **曲線** disable auto routing, while **迂回** re-enables
  bent routing and clears a manual bend. Old arrows default to auto routing too.
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
| `N` | Sticky note | | |

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

`NOTTIO_DATA_DIR` overrides the working directory for isolated smoke tests or a
portable board. Normal launches continue to use `%APPDATA%\nottio`. The current
scene is also saved synchronously on window close, so the debounce cannot drop
the last completed edit.

## Code layout

```
src/main.rs      entry point
src/app.rs       window, native menu, root view, and every board hook
src/ui.rs        status strip + the theme handle
src/theme.rs     both palettes
src/storage.rs   paths, atomic writes, UI-state file
vendor/gpui-whiteboard/  MIT upstream + native sticky/connector patches
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
cargo clippy --workspace --all-targets -- -D warnings
```

The suite covers the parts that are easy to get subtly wrong and hard to see by
looking: the UI-state round trip, hex-vs-decimal parsing of saved swatches,
atomic replacement of an existing export, and the clipboard sniffing that
decides whether a paste is board elements or ordinary text.

GPUI interaction tests additionally cover toolbar drag/drop, cancellation,
Japanese multiline text, dragging a note, undo/redo, actual sticky-to-shape
connector snapping, elbow editing, head toggles, attached-endpoint following,
and JSON restore. They use the test platform, not Windows IME automation.

Auto routing avoids objects, **not other arrows' crossings**. It uses bounded
orthogonal grid search, prefers fewer corners, and retains the endpoint anchors.
Overlapping terminal objects or search-capacity limits can make a route
unavailable; **迂回 !** then indicates fallback. Move the conflicting objects or
use a manual bend. Existing mind-map Bezier/orthogonal styles remain unchanged.

See [local patch notes](vendor/gpui-whiteboard/nottio-patches.md) for the schema
and licensing. Unpatched upstream 0.5.1 cannot read the new `sticky` variant.
Project terms and boundaries live in [context](docs/context.md); adopted user
requirements are recorded under `docs/decisions/`. The [FigJam reference](docs/figjam-connector-reference.md)
lists official behavior and the supported subset; this is not an exact clone.
