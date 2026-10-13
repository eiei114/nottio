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
- **Bend**: an ordered user-positioned path handle, shown on selection. Bent
  connectors support up to 33 independent handles, connected by orthogonal legs.
  Curved connectors expose one on-path handle, not cubic control points.
- **Auto route**: derived orthogonal detour around the padded bounds of board
  objects. Recomputed on object movement, not on camera changes. Manual bends
  override it; only bent connectors auto-route. Arrow-to-arrow crossing avoidance
  is not part of this feature.
- **Arrowhead**: independently enabled start/end markers. Geometry direction
  does not need reversing to show a start head instead of an end head.
- **Working board**: autosaved local JSON; export/import are explicit file actions.

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
`figjam-connector-reference.md`. None/Solid heads and ordered manual bends are the
current subset; richer endpoint styles and separate per-section translation
handles are deferred. `bend` stores the first handle for backwards compatibility;
`extra_bends` stores additional handles as a compact JSON array. They are document
data, unlike the derived automatic route cache.
