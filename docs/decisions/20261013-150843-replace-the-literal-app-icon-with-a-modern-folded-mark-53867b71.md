---
type: Decision
title: "Replace the literal app icon with a modern folded mark"
decision_status: adopted
decision_kind: product
recorded_by: ai
review_status: unreviewed
adoption_mode: auto
decided_at: 2026-10-13T15:08:43Z
impact: normal
source_refs:
  - "conversation:current-user-modern-icon-request"
  - "docs/modern-app-icon-research.md"
supersedes:
  - "docs/decisions/20261013-144748-generate-and-install-the-nottio-windows-app-icon-06ccabd3.md"
---

# Replace the literal app icon with a modern folded mark

## Decision

Replace the yellow sticky/arrow illustration with an original porcelain and blue-violet folded-ribbon mark, after comparing official Linear, Raycast, Craft and Obsidian assets and Microsoft Windows guidance. Keep one large coherent silhouette and restrained color/depth. Update both executable resource and the existing taskbar pin; use a content-addressed copy of the ICO to avoid retaining the previous image under a cached filename.

## Rationale

User rejected the previous icon as insufficiently modern and explicitly requested research and replacement. Reference brands are analyzed, not copied; small-size readability and clean transparency are checked.

## Alternatives

Not recorded.

## Consequences

Asset generation preserves a fixed source crop and analytic tile mask; 10 ICO resolutions remain. No global cache purge, application-grouping change, or board-data migration.
