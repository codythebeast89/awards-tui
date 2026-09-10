# Quickstart: GUI Refresh

Manual validation scenarios for the Refresh control, to be run against a real window (same
constraint as `004-gui-lookup-add`'s quickstart.md — this environment cannot open a display).

## Prerequisites

- A working `awards-gui` setup (same as `004-gui-lookup-add`'s quickstart.md Prerequisites).
- A second way to write to the Decorations Database while the GUI is open and already synced —
  either the terminal tool (`awards-tui`) or a second copy of the GUI, both pointed at the same
  sheet.

## Scenario 1 — Refresh picks up a change made elsewhere

1. Launch `awards-gui` and wait for the initial sync to finish.
2. Using the terminal tool (or a second window), add an award to a test user who did not already
   have it.
3. In the already-open GUI window, click Refresh and wait for "Syncing..." to clear.
4. Look up that test user.
5. **Expected**: the award added in step 2 appears, even though the GUI window was never closed
   (spec Acceptance Scenario 1, SC-001).

## Scenario 2 — Lookup and Add are blocked during a refresh

1. With the application open, click Refresh.
2. While "Syncing..." is showing, try to submit a lookup and try to click Add Award.
3. **Expected**: both are unavailable, exactly as they already are during the very first sync
   after launch — no crash, no stale-data lookup result (spec Acceptance Scenario 2).

## Scenario 3 — A second Refresh click does nothing mid-flight

1. Click Refresh, then immediately click it again before "Syncing..." clears.
2. **Expected**: only one sync runs — the second click has no effect (spec Acceptance Scenario 3).

## Scenario 4 — Recovering from a failed startup sync

1. Launch `awards-gui` with no network access (or an unreachable sheet), so the initial sync
   fails.
2. Restore network access.
3. Click Refresh.
4. **Expected**: the sync succeeds this time, with no need to quit and relaunch the application
   (spec Acceptance Scenario 4, SC-002).

## Regression check

1. Run `cargo test --workspace --locked` and `cargo clippy --workspace --all-targets --locked --
   -D warnings` — all `004-gui-lookup-add` tests and behavior remain unchanged; this feature adds
   one new test and edits no existing test's expected behavior.
2. Re-run `004-gui-lookup-add`'s own quickstart.md Scenarios 1 and 2 (Lookup) to confirm Refresh
   introduced no regression to Lookup itself.
