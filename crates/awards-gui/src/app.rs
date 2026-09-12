//! Windowing-free application state for `awards-gui` (mirrors `awards-tui/src/tui/app.rs`'s
//! split from its own `ui.rs`). Everything in this module is plain Rust — no `egui`/`eframe`
//! window is ever opened by it — so it is unit-testable in a headless environment (plan.md
//! Testing section; this build/test environment cannot open an X11/Wayland display at all).
//!
//! Scope for this first GUI milestone (spec 004-gui-lookup-add): Lookup (read-only) and Add
//! (a single guarded write), nothing else — see spec.md FR-010 for what is deliberately absent.

use awards_core::{
    get_awards_for_username, match_catalog_entries, owned_award_columns, upsert_award_in_index,
};
use awards_core::{Award, AwardDef, AwardsData};
use awards_sheets::{
    account_label, add_award_to_user, auth_status, build_awards_data, can_sign_out, login,
    logout, update_award_cell, EditError, EditResult,
};
use std::sync::mpsc;
use std::thread;

/// The clerk's current relationship to Google sign-in. Lookup works in every state; only Add is
/// gated on `SignedIn` (spec FR-006, FR-007; research.md §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthState {
    /// Before the first `auth_status()` check completes at startup.
    Unknown,
    /// `auth_status()` reported `"oauth_needs_login"` or `"missing"`.
    SignedOut,
    /// `auth_status()` reported `"oauth_token"` or `"service_account"`.
    SignedIn,
    /// `login()` is running on a background thread — the Sign In control shows a spinner and is
    /// disabled while this holds.
    SigningIn,
}

/// The visual weight `ui.rs` gives the status line — critique follow-up (007-gui-edit polish):
/// every status update used to render as the same muted gray, so a refused write and a
/// successful one were visually identical. `Success`/`Error` are reserved for the outcome of a
/// clerk-deliberate action (Add, Edit, Sign In); routine/automatic updates (sync progress,
/// lookup results, guardrail nudges) stay `Info` so the accent and error colors keep the rarity
/// `DESIGN.md`'s Single-Accent Rule already commits to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Info,
    Success,
    Error,
}

/// Turns a write result into clerk-facing status text and its `StatusKind` — critique follow-up:
/// an `EditError::Api` failure (network/auth/HTTP) previously reached the clerk as a raw string
/// like `"Sheets API HTTP 401: {...}"`. That detail is still worth having for diagnosis, so it
/// goes to stderr, but the clerk now sees plain language and a next step instead. Every other
/// `EditError` variant's message passes through unchanged — `edit.rs`'s own doc comment commits
/// to those staying byte-identical for its other caller (the TUI), and they're already specific
/// and actionable (e.g. `cell_stale_message`).
fn describe_result(result: &EditResult) -> (String, StatusKind) {
    if result.ok {
        return (result.message.clone(), StatusKind::Success);
    }
    if let Some(EditError::Api(detail)) = &result.error {
        eprintln!("awards-gui: Sheets API call failed: {detail}");
        return (
            "Couldn't reach Google Sheets. Check your connection and try again.".to_string(),
            StatusKind::Error,
        );
    }
    (result.message.clone(), StatusKind::Error)
}

/// Everything a background thread can report back to the UI thread, the `awards-gui` equivalent
/// of `awards-tui`'s `WorkerMsg` (research.md §3). Every variant intentionally shares the `Done`
/// postfix (background op finished) rather than being renamed to satisfy clippy's
/// `enum_variant_names` lint, matching the established `WorkerMsg` naming convention this type
/// mirrors and the exact names already documented in data-model.md/research.md.
#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
pub enum GuiMsg {
    SyncDone(Result<AwardsData, String>),
    LoginDone(Result<String, String>),
    AddDone {
        username: String,
        result: EditResult,
    },
    EditDone {
        username: String,
        result: EditResult,
    },
}

/// The read-only view produced by a successful (or not-found) lookup (spec Key Entity:
/// "Looked-Up User View"). `not_found` is a third field rather than folding "no records" into
/// `awards: vec![]` alone, so "searched and found nothing" stays distinct from "haven't searched
/// yet" (`GuiApp.looked_up == None`) — spec FR-008.
#[derive(Debug, Clone)]
pub struct LookedUpUser {
    pub username: String,
    pub awards: Vec<Award>,
    pub not_found: bool,
}

/// The in-progress add flow (spec Key Entity: "Award Catalog Selection") — `Some` only while the
/// picker is open, mirroring how `awards-tui`'s `Modal::Add` only exists while that modal is
/// open.
#[derive(Debug, Clone)]
pub struct AddPicker {
    /// Computed once when the picker opens; does not change while it stays open.
    pub candidates: Vec<AwardDef>,
    pub filter: String,
    pub filtered: Vec<AwardDef>,
    pub selected: Option<AwardDef>,
    pub suffix: String,
    /// `true` while `add_award_to_user` is running on a background thread.
    pub submitting: bool,
}

impl AddPicker {
    fn new(candidates: Vec<AwardDef>) -> Self {
        let filtered = candidates.clone();
        Self {
            candidates,
            filter: String::new(),
            filtered,
            selected: None,
            suffix: String::new(),
            submitting: false,
        }
    }
}

/// The in-progress edit flow (spec 007-gui-edit) — `Some` only while a single award's cell is
/// being corrected, mirroring `AddPicker`'s "exists only while open" shape and `awards-tui`'s own
/// `Modal::Edit` (`EditModal { award, input }`, `crates/awards-tui/src/tui/app.rs`).
#[derive(Debug, Clone)]
pub struct EditFlow {
    /// The award this flow is correcting — fixed for the life of the flow; only `input` changes.
    pub award: Award,
    /// Pre-filled from `award.cell` (or `award.name` if `cell` is empty) when the flow opens,
    /// matching the TUI's own `action_edit` prefill (research.md §2) — then free-typed.
    pub input: String,
    /// `true` while `update_award_cell` is running on a background thread.
    pub submitting: bool,
}

