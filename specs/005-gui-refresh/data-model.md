# Data Model: GUI Refresh

Phase 1 output for `specs/005-gui-refresh/plan.md`. No new types and no new fields — this feature
is additive at the behavior level only, reusing every type `004-gui-lookup-add`'s data-model.md
already defined.

## Reused, unchanged

- `GuiApp.syncing: bool` — already the single flag every gate (Lookup, Add) checks; a refresh sets
  it the same way the startup sync already does.
- `GuiApp.sync_error: Option<String>` — already populated on a failed sync (including, after this
  feature, a failed refresh) and already surfaced via the status bar (`004`'s converge finding F2
  made `submit_lookup`'s message distinguish a permanent failure from "still syncing," which this
  feature's Edge Case — "refresh fails, keep the last-known-good data" — depends on: `data` is
  only ever replaced by a *successful* `SyncDone`, so a failed refresh leaves the previous
  `AwardsData` in place untouched).
- `GuiApp::start_sync(&mut self)` — reused as the refresh trigger itself (research.md §1), with one
  added guard clause (research.md §2).
- `GuiMsg::SyncDone(Result<AwardsData, String>)` and `GuiApp::handle_sync_done` — reused unchanged;
  a refresh's result is indistinguishable from the startup sync's, by design.

## State Transitions

```text
Idle (auth known, data present or absent, syncing == false)
  clerk clicks Refresh
    if syncing == true: no-op (spec FR-004) — the button is disabled anyway, and start_sync()
                         itself refuses to start a second fetch even if called directly
    else: start_sync() — syncing = true, status = "Syncing...", spawns fetch thread
  → SyncDone(Ok(data)): data = Some(data) (replaces the previous value), syncing = false,
                         sync_error = None — new awards added elsewhere are now visible to
                         the next Lookup (spec FR-005)
  → SyncDone(Err(e)):   sync_error = Some(e), syncing = false, data UNCHANGED — a failed
                         refresh never blanks out previously-synced data (spec FR-006)
```

No new entity, no new enum, no new struct field.
