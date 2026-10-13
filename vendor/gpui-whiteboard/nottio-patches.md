# nottio local patch

Source: crates.io `gpui-whiteboard` 0.5.1, MIT, copyright Will Lehnertz.
Upstream files and bundled font/icon licenses are retained. `sticky.svg` is a
local icon. Cargo uses this directory, not a modified registry cache.

## Sticky objects

- `ElementKind::Sticky(BoxGeom)` is a distinct JSON `sticky` discriminator.
  Text is owned by the same element (`label`, `label_color`, `styles`), not an
  independently draggable text object. Existing rectangle boards still load;
  rectangles are not silently converted to stickies.
- `Tool::Sticky` is standalone, immediately after Shapes in the native toolbar;
  it moves with the toolbar in either orientation. Shortcut: N.
- A click places a 180 × 140 world-unit note and starts text editing. Escape
  finishes editing; selection mode enables moving the note on the next drag.
- Dragging from the sticky icon previews a fixed-size note and drops it on
  release. Returning to the toolbar, releasing outside the board, or pressing
  Escape cancels without adding an object or an undo entry.
- Shared box geometry supplies hit tests, resize, rotation, connectors and
  anchor following. Sticky fill and text colours default to yellow/dark ink.
- Japanese system fonts are preferred on Windows before other CJK fallbacks.

Rejected: host input veil with a separately positioned button. It bypassed the
board's tool routing and was not actually part of the requested toolbar.

Source of product choices: user conversation (separate sticky object, toolbar
next to Shapes, default size, drag, text, and arrow bindings).

Upstream 0.5.1 does not understand the new `sticky` tag; exports remain JSON but
require this patched library to read sticky objects.

## Editable connectors

- `SegGeom::bend` is an optional world-space elbow. Selected connectors show a
  circular midpoint/elbow handle; drag it to bend, Shift snaps to the grid,
  double-click or the Straight action removes the bend.
- `SegGeom::heads` optionally overrides start/end heads independently. Absent
  fields keep old boards' defaults (Arrow: end only; Line: no heads).
- Geometry includes the elbow for bounding, line-path hit tests, translation,
  resize and rotation. Rendering keeps dashed strokes and points each head
  along its local segment direction. Endpoint anchor bindings remain intact.
- GPUI keydown propagation is stopped for handled commands (e.g. Enter) but
  printable text still reaches the native input handler. An interaction test
  caught duplicate newlines in upstream's original event handling.

Source: user conversation, request to bend arrows and add/remove arrowheads.
Initial scope was one manual elbow; the later multiple-fold extension below
supersedes that limit.

## FigJam-like connector types (subsequent user request)

- `SegGeom::connector_type` persists `straight`, `bent` or `curved`; absent fields
  default to bent. The three native-toolbar actions retain endpoint bindings.
- Bent legs have small rounded corners; curves use a sampled quadratic whose
  middle handle lies on the actual path. Rendering and hit tests share the path.
- Arrow-tool gestures can begin inside an object, choosing its facing side while
  dragging. Creation and endpoint reconnection snap onto target object bodies.
- None/Solid heads remain independent. Extra FigJam cap types and separate
  per-section translation handles are outside this patch's implemented subset.

Source: subsequent user request to follow FigJam's connector behavior; primary
documentation and deliberate differences: `../../docs/figjam-connector-reference.md`.

## Automatic obstacle avoidance (subsequent user request)

- Default-on `SegGeom::auto_route` computes an orthogonal detour around padded
  object bounding boxes. Connector-side exit stubs keep the two ends attached.
  Manual bends override automatic routes; a toolbar action re-enables routing.
- `routing.rs` uses grid visibility edges plus axis-aware Dijkstra costs to
  prefer shorter routes with fewer bends. Geometry/settings fingerprints avoid
  recalculating when only the camera or selection changes.
- Intermediate coordinates and failure status are derived caches, not persisted
  JSON. Hit tests, selection handles and arrowhead tangents use the same path.
- Limits: 64 obstacles, 4096 grid nodes, 32 intermediate points. An unavailable
  route falls back with a visible warning. Arrow-to-arrow crossing prevention is
  not claimed. Existing mind-map connection styles keep their original renderer.

Source: user's subsequent request to detour arrows so they do not overlap objects.

## Multiple manual folds (latest user request)

- The legacy `bend` field remains the first handle. `extra_bends` stores up to
  32 more ordered handles as a compact JSON array; old one-handle boards load.
- **＋ 折り目** splits the longest handle interval and enters manual bent mode.
  Each handle drags independently. Double-click removes that fold;
  **− 折り目** removes the last. Straightening and enabling automatic routing
  explicitly clear all manual handles, retaining endpoint bindings and heads.
- Geometry, hit tests, rendering, translate/resize/rotate, undo/redo and JSON
  restore include all handles. Curved mode exposes only the first, retaining
  additional handles when returning to bent mode.

Source: user's latest request to increase the number of folds.

## Direct portion editing and sections (subsequent requests)

- The add/remove-fold toolbar was superseded by per-portion middle handles.
  `connector_edit.rs` moves interior portions on one axis, grows terminal doglegs
  without moving endpoints, and removes redundant vertices on release. Escape,
  undo/redo and no-op clicks preserve correct transaction boundaries.
- `manual_polyline` differentiates exact saved inner vertices from old waypoint
  controls, using the existing compact bend fields. Endpoint following maintains
  orthogonal terminal legs. X/Shift+L, L, V/Escape follow the connector shortcuts;
  Ctrl bypasses snapping in connector gestures without losing canvas pan elsewhere.
- `ElementKind::Frame(BoxGeom)`, standalone Frame tool / Shift+S: default square
  on click, custom bounds on drag, header name editable with Enter to finish.
- `frame.rs` derives single-owner membership from any positive overlap. Geometry
  bounds count for notes/shapes; line/pen paths use stroke intersection. Follower
  IDs freeze at grab time. Frame movement translates those followers once,
  preserving arrows' external anchors. Undo/redo and JSON restore all geometry.
- Frames remain background layers, cannot rotate/nest and are excluded from
  connector obstacles/targets. Resizing changes containment without scaling
  children. Removing/copying a frame acts on its boundary, not its contents.

Source: user requests for FigJam-like interaction and overlapping frame groups;
primary references and intentional differences are documented in the project.

## Right-angle invariant and clipboard-image fallback

- Diagonal manual Bent paths are canonicalized on load and when stored. Rendering
  normalizes sharp legs after transforms too. Endpoints and supplied waypoints
  survive via cardinal elbows/doglegs. Bent direct rotation is disabled; rotated
  terminal normals use world cardinal exits. Initial tool is Select.
- `PasteImageFn` supplies a durable source and pixel dimensions only after copied
  board JSON is unavailable. Ctrl+V and context-menu Paste share placement at the
  viewport center, selection, one undo snapshot and ordinary persistence flush.
  Text editing keeps its existing text-paste behavior.
- Image overlays use explicit pixel width/height and Fill, matching the model's
  nonuniform edge-resize handles without cropping. Hosts still own image bytes,
  decoding, quarter-turn variants and cache policy. nottio embeds bounded PNG
  pixels in JSON; no operating-system clipboard dependency was added here.

Source: user's diagonal-arrow screenshot and image-copy/paste request.
