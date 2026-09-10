# Feature Specification: Audit Fix Selection

**Feature Branch**: `002-audit-fix-selection`

**Created**: 2026-09-10

**Status**: Draft

**Input**: User description: "Configure the Internal Audit command in awards-tui so an end user can select a person/finding listed in the audit results and go directly into fixing it, rather than only viewing a read-only text report. Today `collect_sheet_audit` builds a structured AuditReport (duplicate groups keyed by user, similar-username pairs, malformed cells, unparsed cells), and the audit action flattens it into plain text shown in a scroll-only, read-only view with no selection. The audit action should instead present its flagged entries as a navigable, selectable list — grouped by the person involved — and let the clerk pick a specific finding and jump straight into the existing fix flow for that award/cell (Edit to correct a malformed/conflicting cell, Delete/remove a duplicate row, or Rename), reusing the app's existing OAuth-gated live-sheet write path rather than introducing a new one. The existing read-only text report and its saved-file export should remain available, since it's used for record-keeping. Selecting a finding and applying a fix should re-run or refresh the audit so resolved findings drop off the list."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Fix a flagged issue directly from the audit (Priority: P1)

A Logistics Clerk runs the Internal Audit and sees a list of specific issues (duplicate award entries, look-alike usernames, malformed cells) instead of a wall of static text. The clerk picks one issue involving a specific member and is taken straight into the matching fix action — editing the cell, removing the duplicate row, or renaming the username — with the details already filled in from the finding, rather than having to memorize the member's name and award column and re-enter them by hand elsewhere in the app.

**Why this priority**: This is the entire point of the feature — turning the audit from a report the clerk has to act on manually into a worklist they can clear directly. Without this, the rest of the feature has no value.

**Independent Test**: Run the audit against data with at least one duplicate-award finding for a known member; select that finding; confirm the clerk lands on the correct fix action pre-filled with that member's award and cell, completes the fix, and the underlying data changes accordingly.

**Acceptance Scenarios**:

1. **Given** the audit has found a duplicate award entry for a member, **When** the clerk selects that finding, **Then** the clerk is taken into the remove/delete flow for that specific duplicate row, with the member and award already identified.
2. **Given** the audit has found a malformed award cell for a member, **When** the clerk selects that finding, **Then** the clerk is taken into the edit flow for that specific cell, with the member and award already identified.
3. **Given** the audit has found two look-alike usernames for the same award, **When** the clerk selects that finding, **Then** the clerk is taken into the rename flow with one of the two usernames pre-filled as the name being corrected.

---

### User Story 2 - Keep the record-keeping report available (Priority: P2)

A Logistics Clerk (or their supervisor) still wants the complete audit findings as a plain-text report they can save, forward, or file for later reference, the same way the audit already works today.

**Why this priority**: This is existing, relied-upon behavior. The new selectable worklist is additive — it must not take away the record-keeping export that record-keeping and hand-off between clerks currently depends on.

**Independent Test**: Run the audit and confirm the same plain-text report is still produced and saved to a file, with the same content and structure as before this feature, independent of whether the clerk ever uses the new selectable list.

**Acceptance Scenarios**:

1. **Given** the clerk runs the audit, **When** they choose to view the full report instead of the selectable list, **Then** they see the same complete, plain-text findings report the audit has always produced, and it is still saved to a file for later reference.

---

### User Story 3 - See progress as issues get resolved (Priority: P3)

A Logistics Clerk working through a list of flagged issues wants the list to reflect what they've already fixed, so they don't waste time re-checking or re-fixing something they just handled, and can tell at a glance how much is left.

**Why this priority**: Valuable for clerks clearing several issues in one sitting, but the feature is still useful without it — a clerk could always re-run the audit manually. Lower priority than the core select-and-fix flow.

**Independent Test**: Fix one finding from the selectable list, then confirm that finding is no longer present without the clerk manually re-triggering the audit from scratch.

**Acceptance Scenarios**:

1. **Given** the clerk has fixed a flagged issue from the selectable list, **When** they return to the list, **Then** that specific issue is no longer shown.
2. **Given** the clerk fixes one of several issues affecting the same member, **When** they return to the list, **Then** that member's remaining, still-unresolved issues are still shown.

### Edge Cases

