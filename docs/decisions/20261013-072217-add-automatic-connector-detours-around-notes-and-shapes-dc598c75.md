---
type: Decision
title: "Add automatic connector detours around notes and shapes"
decision_status: adopted
decision_kind: product
recorded_by: ai
review_status: unreviewed
adoption_mode: auto
decided_at: 2026-10-13T07:22:17Z
impact: normal
source_refs:
  - "conversation:current"
supersedes:
  - "docs/decisions/20261013-065726-use-first-class-draggable-stickies-and-editable-arrow-connectors-51d265b8.md"
---

# Add automatic connector detours around notes and shapes

## Decision

Add default-on automatic orthogonal obstacle avoidance for connectors. Preserve endpoint bindings, reroute when objects move, let manual bends override the automatic path, and expose a per-connector toggle.

## Rationale

The user subsequently requested arrows that detour to avoid overlapping objects, extending the original manual-elbow scope.

## Alternatives

Not recorded.

## Consequences

The initial no-automatic-routing consequence is superseded. Manual editing still has one elbow; derived automatic paths can have several corners. Avoids board objects, not crossings with other arrows. Capacity or overlapping-terminal failures show an unavailable-route warning.
