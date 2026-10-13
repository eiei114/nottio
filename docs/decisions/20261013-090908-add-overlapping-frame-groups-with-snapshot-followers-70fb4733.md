---
type: Decision
title: "Add overlapping frame groups with snapshot followers"
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
supersedes: []
---

# Add overlapping frame groups with snapshot followers

## Decision

Add FigJam-style background frames with click or drag creation and editable headers. Any positive overlap groups notes, shapes and strokes. Moving a frame moves its grab-time followers together while preserving external connector bindings.

## Rationale

The user explicitly requested frame groups and membership even when an object overlaps only slightly. Freezing follower IDs prevents accidental pickup during the gesture.

## Alternatives

Not recorded.

## Consequences

Membership derives from saved geometry, with the smallest overlapping frame as owner. Frames stay behind content, resizing only changes bounds, and members can leave independently. Nested frames and implicit copy/delete of children are outside this prototype.
