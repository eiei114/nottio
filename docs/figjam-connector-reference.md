# FigJam connector reference

The user requested FigJam-like connectors after requesting editable bends,
independent arrowheads and detours. This note distinguishes the official
reference from this local prototype's implemented subset.

## Official behavior

[Figma Help: Create diagrams and flows with connectors in FigJam][help] documents:

- Bent, curved and straight connectors; solid or dashed strokes.
- Dragging between objects attaches connectors to their sides. Attached lines
  continue following objects when they move.
- Select a connector, then drag the blue endpoint dot to reconnect it to another
  object or another side of the same object.
- Drag path handles to reshape portions. Moving a start/end portion can add
  another portion to the path.
- Start and end styles are independent: None, Solid, Line, Triangle, Diamond.
- On Windows, Ctrl bypasses side snapping when positioning an endpoint.

The [ConnectorNode API][node] independently exposes `connectorLineType` values
`ELBOWED`, `STRAIGHT`, `CURVED`, `connectorStart`, `connectorEnd`, and independent
`connectorStartStrokeCap` / `connectorEndStrokeCap`. The [ConnectorEndpoint
API][endpoint] describes the endpoints' attachment representation.

The Help page says bent connectors can wrap around objects and describes manual
path editing. It does **not** promise automatic avoidance of every object or of
crossings with other connectors.

## nottio implementation

- **直線 / 折れ線 / 曲線** buttons on the selected connector's native toolbar.
  New arrows remember the most recently chosen type within the current session.
- Arrow-tool drag from a sticky/shape body selects its facing connection side.
  Release over another object snaps to its facing side. Endpoints can be dragged
  onto a different object body or a specific side; bindings follow movement.
- A circular handle directly reshapes the path. Bent lines have orthogonal legs
  and rounded corners. Curves use a sampled quadratic passing through the handle,
  rather than an invisible off-path control point.
- Bent paths expose **a middle handle on each portion**, instead of add/remove
  fold buttons. Dragging moves that portion perpendicular to its direction.
  Start/end portions grow doglegs automatically, retaining endpoint attachments.
  Bringing portions into alignment collapses redundant folds on release. Clicking
  a handle without dragging creates no undo entry. Escape restores the whole path.
  Up to 33 inner vertices can be saved as an explicit manual polyline.
- Independent **← 始点 / 終点 →** toggles support None and Solid heads at either
  end. Existing arrow defaults remain end-only; lines default to no heads.
- Default bent routing detours around notes/shapes. Manual bends take precedence.
  **迂回** clears the manual bend and enables automatic bent routing. Straight and
  curved choices disable it. Derived coordinates are not persisted.
- Type, bend, heads and attachments survive JSON save/restore and undo/redo.

## Deliberate differences and limits

This is not an exact FigJam clone. Line/Triangle/Diamond caps and connector-label
position handles are not implemented. X / Shift+L select bent connectors, L a
straight line, V / Escape exit drawing. A remains a local alias that remembers
the chosen type; the hexagon shortcut moved to J. Ctrl bypasses attachment
snapping during connector gestures (unlike FigJam's arbitrary on-object bound
endpoint). Elsewhere Ctrl still pans; Alt detaches and grid-snaps during endpoint
editing. Legacy waypoint stubs use axis-aligned side directions; automatic routes
account for rotated terminals. Converting a portion-edited path to curved mode
resets its manual vertices; undo can restore that conversion.

The user's diagonal-path screenshot added a strict local invariant: every sharp
bent leg is horizontal or vertical. Imported manual vertices are canonicalized
with axis-aligned elbows/doglegs, preserving endpoints and supplied waypoints.
Bent paths no longer expose direct rotation. Rotated terminals use cardinal
exit axes; rounded corners remain smooth. Straight/curved modes are unaffected.

Automatic avoidance is an additional user requirement. It avoids padded object
bounds, **not arrow-to-arrow crossings**. Overlapping terminals and bounded search
capacity can prevent a valid route; **迂回 !** reports fallback. Curves and manual
bends do not promise obstacle avoidance.

Verification: `sticky_tests.rs` covers the rendered GPUI input tree for body-to-body
creation, endpoint reconnection, three-type switching, curve-handle editing,
terminal fold growth, interior portion translation, collapse, cancellation, transforms,
bindings, routing, JSON restore and undo/redo. This is not a claim that every
native Windows input method has been tested.

## Frame/section reference

[Organize your FigJam board with sections][sections] documents Shift+S, canvas
click/drag creation, drawing over existing objects, moving objects into a section,
moving a section's contents together and double-clicking its name to rename.
It does not define a partial-overlap threshold in the extracted documentation.
nottio follows these core gestures; **any positive overlap** is the user's explicit
additional rule, not an inferred Figma specification. Frame copy-with-contents,
nested sections, section locking and collaboration remain outside this subset.

[sections]: https://help.figma.com/hc/en-us/articles/4939765379351-Organize-your-FigJam-board-with-sections

[help]: https://help.figma.com/hc/en-us/articles/1500004414542-Create-diagrams-and-flows-with-connectors-in-FigJam
[node]: https://developers.figma.com/docs/plugins/api/ConnectorNode/
[endpoint]: https://developers.figma.com/docs/plugins/api/ConnectorEndpoint/
