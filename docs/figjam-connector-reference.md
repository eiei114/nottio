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
- **＋ 折り目 / − 折り目** add or remove ordered manual bends. Each bent handle is
  independently draggable; double-click removes that specific fold. Up to 33
  handles are supported. Adding folds enters manual bent mode, without detaching
  endpoints or changing arrowheads. Straightening/automatic routing clear them;
  changing to curved mode hides the additional handles until bent mode returns.
- Independent **← 始点 / 終点 →** toggles support None and Solid heads at either
  end. Existing arrow defaults remain end-only; lines default to no heads.
- Default bent routing detours around notes/shapes. Manual bends take precedence.
  **迂回** clears the manual bend and enables automatic bent routing. Straight and
  curved choices disable it. Derived coordinates are not persisted.
- Type, bend, heads and attachments survive JSON save/restore and undo/redo.

## Deliberate differences and limits

This is not an exact FigJam clone. Ordered bend handles are supported, not a
separate translation handle for every polyline portion. Line/Triangle/Diamond caps and connector-label
position handles are not implemented. A/L are the local tool shortcuts; Ctrl
still pans, while Alt bypasses snapping during connector creation. During
endpoint editing Alt detaches and grid-snaps. Manual bent stubs use axis-aligned
side directions; automatic routes account for rotated terminals.

Automatic avoidance is an additional user requirement. It avoids padded object
bounds, **not arrow-to-arrow crossings**. Overlapping terminals and bounded search
capacity can prevent a valid route; **迂回 !** reports fallback. Curves and manual
bends do not promise obstacle avoidance.

Verification: `sticky_tests.rs` covers the rendered GPUI input tree for body-to-body
creation, endpoint reconnection, three-type switching, curve-handle editing,
multiple-fold addition/removal, individual dragging, cancellation, transforms,
bindings, routing, JSON restore and undo/redo. This is not a claim that every
native Windows input method has been tested.

[help]: https://help.figma.com/hc/en-us/articles/1500004414542-Create-diagrams-and-flows-with-connectors-in-FigJam
[node]: https://developers.figma.com/docs/plugins/api/ConnectorNode/
[endpoint]: https://developers.figma.com/docs/plugins/api/ConnectorEndpoint/
