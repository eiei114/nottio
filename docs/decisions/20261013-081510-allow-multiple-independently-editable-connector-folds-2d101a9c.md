---
type: Decision
title: "Allow multiple independently editable connector folds"
decision_status: adopted
decision_kind: product
recorded_by: ai
review_status: unreviewed
adoption_mode: auto
decided_at: 2026-10-13T08:15:10Z
impact: normal
source_refs:
  - "conversation:current"
supersedes:
  - "docs/decisions/20261013-075927-follow-figjam-connector-interaction-patterns-91ed842c.md"
---

# Allow multiple independently editable connector folds

## Decision

Allow adding and removing ordered manual folds on bent connectors. Each fold has its own draggable handle; preserve attachment IDs, arrowheads, undo/redo and JSON data.

## Rationale

The user explicitly requested increasing the number of folds after trying the single-handle connector.

## Alternatives

Not recorded.

## Consequences

Supersedes the one-manual-elbow scope in earlier connector decisions. Supports 33 manual handles; automatic routing and straightening clear them explicitly. Curved mode still uses one on-path handle while retaining additional bent handles.
