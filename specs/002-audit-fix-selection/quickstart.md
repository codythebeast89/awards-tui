# Quickstart: Audit Fix Selection

Validation walkthrough for `specs/002-audit-fix-selection`. See `data-model.md` for the types and
`contracts/tui-audit-interaction.md` for the full keybinding contract these steps exercise.

## Prerequisites

- Built `awards-tui` binary (`cargo build -p awards-tui`).
- Read access to the QMC Decorations Database is enough for User Stories 1 and 3 below if using
  fixture/offline data; applying an actual fix against the live sheet additionally needs a signed-in
  Logistics Clerk account, exactly as the existing Edit/Delete/Rename actions already require.
- At least one known duplicate, malformed, or similar-username condition present in the data under
  test (seed one deliberately if validating against a clean sheet — see
  `crates/awards-core/tests/offline.rs` fixtures for examples already used by the audit's own unit
  tests).

## Offline validation (no live writes — covers `data-model.md`'s pure logic)

1. `cargo test -p awards-core` — confirms `flatten_audit_findings` and `AuditFinding::to_award`
   unit tests pass (added by this feature; see `tasks.md`), without touching the network.
2. `cargo test -p awards-tui` — confirms the new/changed modal-transition tests for the Audit
   browser's List/Report toggle and per-finding-kind selection pass (constitution Testing
   Standards principle).

## Interactive walkthrough (User Story 1 — P1)

1. Launch `awards-tui`, let the initial sync complete.
2. Press the Audit action. **Expected**: the modal opens directly into the findings list
   (`AuditView::List`), not a wall of text — grouped by member.
3. Navigate to a known duplicate-row finding for a specific member and press `Enter`. **Expected**:
   the modal closes and the Delete confirmation modal opens, pre-filled with that member's award
   and the duplicate row — no re-typing the member's name.
4. Confirm the delete (typed `delete` confirmation, existing behavior, unchanged). **Expected**:
   the write succeeds, the app returns to the findings list, and that specific finding is gone
   (User Story 3 / FR-006) — without manually re-running the audit.
5. Repeat step 3 with a malformed-cell finding, landing in the Edit modal instead, and with a
   similar-usernames finding, confirming the two-username choice appears before Rename opens
   (FR-010).

## Record-keeping check (User Story 2 — P2)

1. From the findings list, switch to `AuditView::Report`. **Expected**: the same complete
   plain-text report the audit has always produced, byte-identical in structure to before this
   feature.
2. Confirm `audits/audit-<timestamp>.txt` was still written on this audit run, unchanged in format.

## Stale-write check (Edge Case / FR-007)

1. Open a fix action from a selected finding.
2. Before confirming, have another session (or a manual sheet edit) change that same cell.
3. Confirm the fix. **Expected**: the existing "changed on the sheet... refresh and try again"
   stale-write message appears (same wording `cell_stale_message` already produces elsewhere),
   not a silent overwrite.

## Permission check (SC-004)

1. Run the app without valid write credentials (no `token.json` / service account, or via
   `--auth-status` to confirm signed-out state — reuse the existing check, do not add a new one).
2. Confirm every finding is still visible in the list.
3. Attempt to fix one. **Expected**: the same permission-denied message the Edit/Delete/Rename
   actions already give when unauthenticated — no new or different failure mode.
