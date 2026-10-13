---
type: Decision
title: "Use direct FigJam-style connector portion editing"
decision_status: adopted
decision_kind: product
recorded_by: ai
review_status: unreviewed
adoption_mode: auto
decided_at: 2026-10-13T09:09:08Z
impact: normal
source_refs:
  - "conversation:current"
  - "docs/figjam-connector-reference.md"
supersedes:
  - "docs/decisions/20261013-081510-allow-multiple-independently-editable-connector-folds-2d101a9c.md"
---

# Use direct FigJam-style connector portion editing

## Decision

Replace add/remove-fold buttons with draggable handles on connector portions. Interior portions translate on one axis; terminal portions grow folds without moving attached endpoints. Collapsing redundant folds, Escape, undo and JSON restore remain supported.

## Rationale

The user requested the same UX as FigJam rather than a separate add-fold button workflow. Official guidance describes moving path portions and growing terminal portions.

## Alternatives

Not recorded.

## Consequences

Supersedes the add/remove toolbar workflow; legacy saved bend points still load. Supports bounded manual polylines, not every FigJam cap, label or collaboration feature.
