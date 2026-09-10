---

description: "Feature specification template"
---

# Feature Specification: GUI Edit

**Feature Branch**: `007-gui-edit`
**Created**: 2026-09-10
**Status**: Draft
**Input**: "Add an Edit capability to the awards-gui desktop application, letting a clerk correct
the raw cell text of an award already shown in a looked-up user's results — the same underlying
write the terminal tool's own Edit modal already performs (`update_award_cell`), reached from a
per-award control in the results view. No new confirmation gate beyond Add's existing sign-in
gate, since this corrects an existing entry rather than destroying one. This is the 'Edit'
capability the clerk selected as the next GUI milestone after 006-gui-visual-theme."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Correct a malformed or outdated award entry (Priority: P1)

A clerk looks up a Roblox user, notices one of their awards has the wrong suffix/qualifier text
(for example a typo'd repeat count, or a name that should read differently after the underlying
catalog display format changed), and corrects just that cell's text without deleting and re-adding
the award.

**Why this priority**: This is the entire feature — the one capability being added.

**Independent Test**: Look up a user with at least one award, click that award's Edit control,
change the text, save, and confirm the results view shows the corrected text without a full page
reload or re-sync.

**Acceptance Scenarios**:

1. **Given** a looked-up user with an award showing "torba_f", **When** the clerk clicks that
   award's Edit control, **Then** an inline edit field opens prefilled with the award's current
   cell text ("torba_f").
2. **Given** the edit field is open and the clerk changes the text to "torba_f x2" and clicks
   Save, **When** the write succeeds, **Then** the results view immediately shows the updated
   text and the edit field closes.
3. **Given** the edit field is open, **When** the clerk clicks Cancel (or clicks Edit on a
   different award), **Then** nothing is written and the field closes (or switches to the newly
   selected award) with no side effect on the untouched entry.
4. **Given** the clerk is not signed in, **When** they open the edit field, **Then** Save is
   disabled and a "Sign in to edit awards" hint is shown, matching Add's existing sign-in gate.
5. **Given** the clerk submits an edit whose target cell changed on the sheet since this user was
   last looked up (a stale write), **When** the write is refused, **Then** the edit field stays
   open with the clerk's typed text intact and the refusal message is shown, so nothing is lost
   and no retry requires re-typing.
6. **Given** the clerk edits a cell's leading username to a *different* person's username,
   **When** the write succeeds, **Then** the award no longer appears in the currently-viewed
   user's results (it now belongs to whoever the new cell names) and the status line notes it is
   "no longer under @<original-username>".

### Edge Cases

- Submitting an empty (or whitespace-only) value is refused before any network call, matching the
  underlying `update_award_cell`'s own validation ("Cell value cannot be empty — use delete
  instead").
- Submitting text that doesn't start with a recognizable username is refused the same way
  ("Cell must start with a username").
- Opening Edit on one award while the Add picker is already open closes the Add picker first (and
  vice versa) — only one write flow is open at a time, matching the terminal tool's single-modal
  convention.
- A fresh Look Up closes any in-progress Edit for the previous user, exactly as it already does
  for an in-progress Add.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The results view MUST offer a per-award control that opens an inline edit field
  prefilled with that award's current cell text (or its display name, if the cell is somehow
  empty).
- **FR-002**: Saving MUST be disabled unless the clerk is signed in and the edit field is
  non-empty and not already mid-submit, matching Add's existing `can_confirm_add` gating pattern.
- **FR-003**: Saving MUST call the exact same guarded write the terminal tool's own Edit action
  uses (`update_award_cell`, non-interactive auth) — no new or differently-guarded write path.
- **FR-004**: A refused write (stale cell, validation failure, or a conflicting live collision)
  MUST leave the edit field open with the clerk's typed text intact and MUST show the refusal
  message, never silently discard the in-progress edit.
- **FR-005**: A successful write MUST update the results view immediately from the in-memory
  index, without requiring a full Refresh.
- **FR-006**: If a successful edit reassigns the cell to a different username than the one
  currently being viewed, the award MUST disappear from the current results view and the status
  MUST note it moved, rather than continuing to show a now-inaccurate entry.
- **FR-007**: Only one write flow (Add or Edit) MUST be open at a time; opening one MUST close
  the other with no side effect on whichever was open.
- **FR-008**: Lookup, Refresh, and Add remain unaffected — Edit introduces no change to any of
  their existing behavior (004-gui-lookup-add, 005-gui-refresh).

## Assumptions

- Edit does not require a typed confirmation phrase (unlike Delete/Rename in the terminal tool):
  it corrects an existing entry rather than destroying data, so Constitution III's "destructive or
  hard to reverse" confirmation gate does not apply — this matches the terminal tool's own Edit
  modal, which submits on a single Enter.
- Delete, Rename, Assist, Audit, and Discord-paste quick-add remain out of scope, unchanged from
  004-gui-lookup-add/005-gui-refresh/006-gui-visual-theme.
- The visual theme (006-gui-visual-theme) applies to Edit's UI automatically through the shared
  `theme.rs` palette — no new color is introduced by this feature.
