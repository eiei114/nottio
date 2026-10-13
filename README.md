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
- **Paste images**: copy a screenshot or raster image, focus the canvas, then
  **Ctrl+V** (or right-click → Paste). It becomes a selected image object,
  initially 280 screen pixels on its longest edge. Drag to move; use selection
  handles to resize. Images also follow overlapping frames and support undo/redo.
  PNG bytes are embedded in the JSON, so reopen, Save a copy and image-object
  copy/paste retain the image without separate files. While editing text, Ctrl+V
  remains text paste; finish editing with Escape before pasting an image.
- With the Arrow tool (A), drag from one sticky/shape body to another to connect
  their facing sides. Arrows and lines also **snap to connector points** and stay
  attached when the object moves, resizes or rotates. Select an endpoint and drag
  it to another object's body or side to reconnect it.
- The selected connector's **直線 / 折れ線 / 曲線** buttons switch between a direct
  line, orthogonal legs with rounded corners, and a smooth quadratic curve.
  Bent paths never use diagonal sharp legs, including old manual paths loaded
  from disk. Their direct rotation handle is disabled; straight/curved remain
  explicit alternatives. The board opens in Select mode, ready to move objects.
- Select a bent connector and **drag a portion's middle handle**. Horizontal
  portions move vertically, vertical portions move horizontally. Dragging an end
  portion automatically adds folds while keeping the attached endpoint fixed.
  Move portions back into alignment to collapse unnecessary folds. No add/remove
  fold buttons. Shift snaps the portion to the grid. Curves retain one on-path
  handle; straight connectors expose endpoint handles, not bending handles.
- **Frames / Sections**: use the frame icon beside Sticky, or **Shift+S**. Click
  the canvas for a 400 × 400 square, or drag to draw a custom frame over existing
  objects. Any positive overlap groups notes/shapes; crossing lines are included
  by their actual stroke path. Drag the frame's header or empty background to
  move its contents together. Moving an object fully outside releases it.
  Double-click the name and press Enter to rename. Resizing changes the boundary,
  not the contents' size. Frames always remain behind their contents.
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
| `V` | Select | `J` | Hexagon |
| `P` | Pen | `L` | Line |
| `R` | Rectangle | `A` | Arrow |
| `O` | Ellipse | `T` | Text |
| `D` | Diamond | | |
| `G` | Triangle | | |
| `U` | Rounded rectangle | | |
| `N` | Sticky note | | |

`X` / `Shift+L`: bent connector. `Shift+S`: frame/section. `V` / Escape exits
connector creation. Ctrl bypasses endpoint snapping during connector gestures;
Ctrl-drag elsewhere still pans. Alt additionally grid-snaps a detached endpoint.

`Ctrl+Z` / `Ctrl+Shift+Z` undo and redo; `Ctrl+C` / `Ctrl+X` / `Ctrl+V` copy,
cut and paste elements (Ctrl+V also pastes clipboard images); `Ctrl+]` / `Ctrl+[` change z-order; `Ctrl+B` / `Ctrl+I`
/ `Ctrl+U` / `Ctrl+Shift+X` / `Ctrl+Shift+H` apply bold, italic, underline,
strikethrough and highlight to text.

Image paste accepts raster bitmap entries exposed by GPUI's Windows clipboard
reader (including PNG, JPEG and CF_DIB/BMP). It normalizes to a static PNG and
preserves transparency where the source contains it. Limits: 32 MiB encoded
input/output, 8192 pixels per side, 16 million pixels. SVG, animation and Explorer
file-path paste are not supported. Imported external image URLs/files are not
loaded. Invalid images show an error without changing the board. Large images
increase JSON size because the board is self-contained.

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

Frame membership is derived from overlap and frozen during a frame drag. Objects
crossed while dragging are not picked up mid-gesture; membership is reevaluated
on the next grab. Overlapping frames use the smallest frame, then the frontmost
on ties. Nested frames are not supported. Deleting a frame leaves its objects;
frame-only copy/export does not implicitly include its contents. Connections to
objects outside a moving frame retain their external attachment instead of
being detached. This is a local section prototype, not every FigJam feature.

See [local patch notes](vendor/gpui-whiteboard/nottio-patches.md) for the schema
and licensing. Unpatched upstream 0.5.1 cannot read the new `sticky` variant.
Project terms and boundaries live in [context](docs/context.md); adopted user
requirements are recorded under `docs/decisions/`. The [FigJam reference](docs/figjam-connector-reference.md)
lists official behavior and the supported subset; this is not an exact clone.
