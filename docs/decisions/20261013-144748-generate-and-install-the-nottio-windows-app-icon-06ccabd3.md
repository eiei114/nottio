---
type: Decision
title: "Generate and install the nottio Windows app icon"
decision_status: adopted
decision_kind: product
recorded_by: ai
review_status: unreviewed
adoption_mode: auto
decided_at: 2026-10-13T14:47:48Z
impact: normal
source_refs:
  - "conversation:current-user-icon-request"
supersedes: []
---

# Generate and install the nottio Windows app icon

## Decision

Generate an original sticky-note and orthogonal-arrow app icon, convert it to a multi-resolution ICO, and embed it as Windows ICON resource #1 so the executable shell icon and GPUI window class share the identity. Keep source artwork and regeneration tooling in the application repository.

## Rationale

User explicitly requested icon generation and configuration. GPUI Windows already loads resource #1; no runtime icon injection or framework patch is needed.

## Alternatives

Not recorded.

## Consequences

Windows builds use the SDK resource compiler through pinned embed-resource. Committed icon outputs make Pillow optional for regeneration only; board JSON is unchanged.
