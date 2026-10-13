# nottio context

Local Windows GPUI board prototype, developed in the Production/nottio repository.
Source: current user conversation. This folder is the implementation boundary;
there is no server, account system, collaboration service or web frontend.

## Vocabulary

- **Sticky**: a first-class `ElementKind::Sticky` object, not an ordinary shape.
  Fixed default size, yellow fill, owned multiline text; movable as one object.
- **Shape**: rectangle, triangle and other closed shapes. They retain their
  existing types and label support; they are not silently migrated into notes.
- **Connector**: line/arrow with optional endpoint bindings to a sticky or shape.
  Bindings store stable element IDs and a side index and follow object movement.
- **Connector type**: straight, bent (orthogonal legs with rounded corners), or
  curved (sampled quadratic). Stored independently of solid/dashed stroke style.
- **Portion**: a sharp bent-path segment between consecutive vertices. Its middle
  handle moves perpendicular to that segment. Dragging terminal portions inserts
  doglegs while endpoints remain fixed; redundant vertices collapse on release.
- **Manual polyline**: a portion-edited path. `manual_polyline: true` makes `bend`
  and `extra_bends` exact inner vertices, not the older waypoint-control model.
  Up to 33 inner vertices are persisted. Legacy waypoint documents still load.
  Curved connectors expose one on-path handle, not cubic control points.
- **Frame / Section**: a background `ElementKind::Frame(BoxGeom)` with a title
  above its top edge. Membership derives from positive overlap; notes/shapes use
  bounds and strokes use actual path intersection. No clipping or parent IDs.
- **Frame drag snapshot**: frozen follower IDs expanded from the selected frame
  at mouse-down. Capturing only once prevents scooping up unrelated objects while
  crossing them. Outside connector anchors remain attached when the group moves.
- **Auto route**: derived orthogonal detour around the padded bounds of board
  objects. Recomputed on object movement, not on camera changes. Manual bends
  override it; only bent connectors auto-route. Arrow-to-arrow crossing avoidance
  is not part of this feature.
- **Arrowhead**: independently enabled start/end markers. Geometry direction
  does not need reversing to show a start head instead of an end head.
- **Working board**: autosaved local JSON; export/import are explicit file actions.
- **Clipboard image**: a separate `ElementKind::Image` with an embedded PNG
  `data:image/png;base64,` source. Owns its durable pixels rather than a temp path;
  movement, resizing, quarter-turn rotation and frame membership reuse box logic.

## Ownership and boundaries

`src/app.rs` owns the window and persistence/clipboard/menu hooks. The host chrome
in `src/ui.rs` contributes a status strip, not a second creation toolbar.
`vendor/gpui-whiteboard` owns the actual toolbar, object model, input routing,
text editor, transforms, connectors and undo history. Preserve MIT attribution
and bundled font/icon licenses when updating the vendored source.

Missing upstream toolbar extension slots led to a local patch, not a host input
veil or registry cache edits. Details: `../vendor/gpui-whiteboard/nottio-patches.md`.
Old JSON loads without new connector fields; new `sticky` JSON requires this
patched library. No backwards export into unpatched upstream is claimed.

## Verification

GPUI test-platform input dispatch covers dragging off the real sticky toolbar
button, text entry, movement, cancellation, undo/redo, genuine connector snapping,
bending, three-type switching, body-to-body connector creation, endpoint
reconnection, head toggles and bindings following moved notes. These tests do not
certify every Windows IME or accessibility path. Native smoke tests use a
separate data directory and must never delete the user's working board.

`routing.rs` performs bounded grid search with a bend penalty and connector-side
terminal exits. Only `auto_route` is persisted; derived waypoints are cached and
not saved. Cache keys include object geometry and connector settings, never the
camera. Search limits are 64 objects, 4096 grid nodes and 32 intermediate points;
an unavailable route shows a toolbar warning instead of claiming safe avoidance.

The official behavior reference and intentional differences are documented in
`figjam-connector-reference.md`. None/Solid heads and portion translation handles
are the current subset; richer endpoint styles are deferred. `bend` stores the
first vertex for backwards compatibility;
`extra_bends` stores more as a compact array. Portion translation handles are now
implemented, superseding the earlier add/remove-fold toolbar. These vertices are
document data, unlike the derived automatic route cache.

`frame.rs` owns overlap membership and selection expansion. Frames are always
background layers and are excluded from connector obstacles/attachment targets.
Their title is editable, bounds resizable and group movement undoable. Overlapping
frames choose the smallest owner, then frontmost on ties. Frames do not nest;
resizing does not scale children; deleting/copying a frame acts on its boundary
only. Saved geometry deterministically restores membership without extra IDs.

Bent sharp paths are normalized to cardinal legs on load and before manual-path
storage. Rendering also enforces this invariant after group transforms. Rotated
terminal exit directions choose a world cardinal axis. Direct bent rotation is
disabled, and the initial board tool is Select. Rounded corners are unchanged.

`src/images.rs` owns bounded raster decoding, EXIF orientation, PNG normalization
and an encoded quarter-turn LRU (128 MiB). GPUI handles asynchronous image upload.
Clipboard JSON is tried first, then `PasteImageFn` returns durable pixels and
dimensions; the board creates/selects the image and emits its ordinary save hook.
The host's `ImageFn` renders only embedded PNG sources: no arbitrary network or
file access. 32 MiB input/output, 8192px sides and 16M pixels bound each image.
Raster normalization uses the first frame, not animation. No image file-picker,
Explorer path paste or SVG import is wired. Images travel with board export and
selection copy; many images therefore increase JSON and undo-snapshot sizes.

## Windows application identity

`assets/nottio-icon-source.png` is the generated porcelain/blue-violet folded
mark, superseding the initial literal sticky/arrow artwork. It follows the user's
request for a more modern software identity; primary-source comparison is in
`modern-app-icon-research.md`. `scripts/generate-icon.py` crops this source's tile,
applies a clean supersampled rounded mask, and derives a 1024px PNG and a ten-size
16–256px ICO; these outputs are committed, so Pillow is not a build dependency.
`build.rs` uses pinned `embed-resource` to link `assets/windows.rc` into the
Windows `nottio` executable. ICON group ID **1** matches GPUI Windows' native
`load_icon()`/window-class registration, covering shell, title-bar and taskbar
identity without a runtime path or a patched framework. Resource compilation
fails closed; non-Windows targets skip it. Existing board data is unrelated.

A pre-existing taskbar pin has separate icon metadata/cache. On this machine
`nottio.lnk` had an empty icon filename (`,0`) even though the executable resource
and GPUI class icon were correct. `scripts/set-taskbar-icon.ps1 -Apply` sets only
that matching pin's IconLocation to a verified copy of `assets/nottio.ico` under
`%APPDATA%/nottio/icons/nottio-<sha256-prefix>.ico,0`, backs up the link and
issues a targeted shell update. It preserves target/arguments/workdir and leaves
AppUserModelID/grouping unchanged. It never creates/re-pins applications or clears
the global Explorer icon cache. Non-Apply mode is read-only; Apply is idempotent.
The content-addressed path refreshes redesigned artwork without reusing an old
ICO cache key. Old copies remain so backed-up shortcuts still resolve. The
source image, PNG and all ICO resolutions are checked by `test-icon-assets.py`.
If a running/pinned group keeps an old image despite the link update, optional
`-Apply -RefreshShell` sends SHCNE_ASSOCCHANGED as a standard redraw notification.
This is broader than the one-link update (other icons may repaint), but changes
no associations and deletes no cache files. Read-only mode rejects that flag.
