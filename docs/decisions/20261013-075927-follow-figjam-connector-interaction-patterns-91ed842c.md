---
type: Decision
title: "Follow FigJam connector interaction patterns"
decision_status: adopted
decision_kind: product
recorded_by: ai
review_status: unreviewed
adoption_mode: auto
decided_at: 2026-10-13T07:59:27Z
impact: normal
source_refs:
  - "conversation:current"
  - "docs/figjam-connector-reference.md"
supersedes: []
---

# Follow FigJam connector interaction patterns

## Decision

Use FigJam as the connector interaction reference: support straight, bent and curved line types, draggable reshape handles, reconnectable snapped endpoints and independent start/end heads.

## Rationale

The user explicitly requested the same style of arrow interaction as FigJam after requesting manual bends and detours.

## Alternatives

Not recorded.

## Consequences

Implemented subset: one manual handle and None/Solid caps. Automatic object avoidance is additional behavior for bent connectors, not a claim of parity with FigJam or arrow-crossing prevention.
