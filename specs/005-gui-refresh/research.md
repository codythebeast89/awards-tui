# Research: GUI Refresh

Phase 0 output for `specs/005-gui-refresh/plan.md`. One decision — everything else this feature
needs was already decided (and shipped) by `004-gui-lookup-add`.

## 1. How Refresh triggers the sync

**Decision**: Refresh calls the existing `GuiApp::start_sync(&mut self)` directly — the same
method `GuiApp::new` already calls once at construction time. No new function, no new message
variant.

**Rationale**: `start_sync()` already does exactly what a refresh needs: sets `syncing = true`,
sets a "Syncing..." status, spawns a background thread that calls `build_awards_data(None)`, and
sends the result back as `GuiMsg::SyncDone` — which `handle_msg` already routes to
`handle_sync_done`, replacing `self.data` on success or setting `self.sync_error` on failure. Every
place that already checks `app.syncing` to disable Lookup and Add (spec FR-003) is unchanged code
that automatically covers a refresh too, since it doesn't distinguish "first sync" from "a later
one" — spec FR-003 falls out of the existing implementation for free.

**Alternatives considered**:
- A separate `GuiApp::refresh()` method that duplicates `start_sync()`'s body — rejected: it would
  be the exact same four lines, and two copies of "spawn a thread that calls `build_awards_data`"
  is exactly the kind of duplicated business logic the constitution's Code Quality principle and
  `004-gui-lookup-add`'s own FR-009 precedent argue against generalizing away from a single
  source of truth.
- A `GuiMsg::RefreshDone` variant distinct from `SyncDone` — rejected: nothing downstream needs to
  tell "the first sync" and "a refresh" apart; they update the same fields the same way.

## 2. Preventing a second refresh from starting mid-flight (spec FR-004)

**Decision**: `start_sync()` itself gets one guard clause at the top — `if self.syncing { return;
}` — rather than relying solely on the button's `add_enabled(!app.syncing, ...)` state in `ui.rs`.

**Rationale**: Putting the guard in `start_sync()` protects the invariant regardless of the call
site — today that's one button, but a guard that only lived in `ui.rs` would silently stop
protecting the invariant the moment any other code path ever called `start_sync()` directly (a
future keyboard shortcut, for instance). This mirrors why `confirm_add`'s equivalent guard
(`can_confirm_add()`) already lives on `GuiApp` rather than only in the button's `add_enabled`
call in `004-gui-lookup-add`.

**Alternatives considered**: Rely on the disabled button alone — rejected as fragile for the
reason above; egui's `add_enabled` already prevents a *click* while disabled, but that's a UI-layer
guarantee, not a state-layer one, and the constitution's Testing Standards principle expects the
state layer to be independently correct and testable without a window.