/// The application's entire state. Owned by the `eframe::App` implementation in `main.rs` and
/// driven by `ui.rs`'s rendering code, but fully constructible and testable with no window.
pub struct GuiApp {
    tx: mpsc::Sender<GuiMsg>,
    rx: mpsc::Receiver<GuiMsg>,
    /// Cloned into background threads so they can call `request_repaint()` after sending a
    /// message — egui has no TUI-style fixed-tick poll loop, so nothing else wakes the UI up.
    repaint: Box<dyn Fn() + Send + Sync>,

    pub data: Option<AwardsData>,
    pub syncing: bool,
    pub sync_error: Option<String>,

    pub username_input: String,
    pub looked_up: Option<LookedUpUser>,

    pub auth: AuthState,
    /// Best-effort "who's signed in" label, refreshed alongside `auth`. `None` when signed out,
    /// or (rarely) when signed in but nothing on disk yields a readable identity.
    pub account_label: Option<String>,
    /// Whether the Sign Out control should render at all — `false` under a service account,
    /// which is a shared standing credential file, not a per-session login to end.
    pub can_sign_out: bool,

    pub add_picker: Option<AddPicker>,

    pub edit: Option<EditFlow>,

    /// One-line status/result message, shown at the bottom of the window.
    pub status: String,
    /// The visual weight `ui.rs` gives `status` — see `StatusKind`.
    pub status_kind: StatusKind,
}

impl GuiApp {
    /// `repaint` is called (from any thread) whenever a background operation completes, so the
    /// UI redraws promptly. In `main.rs` this is `move || ctx.request_repaint()` for a cloned
    /// `egui::Context`; tests pass a no-op closure since there is no window to repaint.
    pub fn new(repaint: impl Fn() + Send + Sync + 'static) -> Self {
        let (tx, rx) = mpsc::channel();
        let mut app = Self {
            tx,
            rx,
            repaint: Box::new(repaint),
            data: None,
            // Left false here (rather than true) so start_sync()'s own guard against a second
            // concurrent sync — added for the Refresh feature (005-gui-refresh) — doesn't
            // mistake this initial construction for a sync already in flight and skip it.
            syncing: false,
            sync_error: None,
            username_input: String::new(),
            looked_up: None,
            auth: AuthState::Unknown,
            account_label: None,
            can_sign_out: false,
            add_picker: None,
            edit: None,
            status: String::new(),
            status_kind: StatusKind::Info,
        };
        app.refresh_auth_state();
        app.start_sync();
        app
    }

    /// The only place `status`/`status_kind` are written together, so the two can never drift out
    /// of sync — a stale `Error`/`Success` kind surviving into an unrelated later `Info` message
    /// was exactly the bug this replaces (critique follow-up).
    fn set_status(&mut self, message: impl Into<String>, kind: StatusKind) {
        self.status = message.into();
        self.status_kind = kind;
    }

    /// Maps `awards_sheets::auth_status()`'s four raw string values into `AuthState` — the only
    /// place that string is interpreted, so a future new status value has exactly one call site
    /// to update (data-model.md).
    pub fn refresh_auth_state(&mut self) {
        self.auth = match auth_status() {
            "service_account" | "oauth_token" => AuthState::SignedIn,
            _ => AuthState::SignedOut,
        };
        self.account_label = if self.auth == AuthState::SignedIn {
            account_label()
        } else {
            None
        };
        self.can_sign_out = can_sign_out();
    }

    /// Kicks off the initial (and any later Refresh, 005-gui-refresh) sheet sync on a background
    /// thread, mirroring `awards-tui`'s `start_sync` (research.md §6 of 004-gui-lookup-add).
    /// A no-op while a sync is already running (005-gui-refresh spec FR-004) — this guard lives
    /// here rather than only in the Refresh button's enabled state, so the invariant holds no
    /// matter what ever calls this (005-gui-refresh research.md §2).
    pub fn start_sync(&mut self) {
        if self.syncing {
            return;
        }
        self.syncing = true;
        self.set_status("Syncing...", StatusKind::Info);
        let tx = self.tx.clone();
        thread::spawn(move || {
            let result = build_awards_data(None).map_err(|e| e.to_string());
            let _ = tx.send(GuiMsg::SyncDone(result));
        });
        (self.repaint)();
    }