- What happens when the clerk selects a finding for a member whose flagged award was already changed or removed by someone else since the audit was generated? The clerk must be told the data has moved on, the same way the existing edit/delete/rename actions already detect and report a stale write target — not left to apply a fix against data that no longer matches what the audit showed them.
- What happens when a clerk selects a "look-alike usernames" finding — which of the two usernames is treated as the mistake being corrected? The clerk is shown both usernames and can choose which one is being renamed to match the other, rather than the system guessing.
- How does the system handle a cell the audit could not even extract a username from? Since there is no reliable member to attach an automatic fix to, the clerk is shown exactly where that cell lives (award, sheet, row) for manual review rather than being offered a fix action that might apply to the wrong person.
- What happens if the clerk backs out of a fix partway through (cancels the edit/delete/rename)? They return to the selectable findings list with nothing changed, rather than being dropped back to the audit's starting point.
- What happens when there are a large number of findings? The clerk can still tell at a glance which findings belong to which member, rather than having to scroll through an undifferentiated flat list.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST present audit findings as a list of individual, selectable issues — not only as a single block of read-only text — where each entry identifies the member, the award, and the specific problem (duplicate entry, look-alike username, malformed cell, or unparseable cell).
- **FR-002**: The system MUST group findings by the member they involve, so a clerk can see everything flagged for one person together.
- **FR-003**: The system MUST let the clerk select a single finding and be taken directly into the correct fix action for it: removing a duplicate row, editing a malformed or conflicting cell, or renaming a look-alike username.
- **FR-004**: When a clerk is taken into a fix action from a selected finding, the system MUST pre-fill that action with the member, award, and cell identified by the finding, rather than requiring the clerk to re-enter them.
- **FR-005**: The system MUST continue to offer the complete audit findings as a saved, plain-text report, unchanged in content from the audit's current behavior, for clerks or supervisors who need it for record-keeping.
- **FR-006**: After a clerk applies a fix from the selectable list, the system MUST update the list so the resolved finding no longer appears, without the clerk having to separately re-run the audit from the beginning.
- **FR-007**: If the data behind a selected finding no longer matches what the audit found (because it changed after the audit ran), the system MUST inform the clerk instead of applying a fix against outdated information.
- **FR-008**: Fixing an issue from the audit view MUST use the same sign-in and write permissions the system already requires for editing, deleting, or renaming awards elsewhere in the app — this feature MUST NOT create a separate or lesser-guarded way to write to the Decorations Database.
- **FR-009**: For a finding where no member could be identified (an unparseable cell), the system MUST still show the finding's exact location (award and cell) so it can be reviewed manually, rather than presenting a fix action it cannot reliably target.
- **FR-010**: Selecting a "look-alike usernames" finding MUST show the clerk both usernames involved and let them choose which one is being corrected before proceeding into the rename action.

### Key Entities

- **Audit Finding**: A single flagged issue produced by the audit — one of: duplicate award entry, look-alike username pair, malformed cell, or unparseable cell — carrying the member, award, and sheet/column/row location it concerns. Today these exist only as data used to render a static report; this feature makes each one individually selectable.
- **Audit Findings List**: The clerk-facing, navigable presentation of all current Audit Findings, grouped by member, that replaces the audit's plain scroll-only text view as the primary way clerks interact with audit results (the plain-text report remains available separately).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A clerk can go from seeing a flagged issue to having the correct fix action open, fully pre-filled, in a single selection — with no manual re-typing of the member's name or award.
- **SC-002**: A finding a clerk has just fixed no longer appears in the findings list without the clerk taking any separate action to refresh or re-run the audit.
- **SC-003**: The saved, plain-text audit report produced today remains available with unchanged content and format, so existing record-keeping and hand-off practices built around it continue to work without modification.
- **SC-004**: A clerk without write permission can still see every flagged finding, and attempting to fix one surfaces the same permission message the app already gives for edit/delete/rename — no new or confusing failure behavior is introduced.

## Assumptions

- The clerk fixing an issue from the audit view already holds (or is prompted for) the same Google sign-in already required by the app's existing edit, delete, and rename actions; this feature does not change who is allowed to write to the Decorations Database, only how a clerk gets to that action.
- The saved plain-text report remains the audit's system-of-record output for anyone who needs to file or share the full results; the new selectable list is a faster way to act on those same findings, not a replacement source of truth.
- A "look-alike usernames" finding is resolved through the existing rename capability, applied to whichever of the two usernames the clerk identifies as the mistake.
- After a fix is applied, the findings list is refreshed from a new audit pass rather than the system trying to guess which other findings are still valid — consistent with how the rest of the app already re-syncs after a write.
