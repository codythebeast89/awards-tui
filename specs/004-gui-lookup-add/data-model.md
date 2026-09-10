# Data Model: GUI Lookup & Add

Phase 1 output for `specs/004-gui-lookup-add/plan.md`. Every type below lives in the new
`crates/awards-gui/src/app.rs` (the windowing-free state module, research.md §7) unless noted as
reused from an existing crate. No new fields are added to any existing `awards-core`/
`awards-sheets` type — this feature is additive-only at the type level.

## `GuiApp` (state struct)

The application's entire state, owned by the `eframe::App` implementation in `main.rs` and driven
by `ui.rs`'s rendering code, but touchable and testable with no `egui`/`eframe` window involved.

```rust
pub struct GuiApp {
    tx: mpsc::Sender<GuiMsg>,
    rx: mpsc::Receiver<GuiMsg>,
    ctx: egui::Context,              // cloned into background threads for request_repaint (research.md §3)

    // Sheet data (research.md §6)
    pub data: Option<AwardsData>,     // None until the first sync completes
    pub syncing: bool,
    pub sync_error: Option<String>,

    // Lookup (spec User Story 1)
    pub username_input: String,       // the text field's live contents
    pub looked_up: Option<LookedUpUser>,

    // Sign-in (research.md §4)
    pub auth: AuthState,

    // Add (spec User Story 2) — Some only while the add picker is open
    pub add_picker: Option<AddPicker>,

    pub status: String,               // one-line status/result message, shown at the bottom of the window
}
```

`GuiApp::new(ctx: egui::Context) -> Self` creates the channel, stores the context clone, sets
`auth` from an initial `auth_status()` call, and immediately kicks off a sync (mirroring
`awards-tui`'s `App::new` + `start_sync` pair).

## `GuiMsg` (background-thread → UI channel messages)

The `awards-gui` equivalent of `awards-tui`'s `WorkerMsg` (research.md §3) — everything a
background thread can report back.

```rust
pub enum GuiMsg {
    SyncDone(Result<AwardsData, String>),
    LoginDone(Result<String, String>),   // login()'s Ok(message) or Err(AuthError).to_string()
    AddDone {
        username: String,
        result: EditResult,               // awards_sheets::edit::EditResult, reused unchanged
    },
}
```

`GuiApp::handle_msg(&mut self, msg: GuiMsg)` is the single dispatch point, called once per drained
message at the top of `eframe::App::update` — mirrors `awards-tui`'s `handle_worker_msg`.

## `LookedUpUser`

The read-only view produced by a successful lookup (spec Key Entity: "Looked-Up User View").

```rust
pub struct LookedUpUser {
    pub username: String,
    pub awards: Vec<Award>,     // awards_core::Award, reused unchanged; already carries `category`
}
```

Built from the existing `AwardsData.index` lookup (`get_awards_for_username`, research.md §6) — no
new fields, no new shape; `awards` is grouped by `category` at render time in `ui.rs`, not stored
pre-grouped, so there is exactly one code path for "what does this user currently have," matching
how `awards-tui`'s own `results`/`duplicates` are consumed.

A lookup that finds no records at all is represented as `looked_up = Some(LookedUpUser { username,
awards: vec![] })` plus a `not_found: bool` flag alongside it (spec FR-008) — kept as a third field
on `LookedUpUser` rather than `Option<LookedUpUser>` being `None`, so "searched and found nothing"
stays visually and logically distinct from "haven't searched yet":

```rust
pub struct LookedUpUser {
    pub username: String,
    pub awards: Vec<Award>,
    pub not_found: bool,
}
```

## `AuthState`

```rust
pub enum AuthState {
    Unknown,      // before the first auth_status() check completes at startup
    SignedOut,     // auth_status() == "oauth_needs_login" or "missing"
    SignedIn,      // auth_status() == "oauth_token" or "service_account"
    SigningIn,     // login() is running on a background thread; Sign In button shows a spinner, disabled
}
```

`GuiApp::refresh_auth_state(&mut self)` calls `awards_sheets::auth_status()` (unchanged) and maps
its four string values into this enum — the mapping is the only place a raw `auth_status()` string
is interpreted, so a future new status value has exactly one call site to update.

## `AddPicker`

The in-progress add flow (spec Key Entity: "Award Catalog Selection") — `Some` only while open,
mirroring how `awards-tui`'s `Modal::Add` only exists while that modal is open.

```rust
pub struct AddPicker {
    pub candidates: Vec<AwardDef>,   // awards_core::AwardDef, reused unchanged — owned awards already excluded
    pub filter: String,               // live search box contents
    pub filtered: Vec<AwardDef>,      // candidates matching filter, via match_catalog_entries (research.md §6)
    pub selected: Option<AwardDef>,   // chosen from `filtered`; confirming requires this to be Some (spec FR-004)
    pub suffix: String,               // optional trailing detail/count, same free-text convention the TUI uses
    pub submitting: bool,              // true while add_award_to_user is running on a background thread
}
```

`candidates` is computed once when the picker opens (`data.catalog` minus `owned_award_columns` for
the looked-up user, research.md §6) and does not change while the picker is open; only `filtered`
recomputes as `filter` changes, via `match_catalog_entries(&candidates, &filter)`.

## State Transitions

```text
Startup
  → GuiApp::new: auth = Unknown, syncing = true (sync kicked off), status = "Syncing..."
  → SyncDone(Ok(data)): data = Some(data), syncing = false
  → SyncDone(Err(e)): sync_error = Some(e), syncing = false
  → refresh_auth_state(): Unknown → SignedIn | SignedOut

Lookup (spec User Story 1)
  submit_lookup(): reads username_input, looks up in data.index (local, no network)
  → found: looked_up = Some(LookedUpUser { username, awards, not_found: false })
  → not found: looked_up = Some(LookedUpUser { username, awards: [], not_found: true })

Sign-in (research.md §4) — only reachable when auth != SignedIn
  click "Sign In": auth = SigningIn, spawn thread calling login()
  → LoginDone(Ok(msg)): status = msg, refresh_auth_state() (→ SignedIn if login() actually succeeded)
  → LoginDone(Err(msg)): status = msg, refresh_auth_state() (stays SignedOut — login() didn't change auth_status())

Add (spec User Story 2) — the picker can only be opened when looked_up is Some(_) with awards
                           available to add, and closing it (without confirming) simply drops it
  open_add_picker(): candidates computed once; add_picker = Some(AddPicker { .. })
  update_filter(text): add_picker.filter = text; add_picker.filtered = match_catalog_entries(..)
  select(def): add_picker.selected = Some(def)
  confirm_add() — only enabled when auth == SignedIn && add_picker.selected.is_some():
    add_picker.submitting = true, spawn thread calling
    add_award_to_user(username, &selected, &suffix, interactive_auth: false)
  → AddDone { result, .. } where result.ok:
      looked_up.awards.push(result.award.unwrap()); add_picker = None; status = result.message
  → AddDone { result, .. } where !result.ok:
      add_picker.submitting = false; status = result.message (picker stays open — spec Acceptance
      Scenario 2.4: a stale-cell conflict must not silently drop the clerk's in-progress selection)
```