    /// Drains every message currently queued and dispatches each through `handle_msg`. Call once
    /// per `eframe::App::update` tick.
    pub fn drain_messages(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            self.handle_msg(msg);
        }
    }

    /// The single dispatch point for every background-thread message (mirrors
    /// `awards-tui`'s `handle_worker_msg`).
    pub fn handle_msg(&mut self, msg: GuiMsg) {
        match msg {
            GuiMsg::SyncDone(result) => self.handle_sync_done(result),
            GuiMsg::LoginDone(result) => self.handle_login_done(result),
            GuiMsg::AddDone { username, result } => self.handle_add_done(username, result),
            GuiMsg::EditDone { username, result } => self.handle_edit_done(username, result),
        }
    }

    fn handle_sync_done(&mut self, result: Result<AwardsData, String>) {
        self.syncing = false;
        match result {
            Ok(data) => {
                self.sync_error = None;
                // Routine/automatic, not a clerk-deliberate action — stays Info (see StatusKind).
                self.set_status(
                    format!("Synced — {} known award(s) in catalog", data.catalog.len()),
                    StatusKind::Info,
                );
                self.data = Some(data);
            }
            Err(err) => {
                // A sync failure blocks every other capability, so it earns Error even though
                // sync itself is routine.
                self.set_status(format!("Sync failed: {err}"), StatusKind::Error);
                self.sync_error = Some(err);
            }
        }
    }

    // ---- User Story 1: Lookup ----------------------------------------------------------------

    /// Looks up `username_input` purely locally against the already-synced `AwardsData` index —
    /// no network call, mirroring `awards-tui`'s `apply_user_view` (research.md §6). Disabled
    /// while `syncing` (spec Edge Case: no lookup against an incomplete/absent index).
    pub fn submit_lookup(&mut self) {
        if self.syncing {
            return;
        }
        let Some(data) = self.data.as_ref() else {
            match &self.sync_error {
                Some(err) => self.set_status(
                    format!("Sync failed: {err} — no data to look up"),
                    StatusKind::Error,
                ),
                None => {
                    self.set_status("Still syncing — try again shortly", StatusKind::Info);
                }
            };
            return;
        };
        let username = self.username_input.trim().to_string();
        if username.is_empty() {
            self.set_status("Enter a username to look up", StatusKind::Info);
            return;
        }
        let awards = get_awards_for_username(&data.index, &username);
        if awards.is_empty() {
            self.looked_up = Some(LookedUpUser {
                username,
                awards: Vec::new(),
                not_found: true,
            });
            self.set_status("No records found for that username", StatusKind::Info);
        } else {
            self.set_status(
                format!("{username} · {} award(s)", awards.len()),
                StatusKind::Info,
            );
            self.looked_up = Some(LookedUpUser {
                username,
                awards,
                not_found: false,
            });
        }
        // A fresh lookup invalidates any in-progress add or edit for the previous user.
        self.add_picker = None;
        self.edit = None;
    }

    // ---- User Story 2: Add ---------------------------------------------------------------------

    /// Opens the award picker for the currently looked-up user (spec FR-003). Computes
    /// `candidates` once as the full catalog minus this user's already-owned award columns
    /// (`owned_award_columns`, research.md §6); if nothing remains, reports that instead of
    /// opening an empty picker (spec Edge Case).
    pub fn open_add_picker(&mut self) {
        let (Some(data), Some(looked_up)) = (self.data.as_ref(), self.looked_up.as_ref()) else {
            self.set_status("Look up a user before adding an award", StatusKind::Info);
            return;
        };
        let owned = owned_award_columns(&looked_up.awards, &looked_up.username);
        let candidates: Vec<AwardDef> = data
            .catalog
            .iter()
            .filter(|def| !owned.contains(&(def.sheet.clone(), def.col.clone())))
            .cloned()
            .collect();
        if candidates.is_empty() {
            self.set_status("No remaining awards to add for this user", StatusKind::Info);
            return;
        }
        // Only one in-progress write flow at a time (mirrors the TUI's single-modal rule,
        // Constitution III) — opening Add closes any open Edit.
        self.edit = None;
        self.add_picker = Some(AddPicker::new(candidates));
    }

    /// Recomputes `filtered` from `filter`, identical behavior to the TUI's own Add picker filter
    /// (research.md §6).
    pub fn update_filter(&mut self, text: String) {
        let Some(picker) = self.add_picker.as_mut() else {
            return;
        };
        picker.filter = text;
        picker.filtered = match_catalog_entries(&picker.candidates, &picker.filter);
    }

    /// Selecting an award is deliberately a separate step from confirming it (spec FR-004) — this
    /// alone never writes anything.
    pub fn select(&mut self, def: AwardDef) {
        if let Some(picker) = self.add_picker.as_mut() {
            picker.selected = Some(def);
        }
    }

    pub fn set_suffix(&mut self, suffix: String) {
        if let Some(picker) = self.add_picker.as_mut() {
            picker.suffix = suffix;
        }
    }

    /// Closes the picker with no side effect. `ui.rs::render` binds `Escape` to this, matching
    /// the TUI's own Esc-cancel behavior.
    pub fn cancel_add(&mut self) {
        self.add_picker = None;
    }

    /// `true` only when a confirm is actually actionable right now — used by `ui.rs` to enable or
    /// disable the Confirm control (contract: Add table).
    pub fn can_confirm_add(&self) -> bool {
        self.auth == AuthState::SignedIn
            && self
                .add_picker
                .as_ref()
                .is_some_and(|p| p.selected.is_some() && !p.submitting)
    }

    /// Spawns the background write through the exact same guarded path
    /// (`add_award_to_user(.., interactive_auth: false)`) the terminal tool's own Add flow uses
    /// (research.md §5) — this feature introduces no new or differently-guarded write path
    /// (spec FR-005).
    pub fn confirm_add(&mut self) {
        if !self.can_confirm_add() {
            return;
        }
        let Some(username) = self.looked_up.as_ref().map(|u| u.username.clone()) else {
            return;
        };
        let Some(picker) = self.add_picker.as_mut() else {
            return;
        };
        let Some(award_def) = picker.selected.clone() else {
            return;
        };
        let suffix = picker.suffix.clone();
        picker.submitting = true;

        let tx = self.tx.clone();
        thread::spawn(move || {
            let result = add_award_to_user(&username, &award_def, &suffix, false);
            let _ = tx.send(GuiMsg::AddDone { username, result });
        });
        (self.repaint)();
    }

    fn handle_add_done(&mut self, username: String, result: EditResult) {
        let (status, kind) = describe_result(&result);
        if result.ok {
            if let Some(award) = result.award.as_ref() {
                // Keep the shared index in sync (mirrors awards-tui's apply_add_result calling
                // upsert_award_in_index) so a later lookup of this same username — after the
                // clerk has looked at someone else in between — reflects this add without
                // waiting for the next full sync (converge finding F1).
                if let Some(data) = self.data.as_mut() {
                    upsert_award_in_index(&mut data.index, award);
                }
                if let Some(looked_up) = self.looked_up.as_mut() {
                    if looked_up.username == username {
                        looked_up.awards.push(award.clone());
                        looked_up.not_found = false;
                    }
                }
            }
            self.add_picker = None;
        } else {
            // Stale/Conflict/etc: refuse and let the clerk retry without losing their
            // in-progress selection (spec Acceptance Scenario 2.4) — never a silent overwrite.
            if let Some(picker) = self.add_picker.as_mut() {
                picker.submitting = false;
            }
        }
        self.set_status(status, kind);
    }

    // ---- User Story 3: Edit (spec 007-gui-edit) ------------------------------------------------

    /// Opens the edit flow for one award already shown in the results (spec FR-001), prefilled
    /// with its current cell text — or its display name if the cell is somehow empty — exactly
    /// matching the TUI's own `action_edit` prefill (research.md §2). Closes any open Add picker
    /// (only one write flow open at a time, Constitution III).
    pub fn open_edit(&mut self, award: Award) {
        let value = if award.cell.is_empty() {
            award.name.clone()
        } else {
            award.cell.clone()
        };
        self.add_picker = None;
        self.edit = Some(EditFlow {
            award,
            input: value,
            submitting: false,
        });
    }

    pub fn set_edit_input(&mut self, text: String) {
        if let Some(edit) = self.edit.as_mut() {
            edit.input = text;
        }
    }

    /// Closes the flow with no write. `ui.rs::render` binds `Escape` to this, matching the TUI's
    /// own Esc-cancel behavior.
    pub fn cancel_edit(&mut self) {
        self.edit = None;
    }

    /// `true` only when a confirm is actually actionable right now (spec FR-002). Gated on
    /// `SignedIn` even though the TUI's own `action_edit`/`commit_edit` has no explicit auth
    /// check of its own (it just lets the write call fail) — deliberately matching this GUI's
    /// existing Add-confirm pattern instead, since a windowed app has no always-visible terminal
    /// line to surface a doomed write's error against; this is a GUI-specific hardening of the
    /// same underlying rule, not a functional gap (research.md §3).
    pub fn can_confirm_edit(&self) -> bool {
        self.auth == AuthState::SignedIn
            && self
                .edit
                .as_ref()
                .is_some_and(|e| !e.input.trim().is_empty() && !e.submitting)
    }

    /// Spawns the background write through the exact same guarded path
    /// (`update_award_cell(.., interactive_auth: false)`) the terminal tool's own Edit flow uses
    /// (research.md §2) — no new or differently-guarded write path.
    pub fn confirm_edit(&mut self) {
        if !self.can_confirm_edit() {
            return;
        }
        let Some(username) = self.looked_up.as_ref().map(|u| u.username.clone()) else {
            return;
        };
        let Some(edit) = self.edit.as_mut() else {
            return;
        };
        let award = edit.award.clone();
        let new_cell = edit.input.trim().to_string();
        edit.submitting = true;

        let tx = self.tx.clone();
        thread::spawn(move || {
            let result = update_award_cell(&award, &new_cell, false);
            let _ = tx.send(GuiMsg::EditDone { username, result });
        });
        (self.repaint)();
    }

    /// On success, patches the shared index (mirrors `upsert_award_in_index` in `handle_add_done`
    /// and the TUI's own `apply_edit_result`) and then *recomputes* the viewed user's award list
    /// from that patched index — rather than hand-patching one entry, like `handle_add_done`
    /// does — because an edit can reassign a cell to a different username (the underlying
    /// `update_award_cell` guards against a live collision but still allows a deliberate
    /// reassignment); recomputing is what correctly drops the award from this view when that
    /// happens, matching the TUI's `apply_edit_result` "no longer under @user" behavior
    /// (research.md §2) without duplicating its username-comparison logic.
    fn handle_edit_done(&mut self, viewed_username: String, result: EditResult) {
        let (status, kind) = describe_result(&result);
        if result.ok {
            let mut moved_away = false;
            if let Some(award) = result.award.as_ref() {
                if let Some(data) = self.data.as_mut() {
                    upsert_award_in_index(&mut data.index, award);
                }
                if let Some(looked_up) = self.looked_up.as_mut() {
                    if looked_up.username == viewed_username {
                        if let Some(data) = self.data.as_ref() {
                            looked_up.awards =
                                get_awards_for_username(&data.index, &viewed_username);
                            looked_up.not_found = looked_up.awards.is_empty();
                            moved_away = !looked_up
                                .awards
                                .iter()
                                .any(|a| a.sheet == award.sheet && a.col == award.col);
                        }
                    }
                }
            }
            self.edit = None;
            let status = if moved_away {
                format!("{status} · no longer under @{viewed_username}")
            } else {
                status
            };
            self.set_status(status, kind);
        } else {
            // Stale/Conflict/Validation/etc: refuse and let the clerk retry without losing their
            // in-progress edit, matching Add's own refusal handling.
            if let Some(edit) = self.edit.as_mut() {
                edit.submitting = false;
            }
            self.set_status(status, kind);
        }
    }

    // ---- Sign-in (research.md §4) --------------------------------------------------------------

    pub fn start_sign_in(&mut self) {
        if self.auth == AuthState::SigningIn {
            return;
        }
        self.auth = AuthState::SigningIn;
        self.set_status("Signing in...", StatusKind::Info);
        let tx = self.tx.clone();
        thread::spawn(move || {
            let result = login().map_err(|e| e.to_string());
            let _ = tx.send(GuiMsg::LoginDone(result));
        });
        (self.repaint)();
    }

    fn handle_login_done(&mut self, result: Result<String, String>) {
        match result {
            Ok(msg) => self.set_status(msg, StatusKind::Success),
            Err(msg) => self.set_status(msg, StatusKind::Error),
        }
        // Re-derive from auth_status() rather than assuming Ok means SignedIn — mirrors what
        // login() itself guarantees (or doesn't) about the on-disk token.
        self.refresh_auth_state();
    }

    /// Ends the current OAuth session (no-op under a service account — see `can_sign_out`) and
    /// discards any in-progress write flow, since both are gated on `SignedIn` and would
    /// otherwise sit open with no way to actually confirm.
    pub fn sign_out(&mut self) {
        logout();
        self.add_picker = None;
        self.edit = None;
        self.refresh_auth_state();
        self.set_status("Signed out.", StatusKind::Info);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn award(
        category: &str,
        base_name: &str,
        sheet: &str,
        col: &str,
        row: i32,
        cell: &str,
    ) -> Award {
        Award::new(category, base_name)
            .with_location(sheet, col, row)
            .with_cell(cell, base_name)
    }

    fn def(category: &str, sheet: &str, col: &str, base_name: &str) -> AwardDef {
        AwardDef {
            category: category.to_string(),
            sheet: sheet.to_string(),
            col: col.to_string(),
            base_name: base_name.to_string(),
        }
    }

    fn counting_repaint() -> (impl Fn() + Send + Sync + 'static, Arc<AtomicUsize>) {
        let count = Arc::new(AtomicUsize::new(0));
        let counter = count.clone();
        (
            move || {
                counter.fetch_add(1, Ordering::SeqCst);
            },
            count,
        )
    }

    fn no_data_app() -> GuiApp {
        // GuiApp::new() kicks off a real network sync via start_sync(); tests instead build the
        // struct's fields directly through handle_msg so no network access is required, matching
        // awards-tui/src/tui/app.rs's existing test convention for headless testability.
        let (tx, rx) = mpsc::channel();
        GuiApp {
            tx,
            rx,
            repaint: Box::new(|| {}),
            data: None,
            syncing: true,
            sync_error: None,
            username_input: String::new(),
            looked_up: None,
            auth: AuthState::Unknown,
            account_label: None,
            can_sign_out: false,
            add_picker: None,
            edit: None,
            status: String::new(),
            status_kind: StatusKind::Info,
        }
    }

    fn sample_data() -> AwardsData {
        let mut index: HashMap<String, Vec<Award>> = HashMap::new();
        index.insert(
            "torba_f".to_string(),
            vec![award(
                "badges",
                "Army Parachutist Badge",
                "Badges Database",
                "C",
                5,
                "torba_f",
            )],
        );
        AwardsData {
            index,
            catalog: vec![
                def("badges", "Badges Database", "C", "Army Parachutist Badge"),
                def("badges", "Badges Database", "D", "Air Assault Badge"),
                def("ribbons", "Ribbons Database", "E", "Good Conduct Ribbon"),
            ],
            sheet_rows: HashMap::new(),
        }
    }

    #[test]
    fn handle_sync_done_ok_populates_data_and_clears_syncing() {
        let mut app = no_data_app();
        app.handle_msg(GuiMsg::SyncDone(Ok(sample_data())));
        assert!(!app.syncing);
        assert!(app.sync_error.is_none());
        assert_eq!(app.data.unwrap().catalog.len(), 3);
    }

    /// 005-gui-refresh spec FR-004: a second `start_sync()` call while one is already running
    /// must not spawn a second background fetch. Asserted without any real network access by
    /// constructing the app with `syncing` already `true` (as if a sync were mid-flight) and
    /// confirming the guard returns before the repaint callback — and therefore the thread
    /// spawn just before it — is ever reached.
    #[test]
    fn start_sync_is_a_no_op_while_a_sync_is_already_running() {
        let (repaint, count) = counting_repaint();
        let mut app = no_data_app();
        app.repaint = Box::new(repaint);
        app.syncing = true;
        let status_before = app.status.clone();
        app.start_sync();
        assert_eq!(count.load(Ordering::SeqCst), 0);
        assert_eq!(app.status, status_before);
    }

    /// 005-gui-refresh spec FR-006: a failed refresh (or the initial sync) must never discard
    /// data from a previous successful sync.
    #[test]
    fn handle_sync_done_err_leaves_previously_synced_data_untouched() {
        let mut app = no_data_app();
        app.handle_msg(GuiMsg::SyncDone(Ok(sample_data())));
        assert!(app.data.is_some());
        app.handle_msg(GuiMsg::SyncDone(Err("connection dropped".to_string())));
        assert!(!app.syncing);
        assert_eq!(app.sync_error.as_deref(), Some("connection dropped"));
        assert!(
            app.data.is_some(),
            "a failed refresh must not blank out previously-synced data"
        );
    }

    #[test]
    fn handle_sync_done_err_sets_sync_error_and_clears_syncing() {
        let mut app = no_data_app();
        app.handle_msg(GuiMsg::SyncDone(Err("network down".to_string())));
        assert!(!app.syncing);
        assert!(app.data.is_none());
        assert_eq!(app.sync_error.as_deref(), Some("network down"));
    }

    #[test]
    fn submit_lookup_known_username_populates_awards() {
        let mut app = no_data_app();
        app.handle_msg(GuiMsg::SyncDone(Ok(sample_data())));
        app.username_input = "torba_f".to_string();
        app.submit_lookup();
        let looked_up = app.looked_up.expect("lookup should populate looked_up");
        assert!(!looked_up.not_found);
        assert_eq!(looked_up.awards.len(), 1);
    }

    #[test]
    fn submit_lookup_unknown_username_sets_not_found() {
        let mut app = no_data_app();
        app.handle_msg(GuiMsg::SyncDone(Ok(sample_data())));
        app.username_input = "nobody_here".to_string();
        app.submit_lookup();
        let looked_up = app
            .looked_up
            .expect("lookup should still populate looked_up");
        assert!(looked_up.not_found);
        assert!(looked_up.awards.is_empty());
    }

    #[test]
    fn submit_lookup_before_sync_completes_does_not_panic() {
        let mut app = no_data_app();
        app.username_input = "torba_f".to_string();
        app.submit_lookup();
        assert!(app.looked_up.is_none());
    }

    /// Regression test for converge finding F2: after a permanent sync failure, Look Up must not
    /// claim the sync is still in progress.
    #[test]
    fn submit_lookup_after_a_failed_sync_reports_the_failure_not_still_syncing() {
        let mut app = no_data_app();
        app.handle_msg(GuiMsg::SyncDone(Err("network down".to_string())));
        app.username_input = "torba_f".to_string();
        app.submit_lookup();
        assert!(app.looked_up.is_none());
        assert!(app.status.contains("Sync failed"));
        assert!(!app.status.contains("Still syncing"));
    }

    #[test]
    fn open_add_picker_excludes_already_owned_awards() {
        let mut app = no_data_app();
        app.handle_msg(GuiMsg::SyncDone(Ok(sample_data())));
        app.username_input = "torba_f".to_string();
        app.submit_lookup();
        app.open_add_picker();
        let picker = app
            .add_picker
            .expect("candidates remain, picker should open");
        assert_eq!(picker.candidates.len(), 2);
        assert!(picker
            .candidates
            .iter()
            .all(|d| d.base_name != "Army Parachutist Badge"));
    }

    #[test]
    fn open_add_picker_with_nothing_left_stays_closed() {
        let mut app = no_data_app();
        let mut data = sample_data();
        data.catalog
            .retain(|d| d.base_name == "Army Parachutist Badge");
        app.handle_msg(GuiMsg::SyncDone(Ok(data)));
        app.username_input = "torba_f".to_string();
        app.submit_lookup();
        app.open_add_picker();
        assert!(app.add_picker.is_none());
        assert_eq!(app.status, "No remaining awards to add for this user");
    }

    #[test]
    fn update_filter_narrows_to_matching_candidates() {
        let mut app = no_data_app();
        app.handle_msg(GuiMsg::SyncDone(Ok(sample_data())));
        app.username_input = "torba_f".to_string();
        app.submit_lookup();
        app.open_add_picker();
        app.update_filter("air".to_string());
        let picker = app.add_picker.unwrap();
        assert_eq!(picker.filtered.len(), 1);
        assert_eq!(picker.filtered[0].base_name, "Air Assault Badge");
    }

    #[test]
    fn select_only_sets_selection_no_write() {
        let mut app = no_data_app();
        app.handle_msg(GuiMsg::SyncDone(Ok(sample_data())));
        app.username_input = "torba_f".to_string();
        app.submit_lookup();
        app.open_add_picker();
        let candidate = app.add_picker.as_ref().unwrap().candidates[0].clone();
        app.select(candidate.clone());
        assert_eq!(app.add_picker.unwrap().selected, Some(candidate));
    }

    #[test]
    fn handle_add_done_ok_appends_award_and_closes_picker() {
        let mut app = no_data_app();
        app.handle_msg(GuiMsg::SyncDone(Ok(sample_data())));
        app.username_input = "torba_f".to_string();
        app.submit_lookup();
        app.open_add_picker();
        let candidate = app.add_picker.as_ref().unwrap().candidates[0].clone();
        app.select(candidate.clone());

        let new_award = award(
            "badges",
            "Air Assault Badge",
            "Badges Database",
            "D",
            5,
            "torba_f",
        );
        app.handle_add_done(
            "torba_f".to_string(),
            EditResult {
                ok: true,
                message: "Added Air Assault Badge".to_string(),
                error: None,
                award: Some(new_award),
                awards: Vec::new(),
            },
        );
        assert!(app.add_picker.is_none());
        assert_eq!(app.looked_up.unwrap().awards.len(), 2);
        assert_eq!(app.status, "Added Air Assault Badge");
        assert_eq!(app.status_kind, StatusKind::Success);
    }

    /// Regression test for converge finding F1: looking up a different user and then looking the
    /// original user back up must still show an award added earlier in the session, without
    /// waiting for a fresh full sync.
    #[test]
    fn handle_add_done_ok_keeps_a_later_relookup_of_the_same_user_accurate() {
        let mut app = no_data_app();
        let mut data = sample_data();
        data.index.insert("someone_else".to_string(), Vec::new());
        app.handle_msg(GuiMsg::SyncDone(Ok(data)));
        app.username_input = "torba_f".to_string();
        app.submit_lookup();
        app.open_add_picker();
        let candidate = app.add_picker.as_ref().unwrap().candidates[0].clone();
        app.select(candidate.clone());

        let new_award = award(
            "badges",
            "Air Assault Badge",
            "Badges Database",
            "D",
            5,
            "torba_f",
        );
        app.handle_add_done(
            "torba_f".to_string(),
            EditResult {
                ok: true,
                message: "Added Air Assault Badge".to_string(),
                error: None,
                award: Some(new_award),
                awards: Vec::new(),
            },
        );

        // Look at a different user, then come back to torba_f.
        app.username_input = "someone_else".to_string();
        app.submit_lookup();
        app.username_input = "torba_f".to_string();
        app.submit_lookup();

        assert_eq!(app.looked_up.unwrap().awards.len(), 2);
    }

    #[test]
    fn handle_add_done_failure_keeps_picker_open_with_selection_intact() {
        let mut app = no_data_app();
        app.handle_msg(GuiMsg::SyncDone(Ok(sample_data())));
        app.username_input = "torba_f".to_string();
        app.submit_lookup();
        app.open_add_picker();
        let candidate = app.add_picker.as_ref().unwrap().candidates[0].clone();
        app.select(candidate.clone());
        app.add_picker.as_mut().unwrap().submitting = true;

        app.handle_add_done(
            "torba_f".to_string(),
            EditResult {
                ok: false,
                message: "That cell changed since you looked up this user — refresh and retry"
                    .to_string(),
                error: None,
                award: None,
                awards: Vec::new(),
            },
        );
        let picker = app
            .add_picker
            .expect("picker stays open on a refused write");
        assert!(!picker.submitting);
        assert_eq!(picker.selected, Some(candidate));
        assert_eq!(app.looked_up.unwrap().awards.len(), 1);
        assert_eq!(app.status_kind, StatusKind::Error);
    }

    /// Critique follow-up: a raw `EditError::Api` string (network/auth/HTTP detail) must not
    /// reach the clerk verbatim — `describe_result` swaps it for plain language.
    #[test]
    fn handle_add_done_api_failure_shows_plain_language_not_the_raw_error() {
        let mut app = no_data_app();
        app.handle_msg(GuiMsg::SyncDone(Ok(sample_data())));
        app.username_input = "torba_f".to_string();
        app.submit_lookup();
        app.open_add_picker();
        let candidate = app.add_picker.as_ref().unwrap().candidates[0].clone();
        app.select(candidate);

        app.handle_add_done(
            "torba_f".to_string(),
            EditResult {
                ok: false,
                message: "Sheets API HTTP 401: {\"error\":\"invalid_grant\"}".to_string(),
                error: Some(EditError::Api(
                    "HTTP 401: {\"error\":\"invalid_grant\"}".to_string(),
                )),
                award: None,
                awards: Vec::new(),
            },
        );
        assert_eq!(
            app.status,
            "Couldn't reach Google Sheets. Check your connection and try again."
        );
        assert_eq!(app.status_kind, StatusKind::Error);
    }

    #[test]
    fn handle_login_done_ok_refreshes_to_signed_in_when_status_agrees() {
        // auth_status() reads real on-disk credential files, which this headless test
        // environment doesn't have — so it will resolve to SignedOut regardless of the Ok/Err
        // message. What this test asserts is the *shape* of the transition: handle_login_done
        // always re-derives `auth` from refresh_auth_state() rather than assuming Ok means
        // SignedIn, matching data-model.md's documented behavior.
        let mut app = no_data_app();
        app.auth = AuthState::SigningIn;
        app.handle_login_done(Ok("Signed in.".to_string()));
        assert_eq!(app.status, "Signed in.");
        assert_eq!(app.status_kind, StatusKind::Success);
        assert_ne!(app.auth, AuthState::SigningIn);
    }

    #[test]
    fn handle_login_done_err_leaves_auth_not_signed_in() {
        let mut app = no_data_app();
        app.auth = AuthState::SigningIn;
        app.handle_login_done(Err("No credentials found".to_string()));
        assert_eq!(app.status, "No credentials found");
        assert_eq!(app.status_kind, StatusKind::Error);
        assert_ne!(app.auth, AuthState::SignedIn);
    }

    #[test]
    fn confirm_add_is_a_no_op_without_a_selection() {
        let (repaint, count) = counting_repaint();
        let mut app = no_data_app();
        app.repaint = Box::new(repaint);
        app.handle_msg(GuiMsg::SyncDone(Ok(sample_data())));
        app.username_input = "torba_f".to_string();
        app.submit_lookup();
        app.open_add_picker();
        app.auth = AuthState::SignedIn;
        // No selection yet — confirm_add must not spawn a write thread or repaint.
        app.confirm_add();
        assert_eq!(count.load(Ordering::SeqCst), 0);
        assert!(!app.add_picker.unwrap().submitting);
    }

    #[test]
    fn confirm_add_is_a_no_op_when_not_signed_in() {
        let (repaint, count) = counting_repaint();
        let mut app = no_data_app();
        app.repaint = Box::new(repaint);
        app.handle_msg(GuiMsg::SyncDone(Ok(sample_data())));
        app.username_input = "torba_f".to_string();
        app.submit_lookup();
        app.open_add_picker();
        let candidate = app.add_picker.as_ref().unwrap().candidates[0].clone();
        app.select(candidate);
        app.auth = AuthState::SignedOut;
        app.confirm_add();
        assert_eq!(count.load(Ordering::SeqCst), 0);
        assert!(!app.add_picker.unwrap().submitting);
    }

    #[test]
    fn cancel_add_drops_the_picker_with_no_side_effect() {
        let mut app = no_data_app();
        app.handle_msg(GuiMsg::SyncDone(Ok(sample_data())));
        app.username_input = "torba_f".to_string();
        app.submit_lookup();
        app.open_add_picker();
        assert!(app.add_picker.is_some());
        app.cancel_add();
        assert!(app.add_picker.is_none());
        assert_eq!(app.looked_up.unwrap().awards.len(), 1);
    }

    // ---- 007-gui-edit -------------------------------------------------------------------------

    #[test]
    fn open_edit_prefills_input_from_the_cell_when_present() {
        let mut app = no_data_app();
        app.handle_msg(GuiMsg::SyncDone(Ok(sample_data())));
        app.username_input = "torba_f".to_string();
        app.submit_lookup();
        let existing = app.looked_up.as_ref().unwrap().awards[0].clone();
        app.open_edit(existing.clone());
        let edit = app.edit.expect("edit flow should open");
        assert_eq!(edit.input, existing.cell);
        assert!(!edit.submitting);
    }

    #[test]
    fn open_edit_falls_back_to_the_name_when_the_cell_is_empty() {
        let mut app = no_data_app();
        let blank_cell = award(
            "badges",
            "Army Parachutist Badge",
            "Badges Database",
            "C",
            5,
            "",
        );
        app.open_edit(blank_cell.clone());
        assert_eq!(app.edit.unwrap().input, blank_cell.name);
    }

    #[test]
    fn open_edit_closes_any_open_add_picker() {
        let mut app = no_data_app();
        app.handle_msg(GuiMsg::SyncDone(Ok(sample_data())));
        app.username_input = "torba_f".to_string();
        app.submit_lookup();
        app.open_add_picker();
        assert!(app.add_picker.is_some());
        let existing = app.looked_up.as_ref().unwrap().awards[0].clone();
        app.open_edit(existing);
        assert!(app.add_picker.is_none());
        assert!(app.edit.is_some());
    }

    #[test]
    fn open_add_picker_closes_any_open_edit() {
        let mut app = no_data_app();
        app.handle_msg(GuiMsg::SyncDone(Ok(sample_data())));
        app.username_input = "torba_f".to_string();
        app.submit_lookup();
        let existing = app.looked_up.as_ref().unwrap().awards[0].clone();
        app.open_edit(existing);
        assert!(app.edit.is_some());
        app.open_add_picker();
        assert!(app.edit.is_none());
        assert!(app.add_picker.is_some());
    }

    #[test]
    fn set_edit_input_updates_the_field() {
        let mut app = no_data_app();
        let existing = award(
            "badges",
            "Army Parachutist Badge",
            "Badges Database",
            "C",
            5,
            "torba_f",
        );
        app.open_edit(existing);
        app.set_edit_input("torba_f x2".to_string());
        assert_eq!(app.edit.unwrap().input, "torba_f x2");
    }

    #[test]
    fn cancel_edit_drops_the_flow_with_no_side_effect() {
        let mut app = no_data_app();
        app.handle_msg(GuiMsg::SyncDone(Ok(sample_data())));
        app.username_input = "torba_f".to_string();
        app.submit_lookup();
        let existing = app.looked_up.as_ref().unwrap().awards[0].clone();
        app.open_edit(existing);
        app.cancel_edit();
        assert!(app.edit.is_none());
        assert_eq!(app.looked_up.unwrap().awards.len(), 1);
    }

    #[test]
    fn can_confirm_edit_false_without_an_open_edit() {
        let mut app = no_data_app();
        app.auth = AuthState::SignedIn;
        assert!(!app.can_confirm_edit());
    }

    #[test]
    fn can_confirm_edit_false_when_not_signed_in() {
        let mut app = no_data_app();
        let existing = award(
            "badges",
            "Army Parachutist Badge",
            "Badges Database",
            "C",
            5,
            "torba_f",
        );
        app.open_edit(existing);
        app.auth = AuthState::SignedOut;
        assert!(!app.can_confirm_edit());
    }

    #[test]
    fn can_confirm_edit_false_with_a_blank_input() {
        let mut app = no_data_app();
        let existing = award(
            "badges",
            "Army Parachutist Badge",
            "Badges Database",
            "C",
            5,
            "torba_f",
        );
        app.open_edit(existing);
        app.auth = AuthState::SignedIn;
        app.set_edit_input("   ".to_string());
        assert!(!app.can_confirm_edit());
    }

    #[test]
    fn confirm_edit_is_a_no_op_when_not_signed_in() {
        let (repaint, count) = counting_repaint();
        let mut app = no_data_app();
        app.repaint = Box::new(repaint);
        let existing = award(
            "badges",
            "Army Parachutist Badge",
            "Badges Database",
            "C",
            5,
            "torba_f",
        );
        app.open_edit(existing);
        app.auth = AuthState::SignedOut;
        app.confirm_edit();
        assert_eq!(count.load(Ordering::SeqCst), 0);
        assert!(!app.edit.unwrap().submitting);
    }

    #[test]
    fn handle_edit_done_ok_updates_the_awards_cell_text_in_place() {
        let mut app = no_data_app();
        app.handle_msg(GuiMsg::SyncDone(Ok(sample_data())));
        app.username_input = "torba_f".to_string();
        app.submit_lookup();
        let existing = app.looked_up.as_ref().unwrap().awards[0].clone();
        app.open_edit(existing.clone());
        app.auth = AuthState::SignedIn;
        app.set_edit_input("torba_f x2".to_string());
        app.confirm_edit();

        let updated = award(
            "badges",
            "Army Parachutist Badge x2",
            "Badges Database",
            "C",
            5,
            "torba_f x2",
        );
        app.handle_edit_done(
            "torba_f".to_string(),
            EditResult {
                ok: true,
                message: "Updated C5 → torba_f x2".to_string(),
                error: None,
                award: Some(updated),
                awards: Vec::new(),
            },
        );
        assert!(app.edit.is_none());
        let awards = app.looked_up.unwrap().awards;
        assert_eq!(awards.len(), 1);
        assert_eq!(awards[0].cell, "torba_f x2");
        assert_eq!(app.status, "Updated C5 → torba_f x2");
        assert_eq!(app.status_kind, StatusKind::Success);
    }

    /// When the new cell text reassigns the award to a different username, the viewed user's
    /// list must drop it (mirrors the TUI's `apply_edit_result` "no longer under @user" case).
    #[test]
    fn handle_edit_done_ok_drops_the_award_when_it_moves_to_a_different_user() {
        let mut app = no_data_app();
        app.handle_msg(GuiMsg::SyncDone(Ok(sample_data())));
        app.username_input = "torba_f".to_string();
        app.submit_lookup();
        let existing = app.looked_up.as_ref().unwrap().awards[0].clone();
        app.open_edit(existing);
        app.auth = AuthState::SignedIn;
        app.set_edit_input("someone_else".to_string());
        app.confirm_edit();

        let moved = award(
            "badges",
            "Army Parachutist Badge",
            "Badges Database",
            "C",
            5,
            "someone_else",
        );
        app.handle_edit_done(
            "torba_f".to_string(),
            EditResult {
                ok: true,
                message: "Updated C5 → someone_else".to_string(),
                error: None,
                award: Some(moved),
                awards: Vec::new(),
            },
        );
        let looked_up = app.looked_up.unwrap();
        assert!(looked_up.awards.is_empty());
        assert!(looked_up.not_found);
        assert!(app.status.contains("no longer under @torba_f"));
    }

    #[test]
    fn handle_edit_done_failure_keeps_the_flow_open_with_input_intact() {
        let mut app = no_data_app();
        app.handle_msg(GuiMsg::SyncDone(Ok(sample_data())));
        app.username_input = "torba_f".to_string();
        app.submit_lookup();
        let existing = app.looked_up.as_ref().unwrap().awards[0].clone();
        app.open_edit(existing);
        app.auth = AuthState::SignedIn;
        app.set_edit_input("torba_f x2".to_string());
        app.edit.as_mut().unwrap().submitting = true;

        app.handle_edit_done(
            "torba_f".to_string(),
            EditResult {
                ok: false,
                message: "That cell changed since you looked up this user — refresh and retry"
                    .to_string(),
                error: None,
                award: None,
                awards: Vec::new(),
            },
        );
        let edit = app.edit.expect("edit flow stays open on a refused write");
        assert!(!edit.submitting);
        assert_eq!(edit.input, "torba_f x2");
        assert_eq!(app.looked_up.unwrap().awards.len(), 1);
        assert_eq!(app.status_kind, StatusKind::Error);
    }

    #[test]
    fn a_fresh_lookup_closes_any_open_edit() {
        let mut app = no_data_app();
        app.handle_msg(GuiMsg::SyncDone(Ok(sample_data())));
        app.username_input = "torba_f".to_string();
        app.submit_lookup();
        let existing = app.looked_up.as_ref().unwrap().awards[0].clone();
        app.open_edit(existing);
        assert!(app.edit.is_some());
        app.username_input = "someone_else".to_string();
        app.submit_lookup();
        assert!(app.edit.is_none());
    }
}
