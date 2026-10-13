---
type: Decision
title: "Paste clipboard images as durable board objects"
decision_status: adopted
decision_kind: product
recorded_by: ai
review_status: unreviewed
adoption_mode: auto
decided_at: 2026-10-13T09:40:27Z
impact: normal
source_refs:
  - "conversation:current-user-image-paste-request"
supersedes: []
---

# Paste clipboard images as durable board objects

## Decision

Support Ctrl+V and context-menu Paste for raster clipboard images, as separate movable and resizable image objects. Preserve image bytes in board JSON as embedded PNG data sources so autosave, reopen, export and object copy do not depend on temporary files. Board-selection JSON has priority over image fallback.

## Rationale

User explicitly requested image copy/paste. Embedded PNG keeps the local single-file board durable and portable; raster normalization, size limits and quarter-turn cache are implementation choices.

## Alternatives

Not recorded.

## Consequences

Initial long edge is 280 screen pixels. Raster inputs are normalized to static PNG; SVG and external file-path paste are not supported. Limits: 32MiB encoded source/PNG, 8192 pixels per side, 16 million pixels. No remote or arbitrary local-file source loading.
