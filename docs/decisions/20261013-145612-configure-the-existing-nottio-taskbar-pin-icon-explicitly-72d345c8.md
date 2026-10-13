---
type: Decision
title: "Configure the existing nottio taskbar pin icon explicitly"
decision_status: adopted
decision_kind: process
recorded_by: ai
review_status: unreviewed
adoption_mode: auto
decided_at: 2026-10-13T14:56:12Z
impact: normal
source_refs:
  - "conversation:current-user-taskbar-icon-request"
supersedes: []
---

# Configure the existing nottio taskbar pin icon explicitly

## Decision

Set the existing nottio taskbar shortcut icon to the generated committed ICO, with a backup and a targeted shell notification. Verify the shortcut targets this release executable and preserve launch target, arguments, workdir and application grouping. Do not clear the global icon cache, remove pins or restart Explorer.

## Rationale

User requested taskbar icon configuration after executable/window icon setup. The existing pin had an empty icon filename while the resource/window icon was valid; changing only that pin is the narrow fix.

## Alternatives

Not recorded.

## Consequences

Not recorded.
