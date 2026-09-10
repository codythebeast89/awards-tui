# Contract: TUI Audit Browser Interaction

This feature's external interface is the interactive Audit browser inside `awards-tui`'s TUI —
not a new CLI surface (see `research.md` §5 for why the CLI is unchanged). This document is the
interaction contract `crates/awards-tui/src/tui/app.rs` and `ui.rs` must satisfy, in the same role
`contracts/cli-interface.md` played for feature 001's CLI surface.

## Entry point

`Action::Audit` (existing keybinding, existing worker) is unchanged: it runs
`collect_sheet_audit` against the currently-loaded `AwardsData` and opens the Audit modal. What
changes is what the modal opens into.

## View: Findings List (`AuditView::List`) — new default

| Input | Behavior |
|---|---|
| `↑` / `k`, `↓` / `j` | Move selection between findings; grouped by member, groups stay visually distinct (e.g. a header row per `@username`). |
| `PgUp` / `PgDn` | Jump by page, matching the existing Report view's paging. |
| `Enter` | Act on the selected finding (see per-kind behavior below). No-op with a hint if the list is empty. |
| `Tab` (or a dedicated key, e.g. `r`) | Switch to `AuditView::Report` — the existing read-only text view — without closing the modal. |
| `Esc` | Close the Audit modal entirely, **without** overwriting `self.status` — this is the constitution's existing documented exception for the Audit browser (Principle III) and MUST continue to apply from either view. |

**Per-finding-kind `Enter` behavior** (see `data-model.md` for the full state machine):

- **Duplicate row**: closes the Audit modal, opens the existing typed-confirmation Delete modal
  pre-filled with that row's `Award` — identical to selecting that award in the Awards pane and
  pressing `d`.
- **Malformed cell**: closes the Audit modal, opens the existing Edit modal pre-filled with that
  cell's `Award` and current cell text — identical to pressing `e` on that award.
- **Similar usernames**: prompts the clerk to pick which of the two usernames is being corrected
  (a small two-option choice, not a full modal — e.g. inline in the finding row), then opens the
  existing Rename modal with that username as `from`.
- **Unparseable cell**: `Enter` is a no-op; the row already displays the cell's sheet/column/row
  for manual review (there is no username to build a fix action from — FR-009).

## View: Report (`AuditView::Report`) — existing behavior, preserved

Unchanged from before this feature: scrollable read-only plain text
(`↑`/`↓`/`j`/`k`/`PgUp`/`PgDn`/`Home`/`End` to scroll, `Esc` to close without overwriting status),
still backed by `format_audit_report` and still saved to `audits/audit-<timestamp>.txt` on every
audit run (spec FR-005 / User Story 2). Reachable from the List view via the same toggle key.

## Post-fix behavior

After a fix opened from a selected finding completes successfully (`WorkerMsg::WriteDone`
success), the modal returns to `AuditView::List` with findings recomputed locally (no network
call — `research.md` §3) so the resolved finding is gone (FR-006) and any other findings for that
same member remain visible (spec User Story 3, Acceptance Scenario 2). On a stale-write failure
(`EditError::Stale` — `research.md` §4) or any other write failure, the clerk sees the same error
message the Edit/Delete/Rename modals already surface today; the findings list is left as-is
(not silently dropped) so the clerk can retry or pick a different finding.

## Out of scope for this contract

No new CLI flags, no new network endpoints, no new persisted file format — `audits/*.txt`'s
content and location are unchanged (spec FR-005).
