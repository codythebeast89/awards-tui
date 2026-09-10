# Quickstart: GUI Edit

Manual validation scenarios, run against a real window (same display constraint as every prior
`awards-gui` feature's quickstart).

## Prerequisites

- A working `awards-gui` setup, signed in (Edit's Save is gated on sign-in, same as Add's).
- A test user with at least one award.

## Scenario 1 — Edit prefills and saves a correction

1. Look up the test user; click "Edit" next to one of their awards.
2. **Expected**: an inline field opens prefilled with that award's current text.
3. Change the text (e.g. append " x2") and click Save.
4. **Expected**: the field closes and the results view shows the corrected text immediately, with
   no manual Refresh (spec Acceptance Scenario 2, FR-005).

## Scenario 2 — Cancel discards nothing

1. Click Edit on an award, change the text, click Cancel.
2. **Expected**: the field closes, the award's text is unchanged in the results view (spec
   Acceptance Scenario 3).

## Scenario 3 — Signed out blocks Save

1. Sign out (or use a fresh profile with no token) and open Edit on an award.
2. **Expected**: Save is disabled and a "Sign in to edit awards" hint is shown (spec Acceptance
   Scenario 4, FR-002).

## Scenario 4 — A refused write keeps your typed text

1. Open Edit on an award, change the text, and (if reproducible) trigger a stale-write refusal —
   for example by editing the same cell from a second window/session first, then saving here.
2. **Expected**: the edit field stays open with your typed text intact and the refusal message is
   shown (spec Acceptance Scenario 5, FR-004).

## Scenario 5 — Reassigning a cell's username

1. Open Edit on an award and change its leading username to a different valid username not
   already using that award column, then Save.
2. **Expected**: the award disappears from the currently-viewed user's results, and the status
   line reads "... no longer under @<original username>" (spec Acceptance Scenario 6, FR-006).

## Scenario 6 — Only one write flow at a time

1. Open Add (via "Add Award"), then click Edit on an existing award without closing Add first.
2. **Expected**: the Add picker closes and the Edit panel opens instead (spec Edge Case, FR-007).
   Reverse the order (open Edit, then click "Add Award") and confirm the same holds in reverse.

## Regression check — no behavior changed elsewhere

1. Run `cargo test --workspace --locked` and `cargo clippy --workspace --all-targets --locked --
   -D warnings` — every existing test (004/005/006's own) passes unchanged.
2. Re-run 004-gui-lookup-add's and 005-gui-refresh's own quickstart scenarios once, to confirm
   Lookup, Refresh, and Add still behave exactly as before.
