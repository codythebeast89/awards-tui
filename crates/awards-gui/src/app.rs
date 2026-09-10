//! Windowing-free application state for `awards-gui` (mirrors `awards-tui/src/tui/app.rs`'s
//! split from its own `ui.rs`). Everything in this module is plain Rust — no `egui`/`eframe`
//! window is ever opened by it — so it is unit-testable in a headless environment (plan.md
//! Testing section; this build/test environment cannot open an X11/Wayland display at all).
//!
//! Scope for this first GUI milestone (spec 004-gui-lookup-add): Lookup (read-only) and Add
//! (a single guarded write), nothing else — see spec.md FR-010 for what is deliberately absent.

use awards_core::{get_awards_for_username, match_catalog_entries, owned_award_columns};
use awards_core::{Award, AwardDef, AwardsData};
use awards_sheets::{add_award_to_user, auth_status, build_awards_data, login, EditResult};
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

    pub add_picker: Option<AddPicker>,

    /// One-line status/result message, shown at the bottom of the window.
    pub status: String,
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
            syncing: true,
            sync_error: None,
            username_input: String::new(),
            looked_up: None,
            auth: AuthState::Unknown,
            add_picker: None,
            status: "Syncing...".to_string(),
        };
        app.refresh_auth_state();
        app.start_sync();
        app
    }

    /// Maps `awards_sheets::auth_status()`'s four raw string values into `AuthState` — the only
    /// place that string is interpreted, so a future new status value has exactly one call site
    /// to update (data-model.md).
    pub fn refresh_auth_state(&mut self) {
        self.auth = match auth_status() {
            "service_account" | "oauth_token" => AuthState::SignedIn,
            _ => AuthState::SignedOut,
        };
    }

    /// Kicks off the initial (and any later Refresh) sheet sync on a background thread, mirroring
    /// `awards-tui`'s `start_sync` (research.md §6).
    pub fn start_sync(&mut self) {
        self.syncing = true;
        self.status = "Syncing...".to_string();
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
        }
    }

    fn handle_sync_done(&mut self, result: Result<AwardsData, String>) {
        self.syncing = false;
        match result {
            Ok(data) => {
                self.sync_error = None;
                self.status = format!("Synced — {} known award(s) in catalog", data.catalog.len());
                self.data = Some(data);
            }
            Err(err) => {
                self.status = format!("Sync failed: {err}");
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
            self.status = "Still syncing — try again shortly".to_string();
            return;
        };
        let username = self.username_input.trim().to_string();
        if username.is_empty() {
            self.status = "Enter a username to look up".to_string();
            return;
        }
        let awards = get_awards_for_username(&data.index, &username);
        if awards.is_empty() {
            self.looked_up = Some(LookedUpUser {
                username,
                awards: Vec::new(),
                not_found: true,
            });
            self.status = "No records found for that username".to_string();
        } else {
            self.status = format!("{username} · {} award(s)", awards.len());
            self.looked_up = Some(LookedUpUser {
                username,
                awards,
                not_found: false,
            });
        }
        // A fresh lookup invalidates any in-progress add for the previous user.
        self.add_picker = None;
    }

    // ---- User Story 2: Add ---------------------------------------------------------------------

    /// Opens the award picker for the currently looked-up user (spec FR-003). Computes
    /// `candidates` once as the full catalog minus this user's already-owned award columns
    /// (`owned_award_columns`, research.md §6); if nothing remains, reports that instead of
    /// opening an empty picker (spec Edge Case).
    pub fn open_add_picker(&mut self) {
        let (Some(data), Some(looked_up)) = (self.data.as_ref(), self.looked_up.as_ref()) else {
            self.status = "Look up a user before adding an award".to_string();
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
            self.status = "No remaining awards to add for this user".to_string();
            return;
        }
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

    /// Closes the picker with no side effect — matches the TUI's Esc-cancel *outcome* (nothing
    /// happens), even though there is no literal Esc keybinding for a window.
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
        if result.ok {
            if let (Some(looked_up), Some(award)) = (self.looked_up.as_mut(), result.award.clone())
            {
                if looked_up.username == username {
                    looked_up.awards.push(award);
                    looked_up.not_found = false;
                }
            }
            self.add_picker = None;
            self.status = result.message;
        } else {
            // Stale/Conflict/etc: refuse and let the clerk retry without losing their
            // in-progress selection (spec Acceptance Scenario 2.4) — never a silent overwrite.
            if let Some(picker) = self.add_picker.as_mut() {
                picker.submitting = false;
            }
            self.status = result.message;
        }
    }

    // ---- Sign-in (research.md §4) --------------------------------------------------------------

    pub fn start_sign_in(&mut self) {
        if self.auth == AuthState::SigningIn {
            return;
        }
        self.auth = AuthState::SigningIn;
        self.status = "Signing in...".to_string();
        let tx = self.tx.clone();
        thread::spawn(move || {
            let result = login().map_err(|e| e.to_string());
            let _ = tx.send(GuiMsg::LoginDone(result));
        });
        (self.repaint)();
    }

    fn handle_login_done(&mut self, result: Result<String, String>) {
        match result {
            Ok(msg) => self.status = msg,
            Err(msg) => self.status = msg,
        }
        // Re-derive from auth_status() rather than assuming Ok means SignedIn — mirrors what
        // login() itself guarantees (or doesn't) about the on-disk token.
        self.refresh_auth_state();
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
            add_picker: None,
            status: String::new(),
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
        assert_ne!(app.auth, AuthState::SigningIn);
    }

    #[test]
    fn handle_login_done_err_leaves_auth_not_signed_in() {
        let mut app = no_data_app();
        app.auth = AuthState::SigningIn;
        app.handle_login_done(Err("No credentials found".to_string()));
        assert_eq!(app.status, "No credentials found");
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
}
