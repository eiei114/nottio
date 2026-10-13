---
type: Decision
title: "Use first-class draggable stickies and editable arrow connectors"
decision_status: adopted
decision_kind: product
recorded_by: ai
review_status: unreviewed
adoption_mode: auto
decided_at: 2026-10-13T06:57:26Z
impact: normal
source_refs:
  - "conversation:current"
supersedes: []
---

# Use first-class draggable stickies and editable arrow connectors

## Decision

Use GPUI for the Windows board. Give sticky notes their own persisted object type and a standalone toolbar button beside Shapes. Support dragging from that button, fixed-size placement, owned editable text, note movement, snapping connections, a draggable connector bend, and independent start/end arrowheads.

## Rationale

The user explicitly requested FigJam-like sticky cards separate from ordinary shapes, followed by drag creation and editable arrow bends and heads.

## Alternatives

Not recorded.

## Consequences

Local gpui-whiteboard 0.5.1 patch adds the sticky JSON discriminator, optional bend and heads fields; old shapes retain their type. One elbow per connector, not automatic routing. See vendor/gpui-whiteboard/nottio-patches.md.
