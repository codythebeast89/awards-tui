use crate::config::AppConfig;
use awards_core::{
    awards_excluding_duplicate_rows, check_assist, col_to_index, collect_sheet_audit,
    extract_paste_fields, find_duplicates_for_user, find_grant_target, finding_username,
    flatten_audit_findings, flatten_awards_sorted, format_audit_report, get_awards_for_username,
    match_catalog_entries, normalize_username, owned_award_columns, parse_bare_username,
    reindex_column_after_delete, row_offset, shift_column_up_in_rows, split_award_suffix,
    upsert_award_in_index, AssistVerdict, Award, AwardDef, AuditFinding, AwardsData,
    CATEGORY_LABELS, GrantPlan,
};
use awards_sheets::{
    add_award_to_user, auth_status, award_with_live_row, build_awards_data, project_root,
    remove_award, rename_username, update_award_cell, EditResult, SheetsApi,
};
use chrono::Utc;
use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::widgets::ListState;
use std::fs;
use std::sync::mpsc::Sender;
use std::thread;
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

const ACTIONS: &[Action] = &[
    Action::Lookup,
    Action::Add,
    Action::PasteAdd,
    Action::Edit,
    Action::Delete,
    Action::Rename,
    Action::Assist,
    Action::Refresh,
    Action::Audit,
];

#[derive(Debug)]
pub enum WorkerMsg {
    SyncDone(Result<AwardsData, String>),
    RowsFixed {
        gen: u64,
        username: String,
        results: Vec<Award>,
        duplicates: Vec<Award>,
    },
    WriteDone {
        kind: &'static str,
        result: EditResult,
        username: String,
    },
    AuditDone(Result<AuditOutcome, String>),
}

#[derive(Debug, Clone)]
pub struct AuditOutcome {
    pub path: String,
    pub body: String,
    pub summary: String,
    pub findings: Vec<AuditFinding>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusArea {
    Username,
    Actions,
    Awards,
    Detail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Lookup,
    Add,
    /// Paste a forwarded Discord request and pre-fill the Add flow from it
    /// (003-discord-paste-quick-add). Unlike `Add`, opening this does not require a username
    /// already looked up.
    PasteAdd,
    Edit,
    Delete,
    Rename,
    Assist,
    Refresh,
    Audit,
}

impl Action {
    pub fn label(self) -> &'static str {
        match self {
            Self::Lookup => "Lookup",
            Self::Add => "Add",
            Self::PasteAdd => "Paste",
            Self::Edit => "Edit",
            Self::Delete => "Delete",
            Self::Rename => "Rename",
            Self::Assist => "Assist",
            Self::Refresh => "Refresh",
            Self::Audit => "Audit",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AwardTab {
    All,
    Badges,
    Ribbons,
    Foreign,
    Duplicates,
}

impl AwardTab {
    pub const ALL: [Self; 5] = [
        Self::All,
        Self::Badges,
        Self::Ribbons,
        Self::Foreign,
        Self::Duplicates,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Badges => "Badges",
            Self::Ribbons => "Ribbons",
            Self::Foreign => "Foreign",
            Self::Duplicates => "Duplicates/Typos",
        }
    }

    pub fn category(self) -> Option<&'static str> {
        match self {
            Self::Badges => Some("badges"),
            Self::Ribbons => Some("ribbons"),
            Self::Foreign => Some("foreign"),
            Self::All | Self::Duplicates => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VisibleAward {
    pub award: Award,
    pub warning: bool,
}

#[derive(Debug)]
pub enum Modal {
    Add(AddModal),
    /// Discord-paste entry point (003-discord-paste-quick-add) — resolves into a pre-filled
    /// `Modal::Add` (confident match or filtered picker) on submit; see
    /// `contracts/tui-paste-interaction.md`.
    PasteAdd(PasteAddModal),
    Edit(EditModal),
    Delete(DeleteModal),
    Rename(RenameModal),
    Assist(AssistModal),
    Audit(AuditModal),
}

/// State for the Discord-paste entry point (003-discord-paste-quick-add). `buffer` accumulates
/// both bracketed-paste content and typed fallback input; it is never persisted or logged
/// (FR-010) and is dropped the moment the modal resolves into a `Modal::Add` or is cancelled.
#[derive(Debug, Default)]
pub struct PasteAddModal {
    pub buffer: String,
    pub error: Option<String>,
}

#[derive(Debug)]
pub enum AssistStep {
    Query,
    Result,
}

#[derive(Debug)]
pub struct AssistModal {
    pub username: String,
    pub input: Input,
    pub step: AssistStep,
    pub report: String,
    pub can_grant: bool,
    pub grant: Option<GrantPlan>,
    pub scroll: u16,
}

#[derive(Debug)]
pub struct AuditModal {
    pub path: String,
    pub lines: Vec<String>,
    pub scroll: u16,
    /// Which face of the audit browser is currently displayed. Kept as a separate,
    /// stateless field from `list` so toggling between the findings list and the
    /// plain-text report never discards the findings list's data.
    pub view: AuditView,
    pub list: AuditFindingsList,
    pub list_state: ListState,
}

impl AuditModal {
    /// Build a fresh audit modal defaulted to the findings-list view, selecting the
    /// first selectable row (if any).
    pub fn new(path: String, body: &str, findings: Vec<AuditFinding>) -> Self {
        let list = AuditFindingsList::from_findings(findings);
        let mut list_state = ListState::default();
        if !list.is_empty() {
            list_state.select(list.selectable_rows().first().copied());
        }
        Self {
            path,
            lines: body.lines().map(str::to_string).collect(),
            scroll: 0,
            view: AuditView::List,
            list,
            list_state,
        }
    }
}

/// Which face of the Audit modal is currently displayed. `ChooseUsername` is a
/// transient one-shot sub-dialog used to resolve a `SimilarUsernames` finding into a
/// single account to rename; it is not preserved across a List/Report toggle.
#[derive(Debug, Clone)]
pub enum AuditView {
    List,
    Report,
    ChooseUsername {
        a: String,
        b: String,
    },
}

#[derive(Debug, Clone)]
pub struct FindingGroup {
    pub username: Option<String>,
    pub findings: Vec<AuditFinding>,
}

/// The audit's flagged entries, grouped by the username/person involved (an
/// "(unattributed)" group, if present, always sorts last since it has no username to
/// order by).
#[derive(Debug, Clone, Default)]
pub struct AuditFindingsList {
    pub groups: Vec<FindingGroup>,
}

/// One rendered row of the findings list: either a non-selectable group header, or a
/// selectable finding.
pub enum AuditRow<'a> {
    Header(String),
    Item(&'a AuditFinding),
}

impl AuditFindingsList {
    pub fn from_findings(findings: Vec<AuditFinding>) -> Self {
        use std::collections::BTreeMap;
        let mut named: BTreeMap<String, Vec<AuditFinding>> = BTreeMap::new();
        let mut unattributed: Vec<AuditFinding> = Vec::new();
        for finding in findings {
            match finding_username(&finding) {
                Some(user) => named.entry(user.to_string()).or_default().push(finding),
                None => unattributed.push(finding),
            }
        }
        let mut groups: Vec<FindingGroup> = named
            .into_iter()
            .map(|(username, findings)| FindingGroup {
                username: Some(username),
                findings,
            })
            .collect();
        if !unattributed.is_empty() {
            groups.push(FindingGroup {
                username: None,
                findings: unattributed,
            });
        }
        Self { groups }
    }

    pub fn is_empty(&self) -> bool {
        self.groups.iter().all(|g| g.findings.is_empty())
    }

    pub fn len(&self) -> usize {
        self.groups.iter().map(|g| g.findings.len()).sum()
    }

    /// Rendered rows in display order: one header per non-empty group, followed by its
    /// findings.
    pub fn rows_for_render(&self) -> Vec<AuditRow<'_>> {
        let mut out = Vec::new();
        for group in &self.groups {
            let label = match &group.username {
                Some(user) => format!("@{user}"),
                None => "(unattributed)".to_string(),
            };
            out.push(AuditRow::Header(label));
            for finding in &group.findings {
                out.push(AuditRow::Item(finding));
            }
        }
        out
    }

    /// Row indices (into `rows_for_render`'s order) that hold a finding rather than a
    /// group header, in display order.
    pub fn selectable_rows(&self) -> Vec<usize> {
        let mut out = Vec::new();
        let mut row = 0usize;
        for group in &self.groups {
            row += 1; // header row
            for _ in &group.findings {
                out.push(row);
                row += 1;
            }
        }
        out
    }

    /// The finding at the given row index (into `rows_for_render`'s order), or `None`
    /// if that row is a header (or out of range).
    pub fn finding_at_row(&self, row_idx: usize) -> Option<&AuditFinding> {
        let mut row = 0usize;
        for group in &self.groups {
            if row == row_idx {
                return None;
            }
            row += 1;
            for finding in &group.findings {
                if row == row_idx {
                    return Some(finding);
                }
                row += 1;
            }
        }
        None
    }
}

/// One-line human-readable description of an audit finding, used both in the
/// selectable findings list and (indirectly) wherever a finding needs a short label.
pub(crate) fn describe_finding(finding: &AuditFinding) -> String {
    match finding {
        AuditFinding::DuplicateRow {
            base_name,
            sheet,
            col,
            row,
            kind,
            ..
        } => {
            let label = if kind == "identical" {
                "duplicate copy"
            } else {
                "conflicting rows"
            };
            format!("⚠ {base_name} ({label}) · {sheet} {col}{row}")
        }
        AuditFinding::MalformedCell {
            base_name,
            sheet,
            col,
            row,
            cell,
            ..
        } => {
            format!("⚠ {base_name} (malformed cell \"{cell}\") · {sheet} {col}{row}")
        }
        AuditFinding::SimilarUsernames { a, b, base_name, .. } => {
            format!("⚠ {base_name}: @{a} vs @{b} (similar usernames)")
        }
        AuditFinding::UnparseableCell {
            base_name,
            sheet,
            col,
            row,
            cell,
            ..
        } => {
            format!("⚠ {base_name} (unparseable \"{cell}\") · {sheet} {col}{row}")
        }
    }
}

#[derive(Debug)]
pub enum AddStep {
    Pick,
    Suffix,
}

#[derive(Debug)]
pub struct AddModal {
    pub all_candidates: Vec<AwardDef>,
    pub filtered: Vec<AwardDef>,
    pub filter: Input,
    pub suffix: Input,
    pub state: ListState,
    pub step: AddStep,
    pub chosen: Option<AwardDef>,
}

#[derive(Debug)]
pub struct EditModal {
    pub award: Award,
    pub input: Input,
}

#[derive(Debug)]
pub struct DeleteModal {
    pub award: Award,
    pub input: Input,
    pub viewed_username: String,
}

#[derive(Debug)]
pub enum RenameStep {
    Name,
    Confirm,
}

#[derive(Debug)]
pub struct RenameModal {
    pub from: String,
    pub cell_count: usize,
    pub existing_new: usize,
    pub input: Input,
    pub confirm: Input,
    pub step: RenameStep,
}

pub struct App {
    pub data: Option<AwardsData>,
    pub synced_at: Option<String>,
    pub results_username: Option<String>,
    pub results: Vec<Award>,
    pub duplicates: Vec<Award>,
    pub visible: Vec<VisibleAward>,
    pub username: Input,
    pub status: String,
    pub busy: bool,
    pub loading: bool,
    pub focus: FocusArea,
    pub active_tab: AwardTab,
    pub actions_state: ListState,
    pub awards_state: ListState,
    pub modal: Option<Modal>,
    pub should_quit: bool,
    pub config: AppConfig,
    tx: Sender<WorkerMsg>,
    pending_delete: Option<Award>,
    reconcile_gen: u64,
    /// The Audit modal, stashed while the clerk works a fix opened from a selected
    /// finding (`Some` iff `modal` currently holds the Edit/Delete/Rename dialog that
    /// finding opened). Restored on Esc or on a failed write; discarded (replaced by a
    /// freshly recomputed audit) on a successful write.
    saved_audit: Option<AuditModal>,
    /// Set for the duration of a write that was launched from a selected audit
    /// finding, so `handle_write_done` can route it through the audit-refresh path
    /// instead of the normal Lookup-pane update path.
    audit_fix_username: Option<String>,
}

impl App {
    pub fn new(tx: Sender<WorkerMsg>) -> Self {
        let mut actions_state = ListState::default();
        actions_state.select(Some(0));
        let config = AppConfig::load();
        let mut status = "Loading awards from Google Sheets...".to_string();
        if let Some(path) = &config.loaded_from {
            status = format!("Loading… · config {}", path.display());
        }

        Self {
            data: None,
            synced_at: None,
            results_username: None,
            results: Vec::new(),
            duplicates: Vec::new(),
            visible: Vec::new(),
            username: Input::default(),
            status,
            busy: false,
            loading: false,
            focus: FocusArea::Username,
            active_tab: AwardTab::All,
            actions_state,
            awards_state: ListState::default(),
            modal: None,
            should_quit: false,
            config,
            tx,
            pending_delete: None,
            reconcile_gen: 0,
            saved_audit: None,
            audit_fix_username: None,
        }
    }

    pub fn actions(&self) -> &'static [Action] {
        ACTIONS
    }

    pub fn start_sync(&mut self) {
        self.loading = true;
        self.status = "Syncing Badges / Ribbons / Foreign Awards...".to_string();
        let tx = self.tx.clone();
        thread::spawn(move || {
            let result = build_awards_data(None).map_err(|err| err.to_string());
            let _ = tx.send(WorkerMsg::SyncDone(result));
        });
    }

    pub fn handle_worker_msg(&mut self, msg: WorkerMsg) {
        match msg {
            WorkerMsg::SyncDone(result) => self.handle_sync_done(result),
            WorkerMsg::RowsFixed {
                gen,
                username,
                results,
                duplicates,
            } => self.handle_rows_fixed(gen, username, results, duplicates),
            WorkerMsg::WriteDone {
                kind,
                result,
                username,
            } => self.handle_write_done(kind, result, username),
            WorkerMsg::AuditDone(result) => {
                self.busy = false;
                match result {
                    Ok(outcome) => {
                        self.status = outcome.summary.clone();
                        self.modal = Some(Modal::Audit(AuditModal::new(
                            outcome.path,
                            &outcome.body,
                            outcome.findings,
                        )));
                    }
                    Err(message) => {
                        self.status = format!("Audit failed: {message}");
                    }
                }
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Release {
            return;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('q') {
            self.should_quit = true;
            return;
        }

        if self.modal.is_some() {
            self.handle_modal_key(key);
            return;
        }

        if key.code == KeyCode::F(5)
            || (key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('r'))
        {
            self.action_refresh();
            return;
        }

        match key.code {
            KeyCode::Tab => self.cycle_focus(1),
            KeyCode::BackTab => self.cycle_focus(-1),
            KeyCode::Enter if self.focus == FocusArea::Username => self.action_lookup(),
            KeyCode::Char('a') if self.focus != FocusArea::Username => self.action_add(),
            KeyCode::Char('p') if self.focus != FocusArea::Username => self.action_paste_add(),
            KeyCode::Char('e') if self.focus != FocusArea::Username => self.action_edit(),
            KeyCode::Char('d') if self.focus != FocusArea::Username => self.action_delete(),
            KeyCode::Char('n') if self.focus != FocusArea::Username => self.action_rename(),
            KeyCode::Char('c') if self.focus != FocusArea::Username => self.action_assist(),
            KeyCode::Enter if self.focus == FocusArea::Detail => self.action_edit(),
            _ => match self.focus {
                FocusArea::Username => {
                    let event = CrosstermEvent::Key(key);
                    self.username.handle_event(&event);
                }
                FocusArea::Actions => self.handle_actions_key(key),
                FocusArea::Awards => self.handle_awards_key(key),
                FocusArea::Detail => self.handle_detail_key(key),
            },
        }
    }

    pub fn selected_award(&self) -> Option<&Award> {
        let selected = self.awards_state.selected()?;
        self.visible.get(selected).map(|row| &row.award)
    }

    pub fn selected_action(&self) -> Action {
        let selected = self
            .actions_state
            .selected()
            .filter(|idx| *idx < ACTIONS.len())
            .unwrap_or(0);
        ACTIONS[selected]
    }

    fn handle_sync_done(&mut self, result: Result<AwardsData, String>) {
        self.loading = false;
        match result {
            Ok(data) => {
                let synced = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
                let user_count = data.index.len();
                self.data = Some(data);
                self.synced_at = Some(synced);
                let ready = format!("Ready · {user_count} users · {}", auth_note());
                if let Some(username) = self.results_username.clone() {
                    self.apply_user_view(&username, None, Some(ready));
                } else {
                    self.status = ready;
                }
            }
            Err(err) => {
                self.status = format!("Sync failed: {err}");
            }
        }
    }

    fn handle_rows_fixed(
        &mut self,
        gen: u64,
        username: String,
        results: Vec<Award>,
        duplicates: Vec<Award>,
    ) {
        if gen != self.reconcile_gen {
            return;
        }
        if self.results_username.as_deref() != Some(username.as_str()) {
            return;
        }
        self.results = results;
        self.duplicates = duplicates;
        self.refresh_visible(None);
    }

    fn handle_write_done(&mut self, kind: &'static str, result: EditResult, username: String) {
        self.busy = false;
        if self.audit_fix_username.take().is_some() {
            self.handle_audit_write_done(kind, result);
            return;
        }
        if !result.ok {
            if kind == "rename" && !result.awards.is_empty() {
                // Partial batch write: apply cells that landed, then surface the error.
                let message = format!("rename failed: {}", result.message);
                self.apply_rename_result(
                    EditResult {
                        ok: false,
                        message,
                        error: result.error,
                        award: result.award,
                        awards: result.awards,
                    },
                    username,
                );
                return;
            }
            self.status = format!("{kind} failed: {}", result.message);
            return;
        }

        match kind {
            "add" => self.apply_add_result(result, username),
            "edit" => self.apply_edit_result(result, username),
            "delete" => self.apply_delete_result(result, username),
            "rename" => self.apply_rename_result(result, username),
            _ => {
                self.status = result.message;
            }
        }
    }

    fn apply_add_result(&mut self, result: EditResult, username: String) {
        if let (Some(data), Some(award)) = (self.data.as_mut(), result.award.as_ref()) {
            upsert_award_in_index(&mut data.index, award);
            patch_sheet_cell(data, award);
            self.apply_user_view(&username, Some(award), Some(result.message));
        } else {
            self.status = result.message;
        }
    }

    fn apply_edit_result(&mut self, result: EditResult, username: String) {
        if let (Some(data), Some(award)) = (self.data.as_mut(), result.award.as_ref()) {
            upsert_award_in_index(&mut data.index, award);
            patch_sheet_cell(data, award);
            let new_key = normalize_username(Some(&award.cell));
            if let Some(viewed) = self.results_username.clone() {
                if new_key.as_deref() != Some(viewed.as_str()) {
                    self.apply_user_view(
                        &viewed,
                        None,
                        Some(format!("{} · no longer under @{viewed}", result.message)),
                    );
                    return;
                }
            }
            self.apply_user_view(&username, Some(award), Some(result.message));
        } else {
            self.status = result.message;
        }
    }

    fn apply_delete_result(&mut self, result: EditResult, username: String) {
        let Some(award) = self.pending_delete.take() else {
            self.status = result.message;
            return;
        };
        if let Some(data) = self.data.as_mut() {
            reindex_column_after_delete(&mut data.index, &award.sheet, &award.col, award.row);
            if let Some(rows) = data.sheet_rows.get_mut(&award.sheet) {
                shift_column_up_in_rows(rows, &award.sheet, &award.col, award.row);
            }
            self.apply_user_view(&username, None, Some(result.message));
        } else {
            self.status = result.message;
        }
    }

    fn apply_rename_result(&mut self, result: EditResult, username: String) {
        if let Some(data) = self.data.as_mut() {
            for award in &result.awards {
                upsert_award_in_index(&mut data.index, award);
                patch_sheet_cell(data, award);
            }
            self.username = input_with_value(username.clone());
            // On partial failure we keep viewing the old username; landed awards
            // moved to the new key, so don't try to select one of them here.
            let select = if result.ok {
                result.awards.first()
            } else {
                None
            };
            self.apply_user_view(&username, select, Some(result.message));
        } else {
            self.status = result.message;
        }
    }

    /// Handles the outcome of a write launched from a selected audit finding: on
    /// success, locally patches `data` the same way the normal `apply_*_result` paths
    /// do, then recomputes the audit findings from that freshly patched local data
    /// (no network round-trip needed — `patch_sheet_cell` et al. already keep `data`
    /// current) and reopens the Audit modal on the refreshed list, so a resolved
    /// finding drops off. On failure — including a stale-write rejection — `data` is
    /// left untouched and the stashed Audit modal is restored unchanged, so the list
    /// the clerk was looking at does not silently drift from what is on the sheet.
    fn handle_audit_write_done(&mut self, kind: &'static str, result: EditResult) {
        let saved_path = self.saved_audit.as_ref().map(|modal| modal.path.clone());

        if !result.ok {
            self.status = format!("{kind} failed: {}", result.message);
            if kind == "rename" && !result.awards.is_empty() {
                // Partial batch write: apply the cells that landed before restoring,
                // same as the normal (non-audit) rename path does.
                if let Some(data) = self.data.as_mut() {
                    for award in &result.awards {
                        upsert_award_in_index(&mut data.index, award);
                        patch_sheet_cell(data, award);
                    }
                }
            }
            self.pending_delete = None;
            if let Some(saved) = self.saved_audit.take() {
                self.modal = Some(Modal::Audit(saved));
            }
            return;
        }

        if let Some(data) = self.data.as_mut() {
            match kind {
                "edit" => {
                    if let Some(award) = result.award.as_ref() {
                        upsert_award_in_index(&mut data.index, award);
                        patch_sheet_cell(data, award);
                    }
                }
                "delete" => {
                    if let Some(award) = self.pending_delete.take() {
                        reindex_column_after_delete(&mut data.index, &award.sheet, &award.col, award.row);
                        if let Some(rows) = data.sheet_rows.get_mut(&award.sheet) {
                            shift_column_up_in_rows(rows, &award.sheet, &award.col, award.row);
                        }
                    }
                }
                "rename" => {
                    for award in &result.awards {
                        upsert_award_in_index(&mut data.index, award);
                        patch_sheet_cell(data, award);
                    }
                }
                _ => {}
            }
        }
        self.status = result.message;
        self.saved_audit = None;
        self.reopen_audit_from_local_data(saved_path.unwrap_or_default());
    }

    /// Recomputes the audit report from the current in-memory `data` (already patched
    /// by the caller) and reopens `Modal::Audit` on it, defaulted to the findings-list
    /// view. Used after a fix applied from a selected finding, so resolved findings
    /// drop off without a network round-trip.
    fn reopen_audit_from_local_data(&mut self, path: String) {
        let Some(data) = self.data.clone() else {
            return;
        };
        let report = collect_sheet_audit(&data);
        let findings = flatten_audit_findings(&report);
        let generated = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
        let body = format_audit_report(&report, &generated);
        self.modal = Some(Modal::Audit(AuditModal::new(path, &body, findings)));
    }

    fn handle_actions_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => self.move_action(-1),
            KeyCode::Down => self.move_action(1),
            KeyCode::Enter => self.activate_action(self.selected_action()),
            _ => {}
        }
    }

    fn handle_awards_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => self.move_award(-1),
            KeyCode::Down => self.move_award(1),
            KeyCode::Char('[') => self.move_tab(-1),
            KeyCode::Char(']') => self.move_tab(1),
            KeyCode::Char(ch @ '1'..='5') => {
                let idx = ch as usize - '1' as usize;
                if let Some(tab) = AwardTab::ALL.get(idx).copied() {
                    self.active_tab = tab;
                    self.refresh_visible(None);
                }
            }
            KeyCode::Enter => self.action_edit(),
            _ => {}
        }
    }

    fn handle_detail_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter | KeyCode::Char('e') => self.action_edit(),
            KeyCode::Char('d') => self.action_delete(),
            KeyCode::Up => self.move_award(-1),
            KeyCode::Down => self.move_award(1),
            _ => {}
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Esc {
            if let Some(Modal::Audit(audit)) = self.modal.as_mut() {
                if matches!(audit.view, AuditView::ChooseUsername { .. }) {
                    // Back out of the two-username sub-dialog without closing the
                    // whole Audit browser.
                    audit.view = AuditView::List;
                    return;
                }
                self.modal = None;
                // Closing the audit browser must not stomp the summary line left by
                // the audit run (or by a just-applied fix).
                return;
            }
            if self.saved_audit.is_some()
                && matches!(
                    self.modal,
                    Some(Modal::Edit(_)) | Some(Modal::Delete(_)) | Some(Modal::Rename(_))
                )
            {
                // Cancelling a fix that was opened from a selected finding returns to
                // the audit list rather than closing everything.
                self.audit_fix_username = None;
                self.pending_delete = None;
                self.modal = self.saved_audit.take().map(Modal::Audit);
                self.status = "Fix cancelled".to_string();
                return;
            }
            self.modal = None;
            self.status = "Dialog cancelled".to_string();
            return;
        }

        if matches!(self.modal, Some(Modal::Audit(_))) {
            self.handle_audit_modal_key(key);
            return;
        }

        let mut add_filter_changed = false;
        let mut add_confirm: Option<(AwardDef, String)> = None;
        let mut edit_confirm: Option<(Award, String)> = None;
        let mut delete_confirm: Option<Award> = None;
        let mut rename_confirm: Option<String> = None;
        let mut assist_run = false;
        let mut assist_grant = false;
        let mut paste_submit = false;
        let mut status: Option<String> = None;

        if let Some(modal) = self.modal.as_mut() {
            match modal {
                Modal::PasteAdd(paste) => match key.code {
                    KeyCode::Enter => {
                        paste_submit = true;
                    }
                    KeyCode::Backspace => {
                        paste.buffer.pop();
                    }
                    KeyCode::Char(ch) => {
                        paste.buffer.push(ch);
                    }
                    _ => {}
                },
                Modal::Add(add) => match add.step {
                    AddStep::Pick => match key.code {
                        KeyCode::Up => move_list(&mut add.state, add.filtered.len(), -1),
                        KeyCode::Down => move_list(&mut add.state, add.filtered.len(), 1),
                        KeyCode::Enter => {
                            let idx = add.state.selected().unwrap_or(0);
                            if let Some(chosen) = add.filtered.get(idx).cloned() {
                                add.chosen = Some(chosen);
                                add.step = AddStep::Suffix;
                            } else {
                                status = Some("Select an award first".to_string());
                            }
                        }
                        _ => {
                            let event = CrosstermEvent::Key(key);
                            add.filter.handle_event(&event);
                            add_filter_changed = true;
                        }
                    },
                    AddStep::Suffix => match key.code {
                        KeyCode::Enter => {
                            if let Some(chosen) = add.chosen.clone() {
                                add_confirm = Some((chosen, add.suffix.value().trim().to_string()));
                            }
                        }
                        _ => {
                            let event = CrosstermEvent::Key(key);
                            add.suffix.handle_event(&event);
                        }
                    },
                },
                Modal::Edit(edit) => match key.code {
                    KeyCode::Enter => {
                        edit_confirm =
                            Some((edit.award.clone(), edit.input.value().trim().to_string()));
                    }
                    _ => {
                        let event = CrosstermEvent::Key(key);
                        edit.input.handle_event(&event);
                    }
                },
                Modal::Delete(delete) => match key.code {
                    KeyCode::Enter => {
                        if delete.input.value().trim().eq_ignore_ascii_case("delete") {
                            delete_confirm = Some(delete.award.clone());
                        } else {
                            status = Some("Type \"delete\" to confirm".to_string());
                        }
                    }
                    _ => {
                        let event = CrosstermEvent::Key(key);
                        delete.input.handle_event(&event);
                    }
                },
                Modal::Rename(rename) => match rename.step {
                    RenameStep::Name => match key.code {
                        KeyCode::Enter => {
                            let raw = rename.input.value().trim();
                            if let Some(display_new) = parse_bare_username(raw) {
                                let new_key = display_new.to_ascii_lowercase();
                                if new_key == rename.from {
                                    status = Some(
                                        "New username is the same as the current name".to_string(),
                                    );
                                } else {
                                    rename.step = RenameStep::Confirm;
                                }
                            } else {
                                status = Some(
                                    "Enter a bare Roblox username (no suffixes)".to_string(),
                                );
                            }
                        }
                        _ => {
                            let event = CrosstermEvent::Key(key);
                            rename.input.handle_event(&event);
                        }
                    },
                    RenameStep::Confirm => match key.code {
                        KeyCode::Enter => {
                            if rename.confirm.value().trim().eq_ignore_ascii_case("rename") {
                                rename_confirm = Some(rename.input.value().trim().to_string());
                            } else {
                                status = Some("Type \"rename\" to confirm".to_string());
                            }
                        }
                        _ => {
                            let event = CrosstermEvent::Key(key);
                            rename.confirm.handle_event(&event);
                        }
                    },
                },
                Modal::Assist(assist) => match assist.step {
                    AssistStep::Query => match key.code {
                        KeyCode::Enter => {
                            assist_run = true;
                        }
                        _ => {
                            let event = CrosstermEvent::Key(key);
                            assist.input.handle_event(&event);
                        }
                    },
                    AssistStep::Result => match key.code {
                        KeyCode::Enter if assist.can_grant => {
                            assist_grant = true;
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            assist.scroll = assist.scroll.saturating_sub(1);
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            assist.scroll = assist.scroll.saturating_add(1);
                        }
                        _ => {}
                    },
                },
                Modal::Audit(_) => {}
            }
        }

        if add_filter_changed {
            if let Some(Modal::Add(add)) = self.modal.as_mut() {
                add.reload();
            }
        }
        if let Some(Modal::Rename(rename)) = self.modal.as_mut() {
            if matches!(rename.step, RenameStep::Confirm) {
                let new_key = parse_bare_username(rename.input.value())
                    .map(|name| name.to_ascii_lowercase())
                    .unwrap_or_default();
                rename.existing_new = self
                    .data
                    .as_ref()
                    .map(|data| get_awards_for_username(&data.index, &new_key).len())
                    .unwrap_or(0);
            }
        }
        if let Some(message) = status {
            self.status = message;
        }
        if let Some((award_def, suffix)) = add_confirm {
            self.modal = None;
            self.commit_add(award_def, suffix);
        }
        if let Some((award, new_cell)) = edit_confirm {
            self.modal = None;
            self.commit_edit(award, new_cell);
        }
        if let Some(award) = delete_confirm {
            self.modal = None;
            self.commit_delete(award);
        }
        if let Some(new_username) = rename_confirm {
            self.modal = None;
            self.commit_rename(new_username);
        }
        if assist_run {
            self.run_assist_check();
        }
        if assist_grant {
            self.commit_assist_grant();
        }
        if paste_submit {
            self.submit_paste_add();
        }
    }

    /// Handle a bracketed-paste event (`crossterm::event::Event::Paste`) delivered from the run
    /// loop while the Discord-paste modal is open (003-discord-paste-quick-add, research.md §1).
    /// Ignored when that modal isn't open — a paste elsewhere in the app has no meaning here.
    pub fn handle_paste(&mut self, text: String) {
        if let Some(Modal::PasteAdd(paste)) = self.modal.as_mut() {
            paste.buffer.push_str(&text);
        }
    }

    /// Resolve the pasted text in an open `Modal::PasteAdd` into either a pre-filled `Modal::Add`
    /// (a confident single catalog match, or a filtered picker otherwise) or an inline error that
    /// keeps the paste buffer around for the clerk to correct — see
    /// `contracts/tui-paste-interaction.md` "Submit behavior".
    fn submit_paste_add(&mut self) {
        let Some(Modal::PasteAdd(paste)) = self.modal.as_ref() else {
            return;
        };
        let extracted = extract_paste_fields(&paste.buffer);

        let Some(username) = extracted.username else {
            if let Some(Modal::PasteAdd(paste)) = self.modal.as_mut() {
                paste.error = Some(
                    "Couldn't find a username in that text — check it and try again, or Esc to cancel"
                        .to_string(),
                );
            }
            return;
        };
        if self.data.is_none() {
            if let Some(Modal::PasteAdd(paste)) = self.modal.as_mut() {
                paste.error = Some("Still loading awards...".to_string());
            }
            return;
        }

        // Local, in-memory lookup — no network round trip (research.md §7), same path
        // `action_lookup` already uses.
        self.apply_user_view(&username, None, None);

        let Some(data) = self.data.as_ref() else {
            return;
        };
        let owned_source: Vec<Award> = self
            .results
            .iter()
            .chain(self.duplicates.iter())
            .cloned()
            .collect();
        let owned = owned_award_columns(&owned_source, &username);
        let candidates: Vec<AwardDef> = data
            .catalog
            .iter()
            .filter(|def| !owned.contains(&(def.sheet.clone(), def.col.clone())))
            .cloned()
            .collect();
        if candidates.is_empty() {
            self.modal = None;
            self.status = "No remaining awards to add for this user".to_string();
            return;
        }

        let Some(award_text) = extracted.award_text else {
            // No award line at all: still land on the picker so what WAS extracted (the
            // username) carries through, per spec User Story 2.
            self.modal = Some(Modal::Add(AddModal::new(candidates)));
            return;
        };

        let (base_query, suffix) = split_award_suffix(&award_text);
        let matches = match_catalog_entries(&candidates, &base_query);
        if matches.len() == 1 {
            let chosen = matches.into_iter().next().expect("len checked above");
            let mut add = AddModal::new(candidates);
            add.chosen = Some(chosen);
            add.step = AddStep::Suffix;
            add.suffix = input_with_value(suffix);
            self.modal = Some(Modal::Add(add));
            return;
        }

        // Zero or more than one match: fall back to the searchable picker, pre-filtered with
        // the extracted text rather than an auto-confirmed guess (spec User Story 2, AC1).
        let mut add = AddModal::new(candidates);
        add.filter = input_with_value(award_text);
        add.reload();
        self.modal = Some(Modal::Add(add));
    }

    /// Key handling while the Audit modal is open: navigation and view-toggle in both
    /// the findings-list and report views, and picking a username in the transient
    /// `ChooseUsername` sub-dialog. Esc is handled by the caller before this is
    /// reached.
    fn handle_audit_modal_key(&mut self, key: KeyEvent) {
        let mut open_finding: Option<AuditFinding> = None;
        let mut choose_username: Option<String> = None;

        if let Some(Modal::Audit(audit)) = self.modal.as_mut() {
            match audit.view.clone() {
                AuditView::Report => match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        audit.scroll = audit.scroll.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        audit.scroll = audit.scroll.saturating_add(1);
                    }
                    KeyCode::PageUp => {
                        audit.scroll = audit.scroll.saturating_sub(10);
                    }
                    KeyCode::PageDown => {
                        audit.scroll = audit.scroll.saturating_add(10);
                    }
                    KeyCode::Home => audit.scroll = 0,
                    KeyCode::End => {
                        audit.scroll = audit.lines.len().saturating_sub(1) as u16;
                    }
                    KeyCode::Tab => audit.view = AuditView::List,
                    _ => {}
                },
                AuditView::List => match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        move_audit_row(&audit.list, &mut audit.list_state, -1)
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        move_audit_row(&audit.list, &mut audit.list_state, 1)
                    }
                    KeyCode::PageUp => move_audit_row(&audit.list, &mut audit.list_state, -5),
                    KeyCode::PageDown => move_audit_row(&audit.list, &mut audit.list_state, 5),
                    KeyCode::Tab => audit.view = AuditView::Report,
                    KeyCode::Enter => {
                        if let Some(row) = audit.list_state.selected() {
                            open_finding = audit.list.finding_at_row(row).cloned();
                        }
                    }
                    _ => {}
                },
                AuditView::ChooseUsername { a, b } => match key.code {
                    KeyCode::Char('1') => choose_username = Some(a),
                    KeyCode::Char('2') => choose_username = Some(b),
                    _ => {}
                },
            }
        }

        if let Some(finding) = open_finding {
            self.open_fix_for_finding(finding);
        } else if let Some(from) = choose_username {
            self.open_rename_for_finding(from);
        }
    }

    /// Selecting a finding and pressing Enter jumps straight into the existing fix
    /// flow for that award/cell: Delete for a duplicate row, Edit for a malformed
    /// cell, and (via `open_rename_for_finding`) Rename for similar usernames. The
    /// current Audit modal is stashed in `saved_audit` so Esc or a failed write can
    /// return to it unchanged, and `audit_fix_username` is set so `handle_write_done`
    /// routes the result back through the audit-refresh path instead of the normal
    /// Lookup-pane update.
    fn open_fix_for_finding(&mut self, finding: AuditFinding) {
        match &finding {
            AuditFinding::DuplicateRow { .. } => {
                let Some(award) = finding.to_award() else {
                    return;
                };
                let Some(Modal::Audit(audit)) = self.modal.take() else {
                    return;
                };
                let viewed_username = finding_username(&finding).unwrap_or("?").to_string();
                self.audit_fix_username = Some(viewed_username.clone());
                self.saved_audit = Some(audit);
                self.status = "Type \"delete\" to remove this duplicate row".to_string();
                self.modal = Some(Modal::Delete(DeleteModal {
                    award,
                    input: Input::default(),
                    viewed_username,
                }));
            }
            AuditFinding::MalformedCell { .. } => {
                let Some(award) = finding.to_award() else {
                    return;
                };
                let Some(Modal::Audit(audit)) = self.modal.take() else {
                    return;
                };
                let value = if award.cell.is_empty() {
                    award.name.clone()
                } else {
                    award.cell.clone()
                };
                self.audit_fix_username = Some(finding_username(&finding).unwrap_or("?").to_string());
                self.saved_audit = Some(audit);
                self.status = "Fix the malformed cell, then Enter to save".to_string();
                self.modal = Some(Modal::Edit(EditModal {
                    award,
                    input: input_with_value(value),
                }));
            }
            AuditFinding::SimilarUsernames { a, b, .. } => {
                if let Some(Modal::Audit(audit)) = self.modal.as_mut() {
                    audit.view = AuditView::ChooseUsername {
                        a: a.clone(),
                        b: b.clone(),
                    };
                }
                self.status = "Which account is the typo? Press 1 or 2".to_string();
            }
            AuditFinding::UnparseableCell { .. } => {
                self.status =
                    "No direct fix available for this cell — edit it manually on the sheet"
                        .to_string();
            }
        }
    }

    /// Completes the `SimilarUsernames` two-username choice: opens the standard
    /// Rename dialog targeting the chosen account, stashing the Audit modal the same
    /// way `open_fix_for_finding` does. If the chosen account no longer owns any
    /// sheet cells (e.g. a prior fix already resolved it), the audit list is restored
    /// unchanged and nothing opens.
    fn open_rename_for_finding(&mut self, from: String) {
        let Some(Modal::Audit(mut audit)) = self.modal.take() else {
            return;
        };
        // The caller only reaches here from `AuditView::ChooseUsername` (see
        // `open_fix_for_finding`'s `SimilarUsernames` arm). Reset back to the findings
        // list before this modal is stashed or restored below, so Esc from the Rename
        // modal (or a failed rename write) never reopens the Audit modal frozen on the
        // two-username choice — see constitution Principle III.
        audit.view = AuditView::List;
        let cell_count = self
            .data
            .as_ref()
            .map(|data| {
                get_awards_for_username(&data.index, &from)
                    .iter()
                    .filter(|award| {
                        !award.sheet.is_empty() && !award.col.is_empty() && award.row != 0
                    })
                    .count()
            })
            .unwrap_or(0);
        if cell_count == 0 {
            self.status = format!("No sheet cells found for @{from}");
            self.modal = Some(Modal::Audit(audit));
            return;
        }
        self.audit_fix_username = Some(from.clone());
        self.saved_audit = Some(audit);
        self.modal = Some(Modal::Rename(RenameModal {
            from,
            cell_count,
            existing_new: 0,
            input: Input::default(),
            confirm: Input::default(),
            step: RenameStep::Name,
        }));
    }

    fn cycle_focus(&mut self, delta: isize) {
        let current = match self.focus {
            FocusArea::Username => 0,
            FocusArea::Actions => 1,
            FocusArea::Awards => 2,
            FocusArea::Detail => 3,
        };
        let next = (current as isize + delta).rem_euclid(4) as usize;
        self.focus = match next {
            0 => FocusArea::Username,
            1 => FocusArea::Actions,
            2 => FocusArea::Awards,
            _ => FocusArea::Detail,
        };
    }

    fn move_action(&mut self, delta: isize) {
        move_list(&mut self.actions_state, ACTIONS.len(), delta);
    }

    fn move_award(&mut self, delta: isize) {
        move_list(&mut self.awards_state, self.visible.len(), delta);
    }

    fn move_tab(&mut self, delta: isize) {
        let current = AwardTab::ALL
            .iter()
            .position(|tab| *tab == self.active_tab)
            .unwrap_or(0);
        let next = (current as isize + delta).rem_euclid(AwardTab::ALL.len() as isize) as usize;
        if let Some(tab) = AwardTab::ALL.get(next).copied() {
            self.active_tab = tab;
            self.refresh_visible(None);
        }
    }

    fn activate_action(&mut self, action: Action) {
        match action {
            Action::Lookup => self.action_lookup(),
            Action::Add => self.action_add(),
            Action::PasteAdd => self.action_paste_add(),
            Action::Edit => self.action_edit(),
            Action::Delete => self.action_delete(),
            Action::Rename => self.action_rename(),
            Action::Assist => self.action_assist(),
            Action::Refresh => self.action_refresh(),
            Action::Audit => self.action_audit(),
        }
    }

    fn action_lookup(&mut self) {
        let raw = self.username.value();
        let username = normalize_username(Some(raw))
            .unwrap_or_else(|| raw.trim().trim_start_matches('@').to_ascii_lowercase());
        if username.is_empty() {
            self.status = "Enter a username".to_string();
            return;
        }
        if self.data.is_none() {
            self.status = "Still loading awards...".to_string();
            return;
        }
        self.apply_user_view(&username, None, None);
        if !self.visible.is_empty() {
            self.focus = FocusArea::Awards;
        }
    }

    fn action_refresh(&mut self) {
        if self.busy || self.loading || self.modal.is_some() {
            self.status = "Wait for the current sheet operation to finish".to_string();
            return;
        }
        self.start_sync();
    }

    fn action_audit(&mut self) {
        if !self.begin_busy("Running duplicate audit...") {
            return;
        }
        let data = self.data.clone();
        let tx = self.tx.clone();
        thread::spawn(move || {
            let result = run_audit_worker(data);
            let _ = tx.send(WorkerMsg::AuditDone(result));
        });
    }

    fn action_add(&mut self) {
        if !self.begin_dialog() {
            return;
        }
        let Some(username) = self.results_username.as_deref() else {
            self.modal = None;
            self.status = "Look up a user before adding awards".to_string();
            return;
        };
        let Some(data) = self.data.as_ref() else {
            self.modal = None;
            self.status = "Still loading...".to_string();
            return;
        };

        let owned_source: Vec<Award> = self
            .results
            .iter()
            .chain(self.duplicates.iter())
            .cloned()
            .collect();
        let owned = owned_award_columns(&owned_source, username);
        let candidates: Vec<AwardDef> = data
            .catalog
            .iter()
            .filter(|def| !owned.contains(&(def.sheet.clone(), def.col.clone())))
            .cloned()
            .collect();
        if candidates.is_empty() {
            self.modal = None;
            self.status = "No remaining awards to add for this user".to_string();
            return;
        }
        self.modal = Some(Modal::Add(AddModal::new(candidates)));
    }

    /// Open the Discord-paste entry point (003-discord-paste-quick-add). Unlike `action_add`,
    /// this does not require a username already looked up — that's the entire point.
    fn action_paste_add(&mut self) {
        if !self.begin_dialog() {
            return;
        }
        self.modal = Some(Modal::PasteAdd(PasteAddModal::default()));
    }

    fn action_edit(&mut self) {
        if !self.begin_dialog() {
            return;
        }
        let Some(award) = self.selected_award().cloned() else {
            self.modal = None;
            self.status = "Select an award to edit".to_string();
            return;
        };
        let value = if award.cell.is_empty() {
            award.name.clone()
        } else {
            award.cell.clone()
        };
        self.modal = Some(Modal::Edit(EditModal {
            award,
            input: input_with_value(value),
        }));
    }

    fn action_delete(&mut self) {
        if !self.begin_dialog() {
            return;
        }
        let Some(award) = self.selected_award().cloned() else {
            self.modal = None;
            self.status = "Select an award to delete".to_string();
            return;
        };
        let viewed_username = self
            .results_username
            .clone()
            .unwrap_or_else(|| "?".to_string());
        self.modal = Some(Modal::Delete(DeleteModal {
            award,
            input: Input::default(),
            viewed_username,
        }));
    }

    fn action_rename(&mut self) {
        if !self.begin_dialog() {
            return;
        }
        let Some(from) = self.results_username.clone() else {
            self.modal = None;
            self.status = "Look up a user before renaming".to_string();
            return;
        };
        let Some(data) = self.data.as_ref() else {
            self.modal = None;
            self.status = "Still loading...".to_string();
            return;
        };
        let owned: Vec<Award> = self
            .results
            .iter()
            .chain(self.duplicates.iter())
            .filter(|award| {
                normalize_username(Some(&award.cell)).as_deref() == Some(from.as_str())
                    && !award.sheet.is_empty()
                    && !award.col.is_empty()
                    && award.row != 0
            })
            .cloned()
            .collect();
        let cell_count = if owned.is_empty() {
            get_awards_for_username(&data.index, &from)
                .iter()
                .filter(|award| !award.sheet.is_empty() && !award.col.is_empty() && award.row != 0)
                .count()
        } else {
            owned.len()
        };
        if cell_count == 0 {
            self.modal = None;
            self.status = format!("No sheet cells found for @{from}");
            return;
        }
        self.modal = Some(Modal::Rename(RenameModal {
            from,
            cell_count,
            existing_new: 0,
            input: Input::default(),
            confirm: Input::default(),
            step: RenameStep::Name,
        }));
    }

    fn action_assist(&mut self) {
        if !self.begin_dialog() {
            return;
        }
        let Some(username) = self.results_username.clone() else {
            self.modal = None;
            self.status = "Look up a user before Assist".to_string();
            return;
        };
        if self.data.is_none() {
            self.modal = None;
            self.status = "Still loading...".to_string();
            return;
        }
        self.modal = Some(Modal::Assist(AssistModal {
            username,
            input: input_with_value("MCAB".to_string()),
            step: AssistStep::Query,
            report: String::new(),
            can_grant: false,
            grant: None,
            scroll: 0,
        }));
    }

    fn assist_awards_for_user(&self, username: &str) -> Vec<Award> {
        self.data
            .as_ref()
            .map(|data| get_awards_for_username(&data.index, username))
            .unwrap_or_default()
    }

    fn run_assist_check(&mut self) {
        let Some(Modal::Assist(assist)) = self.modal.as_mut() else {
            return;
        };
        let query = assist.input.value().trim().to_string();
        if query.is_empty() {
            self.status = "Enter an award request (MCAB / MCIB / MCMB)".to_string();
            return;
        }
        let username = assist.username.clone();
        // Index-only: never mix similar-username duplicate rows into Assist.
        let awards = self.assist_awards_for_user(&username);
        let verdict = check_assist(&username, &awards, &query);
        let report = verdict.format_report(&username);
        let (can_grant, grant) = match &verdict {
            AssistVerdict::Approve { grant, .. } => (true, Some(grant.clone())),
            _ => (false, None),
        };
        if let Some(Modal::Assist(assist)) = self.modal.as_mut() {
            assist.report = report;
            assist.can_grant = can_grant;
            assist.grant = grant;
            assist.step = AssistStep::Result;
            assist.scroll = 0;
        }
        self.status = if can_grant {
            "Eligible — Enter to grant on sheet, Esc to cancel".to_string()
        } else {
            "Assist result — Esc to close".to_string()
        };
    }

    fn commit_assist_grant(&mut self) {
        let Some(Modal::Assist(assist)) = self.modal.as_mut() else {
            return;
        };
        let username = assist.username.clone();
        let query = assist.input.value().trim().to_string();
        if query.is_empty() {
            self.status = "Nothing to grant".to_string();
            return;
        }
        // Re-check on current index snapshot (closes TOCTOU vs cached Approve).
        let awards = self.assist_awards_for_user(&username);
        let verdict = check_assist(&username, &awards, &query);
        match verdict {
            AssistVerdict::Approve { grant, .. } => {
                let GrantPlan::UpgradeCell { new_cell, .. } = &grant;
                let Some(target) = find_grant_target(&awards, &grant, &username).cloned() else {
                    self.modal = None;
                    self.status =
                        "Could not find a sheet row owned by this user to upgrade".to_string();
                    return;
                };
                self.modal = None;
                self.commit_edit(target, new_cell.clone());
            }
            other => {
                let report = other.format_report(&username);
                if let Some(Modal::Assist(assist)) = self.modal.as_mut() {
                    assist.report = report;
                    assist.can_grant = false;
                    assist.grant = None;
                    assist.step = AssistStep::Result;
                    assist.scroll = 0;
                }
                self.status = "No longer eligible — see Assist result".to_string();
            }
        }
    }

    fn commit_add(&mut self, award_def: AwardDef, suffix: String) {
        if !self.begin_busy(&format!("Writing {}...", award_def.base_name)) {
            return;
        }
        let username = self.results_username.clone().unwrap_or_default();
        let tx = self.tx.clone();
        thread::spawn(move || {
            let result = add_award_to_user(&username, &award_def, &suffix, false);
            let _ = tx.send(WorkerMsg::WriteDone {
                kind: "add",
                result,
                username,
            });
        });
    }

    fn commit_edit(&mut self, award: Award, new_cell: String) {
        if !self.begin_busy("Updating sheet...") {
            return;
        }
        let username = self.results_username.clone().unwrap_or_default();
        let tx = self.tx.clone();
        thread::spawn(move || {
            let result = update_award_cell(&award, &new_cell, false);
            let _ = tx.send(WorkerMsg::WriteDone {
                kind: "edit",
                result,
                username,
            });
        });
    }

    fn commit_delete(&mut self, award: Award) {
        if !self.begin_busy(&format!("Removing {}...", award.name)) {
            return;
        }
        self.pending_delete = Some(award.clone());
        let username = self.results_username.clone().unwrap_or_default();
        let tx = self.tx.clone();
        thread::spawn(move || {
            let result = remove_award(&award, false);
            let _ = tx.send(WorkerMsg::WriteDone {
                kind: "delete",
                result,
                username,
            });
        });
    }

    fn commit_rename(&mut self, new_username: String) {
        let old_username = self.results_username.clone().unwrap_or_default();
        if !self.begin_busy(&format!(
            "Renaming @{old_username} → {new_username} across the sheet..."
        )) {
            return;
        }
        let data = self.data.clone();
        let tx = self.tx.clone();
        thread::spawn(move || {
            let result = rename_username(&old_username, &new_username, data.as_ref(), false);
            // On failure (including partial), keep viewing the old username so
            // remaining cells stay visible. Success jumps to the new name.
            let view_user = if result.ok {
                parse_bare_username(&new_username)
                    .map(|name| name.to_ascii_lowercase())
                    .unwrap_or_else(|| {
                        new_username
                            .trim()
                            .trim_start_matches('@')
                            .to_ascii_lowercase()
                    })
            } else {
                old_username
            };
            let _ = tx.send(WorkerMsg::WriteDone {
                kind: "rename",
                result,
                username: view_user,
            });
        });
    }

    fn begin_busy(&mut self, status: &str) -> bool {
        if self.busy || self.loading || self.modal.is_some() {
            self.status = "Wait for the current sheet operation to finish".to_string();
            return false;
        }
        self.busy = true;
        self.status = status.to_string();
        true
    }

    fn begin_dialog(&mut self) -> bool {
        if self.busy || self.loading || self.modal.is_some() {
            self.status = "Wait for the current sheet operation to finish".to_string();
            return false;
        }
        true
    }

    fn apply_user_view(&mut self, username: &str, select: Option<&Award>, status: Option<String>) {
        let Some(data) = self.data.as_ref() else {
            return;
        };
        let awards = flatten_awards_sorted(&get_awards_for_username(&data.index, username));
        let dup_hits = find_duplicates_for_user(data, username);
        let awards = awards_excluding_duplicate_rows(&awards, &dup_hits);
        let duplicates: Vec<Award> = dup_hits.iter().map(|hit| hit.to_award()).collect();
        let dup_note = if duplicates.is_empty() {
            String::new()
        } else {
            format!(" · {} duplicate(s)", duplicates.len())
        };

        self.results_username = Some(username.to_string());
        self.results = awards;
        self.duplicates = duplicates;
        self.refresh_visible(select);
        self.status = status.unwrap_or_else(|| {
            format!(
                "{username} · {} award(s){dup_note} · a/e/d · F5 refresh",
                self.results.len()
            )
        });
        self.spawn_row_reconcile(username.to_string());
    }

    fn refresh_visible(&mut self, select: Option<&Award>) {
        self.visible = self.visible_for_tab();
        let selected = select
            .and_then(|target| {
                self.visible.iter().position(|row| {
                    row.award.sheet == target.sheet
                        && row.award.col == target.col
                        && row.award.row == target.row
                })
            })
            .or_else(|| {
                let current = self.awards_state.selected().unwrap_or(0);
                (current < self.visible.len()).then_some(current)
            });
        self.awards_state.select(selected);
    }

    fn visible_for_tab(&self) -> Vec<VisibleAward> {
        match self.active_tab {
            AwardTab::All => self
                .results
                .iter()
                .cloned()
                .map(|award| VisibleAward {
                    award,
                    warning: false,
                })
                .chain(self.duplicates.iter().cloned().map(|award| VisibleAward {
                    award,
                    warning: true,
                }))
                .collect(),
            AwardTab::Duplicates => self
                .duplicates
                .iter()
                .cloned()
                .map(|award| VisibleAward {
                    award,
                    warning: true,
                })
                .collect(),
            tab => {
                let category = tab.category().unwrap_or_default();
                self.results
                    .iter()
                    .filter(|award| award.category == category)
                    .cloned()
                    .map(|award| VisibleAward {
                        award,
                        warning: false,
                    })
                    .collect()
            }
        }
    }

    fn spawn_row_reconcile(&mut self, username: String) {
        if !matches!(auth_status(), "service_account" | "oauth_token") {
            return;
        }
        self.reconcile_gen = self.reconcile_gen.wrapping_add(1);
        let gen = self.reconcile_gen;
        let results = self.results.clone();
        let duplicates = self.duplicates.clone();
        let tx = self.tx.clone();
        thread::spawn(move || {
            let Ok(api) = SheetsApi::connect(false) else {
                return;
            };
            let fixed_results = resolve_live_rows(&api, &results);
            let fixed_dups = resolve_live_rows(&api, &duplicates);
            if fixed_results != results || fixed_dups != duplicates {
                let _ = tx.send(WorkerMsg::RowsFixed {
                    gen,
                    username,
                    results: fixed_results,
                    duplicates: fixed_dups,
                });
            }
        });
    }
}

impl AddModal {
    fn new(candidates: Vec<AwardDef>) -> Self {
        let mut state = ListState::default();
        if !candidates.is_empty() {
            state.select(Some(0));
        }
        Self {
            filtered: candidates.clone(),
            all_candidates: candidates,
            filter: Input::default(),
            suffix: Input::default(),
            state,
            step: AddStep::Pick,
            chosen: None,
        }
    }

    fn reload(&mut self) {
        // Shared with the paste-to-prefill flow's fallback path (research.md §5,
        // 003-discord-paste-quick-add) so the two call sites can never drift apart.
        self.filtered = match_catalog_entries(&self.all_candidates, self.filter.value());
        if self.filtered.is_empty() {
            self.state.select(None);
        } else {
            let selected = self
                .state
                .selected()
                .filter(|idx| *idx < self.filtered.len())
                .unwrap_or(0);
            self.state.select(Some(selected));
        }
    }
}

pub fn category_label(category: &str) -> &str {
    CATEGORY_LABELS
        .iter()
        .find(|(key, _)| *key == category)
        .map(|(_, label)| *label)
        .unwrap_or(category)
}

fn input_with_value(value: String) -> Input {
    Input::default().with_value(value)
}

fn move_list(state: &mut ListState, len: usize, delta: isize) {
    if len == 0 {
        state.select(None);
        return;
    }
    let current = state.selected().unwrap_or(0).min(len - 1);
    let next = (current as isize + delta).rem_euclid(len as isize) as usize;
    state.select(Some(next));
}

/// Wraparound navigation over an `AuditFindingsList`'s selectable rows only — group
/// header rows are skipped, never selected.
fn move_audit_row(list: &AuditFindingsList, state: &mut ListState, delta: isize) {
    let selectable = list.selectable_rows();
    if selectable.is_empty() {
        state.select(None);
        return;
    }
    let current_row = state.selected().unwrap_or(selectable[0]);
    let current_pos = selectable
        .iter()
        .position(|&row| row == current_row)
        .unwrap_or(0);
    let next_pos =
        (current_pos as isize + delta).rem_euclid(selectable.len() as isize) as usize;
    state.select(Some(selectable[next_pos]));
}

fn auth_note() -> String {
    match auth_status() {
        "service_account" => "write: service account".to_string(),
        "oauth_token" => "write: logged in".to_string(),
        "oauth_needs_login" => "write: run --login".to_string(),
        "missing" => "write: no credentials".to_string(),
        other => format!("write: {other}"),
    }
}

fn run_audit_worker(data: Option<AwardsData>) -> Result<AuditOutcome, String> {
    let data = match data {
        Some(data) => data,
        None => build_awards_data(None).map_err(|err| err.to_string())?,
    };
    let report = collect_sheet_audit(&data);
    let findings = flatten_audit_findings(&report);
    let generated = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
    let body = format_audit_report(&report, &generated);
    let stamp = Utc::now().format("%Y-%m-%d_%H%M%S");
    let dest = project_root()
        .join("audits")
        .join(format!("audit-{stamp}.txt"));
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    fs::write(&dest, &body).map_err(|err| err.to_string())?;

    let identical = report
        .duplicate_groups
        .iter()
        .filter(|group| group.kind == "identical")
        .count();
    let conflict = report
        .duplicate_groups
        .iter()
        .filter(|group| group.kind == "conflict")
        .count();
    let summary = format!(
        "Wrote {} · {identical} identical · {conflict} conflict · {} similar",
        dest.display(),
        report.similar_pairs.len()
    );
    Ok(AuditOutcome {
        path: dest.display().to_string(),
        body,
        summary,
        findings,
    })
}

fn patch_sheet_cell(data: &mut AwardsData, award: &Award) {
    let csv_index = award.row - 1 - row_offset(&award.sheet);
    if csv_index < 0 {
        return;
    }
    let col_idx = col_to_index(&award.col);
    let rows = data.sheet_rows.entry(award.sheet.clone()).or_default();
    let csv_index = csv_index as usize;
    while rows.len() <= csv_index {
        rows.push(Vec::new());
    }
    while rows[csv_index].len() <= col_idx {
        rows[csv_index].push(String::new());
    }
    rows[csv_index][col_idx] = award.cell.clone();
}

fn resolve_live_rows(api: &SheetsApi, awards: &[Award]) -> Vec<Award> {
    awards
        .iter()
        .map(|award| award_with_live_row(api, award, 24).unwrap_or_else(|_| award.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    //! Modal state-machine tests: what a keypress does to `App::modal` / `App::status`
    //! / `App::busy`, without touching the terminal or the network. Writes route through
    //! `commit_*`, which spawn a background thread that calls the real Sheets client;
    //! with no credentials on the test machine that thread fails fast over on an unrelated
    //! channel, so it never affects the synchronous assertions here.

    use super::*;
    use std::sync::mpsc;

    fn test_app() -> (App, mpsc::Receiver<WorkerMsg>) {
        let (tx, rx) = mpsc::channel();
        (App::new(tx), rx)
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn award(base_name: &str, name: &str, sheet: &str, col: &str, row: i32, cell: &str) -> Award {
        Award::new("badges", name)
            .with_location(sheet, col, row)
            .with_cell(cell, base_name)
    }

    fn awards_data_with(username: &str, awards: Vec<Award>) -> AwardsData {
        let mut data = AwardsData::default();
        data.index.insert(username.to_string(), awards);
        data
    }

    fn awards_data_with_catalog(
        username: &str,
        awards: Vec<Award>,
        catalog: Vec<AwardDef>,
    ) -> AwardsData {
        let mut data = awards_data_with(username, awards);
        data.catalog = catalog;
        data
    }

    fn award_def(category: &str, base_name: &str, sheet: &str, col: &str) -> AwardDef {
        AwardDef {
            category: category.to_string(),
            sheet: sheet.to_string(),
            col: col.to_string(),
            base_name: base_name.to_string(),
        }
    }

    fn assist_modal(username: &str, query: &str, step: AssistStep) -> AssistModal {
        AssistModal {
            username: username.to_string(),
            input: input_with_value(query.to_string()),
            step,
            report: String::new(),
            can_grant: false,
            grant: None,
            scroll: 0,
        }
    }

    // ---------- Esc cancels whatever modal is open ----------

    #[test]
    fn esc_closes_edit_modal_and_reports_cancelled() {
        let (mut app, _rx) = test_app();
        app.modal = Some(Modal::Edit(EditModal {
            award: award("Combat Action Badge", "Combat Action Badge", "Badges Database", "C", 10, "alice"),
            input: input_with_value("alice".into()),
        }));
        app.handle_key(key(KeyCode::Esc));
        assert!(app.modal.is_none());
        assert_eq!(app.status, "Dialog cancelled");
    }

    #[test]
    fn esc_closes_audit_modal_without_overwriting_status() {
        let (mut app, _rx) = test_app();
        app.status = "Wrote audits/audit-x.txt".to_string();
        app.modal = Some(Modal::Audit(AuditModal::new(
            "audits/audit-x.txt".into(),
            "line",
            Vec::new(),
        )));
        app.handle_key(key(KeyCode::Esc));
        assert!(app.modal.is_none());
        assert_eq!(
            app.status, "Wrote audits/audit-x.txt",
            "closing the audit browser must not stomp the summary line"
        );
    }

    // ---------- Opening a modal is refused while another is open, or while busy ----------

    #[test]
    fn action_add_refuses_to_replace_an_open_modal() {
        let (mut app, _rx) = test_app();
        app.data = Some(AwardsData::default());
        app.results_username = Some("alice".into());
        app.modal = Some(Modal::Assist(assist_modal("alice", "MCAB", AssistStep::Query)));
        app.action_add();
        assert!(
            matches!(app.modal, Some(Modal::Assist(_))),
            "an already-open modal must not be replaced"
        );
        assert_eq!(app.status, "Wait for the current sheet operation to finish");
    }

    #[test]
    fn action_edit_refuses_to_open_while_busy() {
        let (mut app, _rx) = test_app();
        app.data = Some(AwardsData::default());
        app.busy = true;
        app.action_edit();
        assert!(app.modal.is_none());
        assert_eq!(app.status, "Wait for the current sheet operation to finish");
    }

    // ---------- Edit modal: opening ----------

    #[test]
    fn action_edit_opens_prefilled_with_the_selected_awards_cell() {
        let (mut app, _rx) = test_app();
        let a = award("Combat Action Badge", "Combat Action Badge", "Badges Database", "C", 10, "alice x2");
        app.visible = vec![VisibleAward { award: a.clone(), warning: false }];
        app.awards_state.select(Some(0));
        app.action_edit();
        match &app.modal {
            Some(Modal::Edit(edit)) => {
                assert_eq!(edit.award, a);
                assert_eq!(edit.input.value(), "alice x2");
            }
            other => panic!("expected Edit modal, got {other:?}"),
        }
    }

    #[test]
    fn action_edit_without_a_selection_shows_a_hint_and_opens_nothing() {
        let (mut app, _rx) = test_app();
        app.action_edit();
        assert!(app.modal.is_none());
        assert_eq!(app.status, "Select an award to edit");
    }

    // ---------- Add modal: pick -> suffix ----------

    #[test]
    fn add_modal_enter_with_a_selection_advances_to_suffix() {
        let (mut app, _rx) = test_app();
        let def = AwardDef {
            category: "badges".into(),
            sheet: "Badges Database".into(),
            col: "C".into(),
            base_name: "Army Service Ribbon".into(),
        };
        app.modal = Some(Modal::Add(AddModal::new(vec![def.clone()])));
        app.handle_key(key(KeyCode::Enter));
        match &app.modal {
            Some(Modal::Add(add)) => {
                assert!(matches!(add.step, AddStep::Suffix));
                assert_eq!(add.chosen.as_ref(), Some(&def));
            }
            other => panic!("expected Add modal in Suffix step, got {other:?}"),
        }
    }

    #[test]
    fn add_modal_enter_with_no_filtered_candidates_stays_on_pick() {
        let (mut app, _rx) = test_app();
        let def = AwardDef {
            category: "badges".into(),
            sheet: "Badges Database".into(),
            col: "C".into(),
            base_name: "Army Service Ribbon".into(),
        };
        let mut add = AddModal::new(vec![def]);
        add.filtered.clear();
        add.state.select(None);
        app.modal = Some(Modal::Add(add));
        app.handle_key(key(KeyCode::Enter));
        match &app.modal {
            Some(Modal::Add(add)) => assert!(matches!(add.step, AddStep::Pick)),
            other => panic!("expected Add modal still on Pick step, got {other:?}"),
        }
        assert_eq!(app.status, "Select an award first");
    }

    #[test]
    fn add_modal_suffix_enter_commits_the_write_and_closes_the_modal() {
        let (mut app, _rx) = test_app();
        app.results_username = Some("alice".into());
        let def = AwardDef {
            category: "badges".into(),
            sheet: "Badges Database".into(),
            col: "C".into(),
            base_name: "Army Service Ribbon".into(),
        };
        let mut add = AddModal::new(vec![def.clone()]);
        add.chosen = Some(def.clone());
        add.step = AddStep::Suffix;
        app.modal = Some(Modal::Add(add));
        app.handle_key(key(KeyCode::Enter));
        assert!(app.modal.is_none(), "confirming Add should close the modal");
        assert!(app.busy, "confirming Add should mark the app busy while the write runs");
        assert_eq!(app.status, format!("Writing {}...", def.base_name));
    }

    // ---------- Delete modal: typed confirmation gate ----------

    #[test]
    fn delete_modal_requires_the_word_delete() {
        let (mut app, _rx) = test_app();
        let a = award("Combat Action Badge", "Combat Action Badge", "Badges Database", "C", 10, "alice");
        app.modal = Some(Modal::Delete(DeleteModal {
            award: a,
            input: input_with_value("nope".into()),
            viewed_username: "alice".into(),
        }));
        app.handle_key(key(KeyCode::Enter));
        assert!(app.modal.is_some(), "wrong confirmation text must not close the modal");
        assert!(!app.busy);
        assert_eq!(app.status, "Type \"delete\" to confirm");
    }

    #[test]
    fn delete_modal_confirms_case_insensitively_and_commits() {
        let (mut app, _rx) = test_app();
        let a = award("Combat Action Badge", "Combat Action Badge", "Badges Database", "C", 10, "alice");
        app.modal = Some(Modal::Delete(DeleteModal {
            award: a.clone(),
            input: input_with_value("DELETE".into()),
            viewed_username: "alice".into(),
        }));
        app.handle_key(key(KeyCode::Enter));
        assert!(app.modal.is_none());
        assert!(app.busy);
        assert_eq!(app.pending_delete.as_ref(), Some(&a));
        assert_eq!(app.status, format!("Removing {}...", a.name));
    }

    // ---------- Rename modal: two-step gate (name, then typed "rename") ----------

    #[test]
    fn rename_modal_rejects_a_non_bare_username() {
        let (mut app, _rx) = test_app();
        app.data = Some(AwardsData::default());
        app.modal = Some(Modal::Rename(RenameModal {
            from: "alice".into(),
            cell_count: 1,
            existing_new: 0,
            input: input_with_value("Bob - Master".into()),
            confirm: Input::default(),
            step: RenameStep::Name,
        }));
        app.handle_key(key(KeyCode::Enter));
        match &app.modal {
            Some(Modal::Rename(rename)) => assert!(matches!(rename.step, RenameStep::Name)),
            other => panic!("expected Rename modal still on Name step, got {other:?}"),
        }
        assert_eq!(app.status, "Enter a bare Roblox username (no suffixes)");
    }

    #[test]
    fn rename_modal_rejects_the_same_username() {
        let (mut app, _rx) = test_app();
        app.data = Some(AwardsData::default());
        app.modal = Some(Modal::Rename(RenameModal {
            from: "alice".into(),
            cell_count: 1,
            existing_new: 0,
            input: input_with_value("alice".into()),
            confirm: Input::default(),
            step: RenameStep::Name,
        }));
        app.handle_key(key(KeyCode::Enter));
        match &app.modal {
            Some(Modal::Rename(rename)) => assert!(matches!(rename.step, RenameStep::Name)),
            other => panic!("expected Rename modal still on Name step, got {other:?}"),
        }
        assert_eq!(app.status, "New username is the same as the current name");
    }

    #[test]
    fn rename_modal_valid_name_advances_to_confirm_step() {
        let (mut app, _rx) = test_app();
        app.data = Some(AwardsData::default());
        app.modal = Some(Modal::Rename(RenameModal {
            from: "alice".into(),
            cell_count: 1,
            existing_new: 0,
            input: input_with_value("bob".into()),
            confirm: Input::default(),
            step: RenameStep::Name,
        }));
        app.handle_key(key(KeyCode::Enter));
        match &app.modal {
            Some(Modal::Rename(rename)) => assert!(matches!(rename.step, RenameStep::Confirm)),
            other => panic!("expected Rename modal to advance to Confirm, got {other:?}"),
        }
    }

    #[test]
    fn rename_modal_confirm_step_recomputes_existing_new_award_count() {
        let (mut app, _rx) = test_app();
        app.data = Some(awards_data_with(
            "bob",
            vec![award("Combat Action Badge", "Combat Action Badge", "Badges Database", "C", 11, "bob")],
        ));
        app.modal = Some(Modal::Rename(RenameModal {
            from: "alice".into(),
            cell_count: 1,
            existing_new: 0,
            input: input_with_value("bob".into()),
            confirm: Input::default(),
            step: RenameStep::Confirm,
        }));
        // Any keystroke on the Confirm step recomputes existing_new, not just Enter.
        app.handle_key(key(KeyCode::Char('x')));
        match &app.modal {
            Some(Modal::Rename(rename)) => assert_eq!(rename.existing_new, 1),
            other => panic!("expected Rename modal on Confirm step, got {other:?}"),
        }
    }

    #[test]
    fn rename_modal_confirm_requires_the_word_rename() {
        let (mut app, _rx) = test_app();
        app.data = Some(AwardsData::default());
        app.modal = Some(Modal::Rename(RenameModal {
            from: "alice".into(),
            cell_count: 1,
            existing_new: 0,
            input: input_with_value("bob".into()),
            confirm: input_with_value("nope".into()),
            step: RenameStep::Confirm,
        }));
        app.handle_key(key(KeyCode::Enter));
        assert!(app.modal.is_some(), "wrong confirm text must not close the modal");
        assert_eq!(app.status, "Type \"rename\" to confirm");
    }

    #[test]
    fn rename_modal_confirm_typed_commits_the_rename_and_closes() {
        let (mut app, _rx) = test_app();
        app.data = Some(AwardsData::default());
        app.results_username = Some("alice".into());
        app.modal = Some(Modal::Rename(RenameModal {
            from: "alice".into(),
            cell_count: 1,
            existing_new: 0,
            input: input_with_value("bob".into()),
            confirm: input_with_value("rename".into()),
            step: RenameStep::Confirm,
        }));
        app.handle_key(key(KeyCode::Enter));
        assert!(app.modal.is_none());
        assert!(app.busy);
        assert_eq!(app.status, "Renaming @alice → bob across the sheet...");
    }

    // ---------- Assist modal: query -> result, then optional grant ----------

    #[test]
    fn assist_query_empty_shows_a_hint_and_stays_on_query_step() {
        let (mut app, _rx) = test_app();
        app.data = Some(AwardsData::default());
        app.modal = Some(Modal::Assist(assist_modal("alice", "", AssistStep::Query)));
        app.handle_key(key(KeyCode::Enter));
        match &app.modal {
            Some(Modal::Assist(assist)) => assert!(matches!(assist.step, AssistStep::Query)),
            other => panic!("expected Assist modal still on Query step, got {other:?}"),
        }
        assert_eq!(app.status, "Enter an award request (MCAB / MCIB / MCMB)");
    }

    #[test]
    fn assist_query_approve_advances_to_result_with_grant_enabled() {
        let (mut app, _rx) = test_app();
        app.data = Some(awards_data_with(
            "alice",
            vec![
                award("Expert Soldier Badge", "Expert Soldier Badge", "Badges Database", "D", 10, "alice"),
                award("Combat Action Badge", "Combat Action Badge", "Badges Database", "C", 10, "alice"),
            ],
        ));
        app.modal = Some(Modal::Assist(assist_modal("alice", "MCAB", AssistStep::Query)));
        app.handle_key(key(KeyCode::Enter));
        match &app.modal {
            Some(Modal::Assist(assist)) => {
                assert!(matches!(assist.step, AssistStep::Result));
                assert!(assist.can_grant);
                assert!(assist.grant.is_some());
            }
            other => panic!("expected Assist modal in Result step, got {other:?}"),
        }
        assert!(app.status.contains("Eligible"));
    }

    #[test]
    fn assist_query_deny_advances_to_result_without_grant() {
        let (mut app, _rx) = test_app();
        app.data = Some(awards_data_with(
            "alice",
            vec![award("Combat Action Badge", "Combat Action Badge", "Badges Database", "C", 10, "alice")],
        ));
        app.modal = Some(Modal::Assist(assist_modal("alice", "MCAB", AssistStep::Query)));
        app.handle_key(key(KeyCode::Enter));
        match &app.modal {
            Some(Modal::Assist(assist)) => {
                assert!(matches!(assist.step, AssistStep::Result));
                assert!(!assist.can_grant);
                assert!(assist.grant.is_none());
            }
            other => panic!("expected Assist modal in Result step, got {other:?}"),
        }
        assert_eq!(app.status, "Assist result — Esc to close");
    }

    #[test]
    fn assist_result_enter_without_can_grant_is_a_no_op() {
        let (mut app, _rx) = test_app();
        app.data = Some(AwardsData::default());
        app.status = "unchanged".into();
        let mut assist = assist_modal("alice", "MCAB", AssistStep::Result);
        assist.report = "DENY ...".into();
        app.modal = Some(Modal::Assist(assist));
        app.handle_key(key(KeyCode::Enter));
        assert!(app.modal.is_some(), "modal should stay open when nothing can be granted");
        assert!(!app.busy);
        assert_eq!(app.status, "unchanged");
    }

    #[test]
    fn assist_result_scroll_is_saturating_and_moves_with_j_k() {
        let (mut app, _rx) = test_app();
        app.data = Some(AwardsData::default());
        app.modal = Some(Modal::Assist(assist_modal("alice", "MCAB", AssistStep::Result)));
        app.handle_key(key(KeyCode::Up)); // saturating_sub(1) from 0 stays 0
        match &app.modal {
            Some(Modal::Assist(assist)) => assert_eq!(assist.scroll, 0),
            other => panic!("expected Assist modal, got {other:?}"),
        }
        app.handle_key(key(KeyCode::Char('j')));
        match &app.modal {
            Some(Modal::Assist(assist)) => assert_eq!(assist.scroll, 1),
            other => panic!("expected Assist modal, got {other:?}"),
        }
    }

    #[test]
    fn assist_result_enter_with_can_grant_commits_the_grant_and_closes_modal() {
        let (mut app, _rx) = test_app();
        app.data = Some(awards_data_with(
            "alice",
            vec![
                award("Expert Soldier Badge", "Expert Soldier Badge", "Badges Database", "D", 10, "alice"),
                award("Combat Action Badge", "Combat Action Badge", "Badges Database", "C", 10, "alice"),
            ],
        ));
        let mut assist = assist_modal("alice", "MCAB", AssistStep::Result);
        assist.can_grant = true;
        assist.grant = Some(GrantPlan::UpgradeCell {
            base_name: "Combat Action Badge".into(),
            new_cell: "alice - MC".into(),
        });
        app.modal = Some(Modal::Assist(assist));
        app.handle_key(key(KeyCode::Enter));
        assert!(app.modal.is_none(), "granting should close the Assist modal");
        assert!(app.busy, "granting dispatches a sheet write (via commit_edit)");
    }

    // ---------- Audit modal: scroll only, Esc handled above ----------

    #[test]
    fn audit_modal_scroll_keys_clamp_and_jump() {
        let (mut app, _rx) = test_app();
        let lines: Vec<String> = (0..20).map(|i| format!("line {i}")).collect();
        let mut audit = AuditModal::new("audits/audit-x.txt".into(), &lines.join("\n"), Vec::new());
        audit.view = AuditView::Report;
        audit.scroll = 5;
        app.modal = Some(Modal::Audit(audit));
        app.handle_key(key(KeyCode::PageUp));
        match &app.modal {
            Some(Modal::Audit(a)) => assert_eq!(a.scroll, 0, "5.saturating_sub(10) == 0"),
            other => panic!("expected Audit modal, got {other:?}"),
        }
        if let Some(Modal::Audit(a)) = app.modal.as_mut() {
            a.scroll = 5;
        }
        app.handle_key(key(KeyCode::PageDown));
        match &app.modal {
            Some(Modal::Audit(a)) => assert_eq!(a.scroll, 15),
            other => panic!("expected Audit modal, got {other:?}"),
        }
        app.handle_key(key(KeyCode::End));
        match &app.modal {
            Some(Modal::Audit(a)) => assert_eq!(a.scroll, (lines.len() - 1) as u16),
            other => panic!("expected Audit modal, got {other:?}"),
        }
        app.handle_key(key(KeyCode::Home));
        match &app.modal {
            Some(Modal::Audit(a)) => assert_eq!(a.scroll, 0),
            other => panic!("expected Audit modal, got {other:?}"),
        }
    }

    // ---------- Full pipeline: keypress opens the right modal ----------

    #[test]
    fn c_key_opens_assist_modal_prefilled_with_mcab() {
        let (mut app, _rx) = test_app();
        app.data = Some(AwardsData::default());
        app.results_username = Some("alice".into());
        app.focus = FocusArea::Awards;
        app.handle_key(key(KeyCode::Char('c')));
        match &app.modal {
            Some(Modal::Assist(assist)) => {
                assert_eq!(assist.username, "alice");
                assert!(matches!(assist.step, AssistStep::Query));
                assert_eq!(assist.input.value(), "MCAB");
            }
            other => panic!("expected Assist modal to open, got {other:?}"),
        }
    }

    #[test]
    fn c_key_is_ignored_while_focus_is_on_the_username_field() {
        let (mut app, _rx) = test_app();
        app.data = Some(AwardsData::default());
        app.results_username = Some("alice".into());
        // Default focus is Username; 'c' should be typed into the field, not open Assist.
        app.handle_key(key(KeyCode::Char('c')));
        assert!(app.modal.is_none());
        assert_eq!(app.username.value(), "c");
    }

    // ---------- Selecting an audit finding jumps into the matching fix flow ----------

    fn duplicate_row_finding() -> AuditFinding {
        AuditFinding::DuplicateRow {
            user: "alice".into(),
            sheet: "Badges Database".into(),
            col: "C".into(),
            base_name: "Combat Action Badge".into(),
            kind: "identical".into(),
            row: 10,
            cell: "alice".into(),
        }
    }

    fn malformed_cell_finding() -> AuditFinding {
        AuditFinding::MalformedCell {
            user: "carol".into(),
            sheet: "Ribbons Database".into(),
            col: "D".into(),
            base_name: "Test Ribbon".into(),
            row: 12,
            cell: "carol -Senior".into(),
            issues: vec!["missing_space_before_dash".into()],
        }
    }

    fn similar_usernames_finding() -> AuditFinding {
        AuditFinding::SimilarUsernames {
            a: "bob".into(),
            b: "bobb".into(),
            sheet: "Badges Database".into(),
            col: "C".into(),
            base_name: "Combat Action Badge".into(),
        }
    }

    #[test]
    fn enter_on_duplicate_finding_opens_delete_modal_and_stashes_audit() {
        let (mut app, _rx) = test_app();
        app.data = Some(AwardsData::default());
        let finding = duplicate_row_finding();
        app.modal = Some(Modal::Audit(AuditModal::new(
            "audits/audit-x.txt".into(),
            "report body",
            vec![finding.clone()],
        )));
        app.handle_key(key(KeyCode::Enter));
        match &app.modal {
            Some(Modal::Delete(delete)) => {
                assert_eq!(delete.award.base_name, "Combat Action Badge");
                assert_eq!(delete.viewed_username, "alice");
            }
            other => panic!("expected Delete modal, got {other:?}"),
        }
        assert!(app.saved_audit.is_some(), "the audit list must be stashed for Esc/failure to restore");
        assert_eq!(app.audit_fix_username.as_deref(), Some("alice"));
    }

    #[test]
    fn enter_on_malformed_finding_opens_edit_modal_prefilled_with_the_cell() {
        let (mut app, _rx) = test_app();
        app.data = Some(AwardsData::default());
        let finding = malformed_cell_finding();
        app.modal = Some(Modal::Audit(AuditModal::new(
            "audits/audit-x.txt".into(),
            "report body",
            vec![finding],
        )));
        app.handle_key(key(KeyCode::Enter));
        match &app.modal {
            Some(Modal::Edit(edit)) => {
                assert_eq!(edit.award.base_name, "Test Ribbon");
                assert_eq!(edit.input.value(), "carol -Senior");
            }
            other => panic!("expected Edit modal, got {other:?}"),
        }
        assert!(app.saved_audit.is_some());
        assert_eq!(app.audit_fix_username.as_deref(), Some("carol"));
    }

    #[test]
    fn esc_from_a_finding_triggered_fix_restores_the_audit_list_unchanged() {
        let (mut app, _rx) = test_app();
        app.data = Some(AwardsData::default());
        let finding = duplicate_row_finding();
        app.modal = Some(Modal::Audit(AuditModal::new(
            "audits/audit-x.txt".into(),
            "report body",
            vec![finding],
        )));
        app.handle_key(key(KeyCode::Enter));
        assert!(matches!(app.modal, Some(Modal::Delete(_))));

        app.handle_key(key(KeyCode::Esc));
        match &app.modal {
            Some(Modal::Audit(audit)) => assert_eq!(audit.list.len(), 1, "the finding is still there"),
            other => panic!("expected Esc to restore the Audit modal, got {other:?}"),
        }
        assert!(app.saved_audit.is_none(), "the stash must be cleared once restored");
        assert!(app.audit_fix_username.is_none());
    }

    #[test]
    fn enter_on_similar_usernames_finding_opens_a_choice_then_rename() {
        let (mut app, _rx) = test_app();
        app.data = Some(awards_data_with(
            "bobb",
            vec![award("Combat Action Badge", "Combat Action Badge", "Badges Database", "C", 10, "bobb")],
        ));
        let finding = similar_usernames_finding();
        app.modal = Some(Modal::Audit(AuditModal::new(
            "audits/audit-x.txt".into(),
            "report body",
            vec![finding],
        )));
        app.handle_key(key(KeyCode::Enter));
        match &app.modal {
            Some(Modal::Audit(audit)) => {
                assert!(matches!(audit.view, AuditView::ChooseUsername { .. }))
            }
            other => panic!("expected the Audit modal to show the username choice, got {other:?}"),
        }

        app.handle_key(key(KeyCode::Char('2'))); // pick "bobb", the one with sheet cells
        match &app.modal {
            Some(Modal::Rename(rename)) => assert_eq!(rename.from, "bobb"),
            other => panic!("expected Rename modal, got {other:?}"),
        }
        assert!(app.saved_audit.is_some());
    }

    #[test]
    fn esc_from_the_similar_usernames_choice_returns_to_the_list_not_the_whole_close() {
        let (mut app, _rx) = test_app();
        app.data = Some(AwardsData::default());
        let finding = similar_usernames_finding();
        app.modal = Some(Modal::Audit(AuditModal::new(
            "audits/audit-x.txt".into(),
            "report body",
            vec![finding],
        )));
        app.handle_key(key(KeyCode::Enter));
        assert!(matches!(
            app.modal,
            Some(Modal::Audit(ref audit)) if matches!(audit.view, AuditView::ChooseUsername { .. })
        ));

        app.handle_key(key(KeyCode::Esc));
        match &app.modal {
            Some(Modal::Audit(audit)) => assert!(matches!(audit.view, AuditView::List)),
            other => panic!("expected the Audit modal to still be open on the list, got {other:?}"),
        }
    }

    #[test]
    fn esc_from_a_rename_opened_via_similar_usernames_restores_the_list_not_the_choice() {
        // Regression test (convergence T025): `open_rename_for_finding` used to stash
        // the Audit modal while its `view` was still `ChooseUsername { .. }`, so Esc
        // (or a failed rename write) would restore the modal frozen on the
        // two-username choice instead of the findings list.
        let (mut app, _rx) = test_app();
        app.data = Some(awards_data_with(
            "bobb",
            vec![award("Combat Action Badge", "Combat Action Badge", "Badges Database", "C", 10, "bobb")],
        ));
        let finding = similar_usernames_finding();
        app.modal = Some(Modal::Audit(AuditModal::new(
            "audits/audit-x.txt".into(),
            "report body",
            vec![finding],
        )));
        app.handle_key(key(KeyCode::Enter));
        assert!(matches!(
            app.modal,
            Some(Modal::Audit(ref audit)) if matches!(audit.view, AuditView::ChooseUsername { .. })
        ));
        app.handle_key(key(KeyCode::Char('2'))); // pick "bobb", the one with sheet cells
        assert!(matches!(app.modal, Some(Modal::Rename(_))));

        app.handle_key(key(KeyCode::Esc));
        match &app.modal {
            Some(Modal::Audit(audit)) => assert!(
                matches!(audit.view, AuditView::List),
                "the restored Audit modal must show the findings list, not be stuck on the choice"
            ),
            other => panic!("expected Esc to restore the Audit modal, got {other:?}"),
        }
        assert!(app.saved_audit.is_none());
    }

    // ---------- Selecting an UnparseableCell finding has no fix flow to jump into ----------

    #[test]
    fn enter_on_unparseable_finding_leaves_the_audit_modal_open_unchanged() {
        let (mut app, _rx) = test_app();
        app.data = Some(AwardsData::default());
        let finding = AuditFinding::UnparseableCell {
            sheet: "Badges Database".into(),
            col: "C".into(),
            base_name: "Combat Action Badge".into(),
            row: 14,
            cell: "???".into(),
        };
        app.modal = Some(Modal::Audit(AuditModal::new(
            "audits/audit-x.txt".into(),
            "report body",
            vec![finding],
        )));
        app.handle_key(key(KeyCode::Enter));
        match &app.modal {
            Some(Modal::Audit(audit)) => {
                assert!(
                    matches!(audit.view, AuditView::List),
                    "no sub-dialog or fix flow opens for an unparseable cell"
                );
                assert_eq!(audit.list.len(), 1, "the finding is still there — nothing selects it away");
            }
            other => panic!("expected the Audit modal to remain open and unchanged, got {other:?}"),
        }
        assert!(app.saved_audit.is_none());
        assert_eq!(
            app.status,
            "No direct fix available for this cell — edit it manually on the sheet"
        );
    }

    // ---------- Findings-list navigation skips headers and wraps ----------

    #[test]
    fn navigation_skips_group_headers_and_wraps_across_multiple_groups() {
        let (mut app, _rx) = test_app();
        app.data = Some(AwardsData::default());
        // "alice" sorts before "carol", so rendered rows are:
        // [Header(alice), Item(alice), Header(carol), Item(carol)] — rows 1 and 3 are
        // the only selectable ones; 0 and 2 are headers.
        let findings = vec![duplicate_row_finding(), malformed_cell_finding()];
        app.modal = Some(Modal::Audit(AuditModal::new(
            "audits/audit-x.txt".into(),
            "report body",
            findings,
        )));
        let selected = |app: &App| match &app.modal {
            Some(Modal::Audit(audit)) => audit.list_state.selected(),
            _ => None,
        };
        assert_eq!(selected(&app), Some(1), "constructor selects the first selectable row");

        app.handle_key(key(KeyCode::Down));
        assert_eq!(selected(&app), Some(3), "Down skips the carol header row (2)");

        app.handle_key(key(KeyCode::Down));
        assert_eq!(selected(&app), Some(1), "Down from the last finding wraps to the first");

        app.handle_key(key(KeyCode::Up));
        assert_eq!(selected(&app), Some(3), "Up from the first finding wraps to the last");

        app.handle_key(key(KeyCode::PageUp));
        assert_eq!(selected(&app), Some(1), "PageUp(5) wraps by position, not by row index");

        app.handle_key(key(KeyCode::PageDown));
        assert_eq!(selected(&app), Some(3), "PageDown(5) wraps back to the last finding");
    }

    // ---------- Tab toggles between the findings list and the plain-text report ----------

    #[test]
    fn tab_toggles_list_and_report_without_closing_the_modal_or_discarding_the_list() {
        let (mut app, _rx) = test_app();
        app.data = Some(AwardsData::default());
        let finding = duplicate_row_finding();
        app.modal = Some(Modal::Audit(AuditModal::new(
            "audits/audit-x.txt".into(),
            "report body",
            vec![finding],
        )));

        app.handle_key(key(KeyCode::Tab));
        match &app.modal {
            Some(Modal::Audit(audit)) => {
                assert!(matches!(audit.view, AuditView::Report));
                assert_eq!(audit.list.len(), 1, "toggling to Report must not discard the findings list");
            }
            other => panic!("expected the Audit modal to stay open on Report, got {other:?}"),
        }

        app.handle_key(key(KeyCode::Tab));
        match &app.modal {
            Some(Modal::Audit(audit)) => {
                assert!(matches!(audit.view, AuditView::List));
                assert_eq!(audit.list.len(), 1, "toggling back to List must still have the findings list");
            }
            other => panic!("expected the Audit modal to stay open on List, got {other:?}"),
        }
    }

    // ---------- A fresh audit run always opens on the findings list ----------

    #[test]
    fn audit_done_success_opens_the_modal_defaulted_to_the_findings_list() {
        let (mut app, _rx) = test_app();
        app.modal = None;
        app.busy = true;
        app.handle_worker_msg(WorkerMsg::AuditDone(Ok(AuditOutcome {
            path: "audits/audit-x.txt".into(),
            body: "report body".into(),
            summary: "1 finding".into(),
            findings: vec![duplicate_row_finding()],
        })));
        assert!(!app.busy);
        assert_eq!(app.status, "1 finding");
        match &app.modal {
            Some(Modal::Audit(audit)) => {
                assert!(
                    matches!(audit.view, AuditView::List),
                    "a fresh audit must open on the findings list, not the text report"
                );
                assert_eq!(audit.list.len(), 1);
            }
            other => panic!("expected the Audit modal to open, got {other:?}"),
        }
    }

    // ---------- A write launched from a selected finding refreshes (or restores) the audit ----------

    #[test]
    fn successful_write_from_a_finding_reopens_audit_with_a_refreshed_list() {
        let (mut app, _rx) = test_app();
        let target = award("Combat Action Badge", "Combat Action Badge", "Badges Database", "C", 10, "alice");
        app.data = Some(awards_data_with("alice", vec![target.clone()]));
        let finding = duplicate_row_finding();
        app.modal = Some(Modal::Audit(AuditModal::new(
            "audits/audit-x.txt".into(),
            "report body",
            vec![finding],
        )));
        app.handle_key(key(KeyCode::Enter));
        assert!(matches!(app.modal, Some(Modal::Delete(_))));

        app.handle_worker_msg(WorkerMsg::WriteDone {
            kind: "delete",
            result: EditResult {
                ok: true,
                message: "Removed Combat Action Badge".to_string(),
                error: None,
                award: None,
                awards: Vec::new(),
            },
            username: "alice".into(),
        });

        match &app.modal {
            Some(Modal::Audit(audit)) => {
                assert_eq!(audit.path, "audits/audit-x.txt", "reuses the same export path");
                assert!(
                    audit.list.is_empty(),
                    "no sheet rows means nothing left to audit after the local patch"
                );
            }
            other => panic!("expected the write to reopen the Audit modal, got {other:?}"),
        }
        assert_eq!(app.status, "Removed Combat Action Badge");
        assert!(app.saved_audit.is_none());
        assert!(app.audit_fix_username.is_none());
    }

    #[test]
    fn failed_write_from_a_finding_restores_the_saved_audit_unchanged() {
        let (mut app, _rx) = test_app();
        let target = award("Combat Action Badge", "Combat Action Badge", "Badges Database", "C", 10, "alice");
        app.data = Some(awards_data_with("alice", vec![target]));
        let finding = duplicate_row_finding();
        app.modal = Some(Modal::Audit(AuditModal::new(
            "audits/audit-x.txt".into(),
            "report body",
            vec![finding],
        )));
        app.handle_key(key(KeyCode::Enter));
        assert!(matches!(app.modal, Some(Modal::Delete(_))));

        app.handle_worker_msg(WorkerMsg::WriteDone {
            kind: "delete",
            result: EditResult {
                ok: false,
                message: "row changed on the sheet".to_string(),
                error: None,
                award: None,
                awards: Vec::new(),
            },
            username: "alice".into(),
        });

        match &app.modal {
            Some(Modal::Audit(audit)) => {
                assert_eq!(audit.list.len(), 1, "the failed fix must not drop the finding")
            }
            other => panic!("expected the failed write to restore the Audit modal, got {other:?}"),
        }
        assert_eq!(app.status, "delete failed: row changed on the sheet");
        assert!(app.saved_audit.is_none());
        assert!(app.audit_fix_username.is_none());
    }

    // ---------- Discord paste quick-add (003-discord-paste-quick-add) ----------

    #[test]
    fn paste_add_with_a_confident_match_lands_in_the_suffix_step_prefilled() {
        // The real badge-request sample gathered while writing spec.md.
        let (mut app, _rx) = test_app();
        app.data = Some(awards_data_with_catalog(
            "torba_f",
            Vec::new(),
            vec![
                award_def("badges", "Army Parachutist Badge", "Badges Database", "C"),
                award_def("badges", "Combat Action Badge", "Badges Database", "D"),
            ],
        ));
        app.modal = Some(Modal::PasteAdd(PasteAddModal::default()));
        app.handle_paste(
            "ROBLOX Username: torba_f\n\
             ROBLOX ID: 2452545815\n\
             Current Division & Rank: 1ID, Colonel\n\
             Badge Requested: Army Parachutist Badge\n\
             Proof: [image attachment]"
                .to_string(),
        );
        app.handle_key(key(KeyCode::Enter));

        match &app.modal {
            Some(Modal::Add(add)) => {
                assert!(matches!(add.step, AddStep::Suffix));
                assert_eq!(
                    add.chosen.as_ref().map(|def| def.base_name.as_str()),
                    Some("Army Parachutist Badge")
                );
                assert_eq!(add.suffix.value(), "");
            }
            other => panic!("expected a pre-filled Add modal, got {other:?}"),
        }
        assert_eq!(app.results_username.as_deref(), Some("torba_f"));
    }

    #[test]
    fn paste_add_with_a_repeat_count_suffix_prefills_the_suffix_field() {
        // The real ribbon-request sample gathered while writing spec.md. The catalog entry is
        // spelled to match the clerk's own "Afganistan" text — extraction and matching don't
        // second-guess the clerk's spelling; a live catalog spelled differently would instead
        // fall through to the picker path (see the "no catalog match" test below), which is the
        // spec-correct behavior for a genuine typo, not a bug.
        let (mut app, _rx) = test_app();
        app.data = Some(awards_data_with_catalog(
            "Nevazaku_u",
            Vec::new(),
            vec![award_def(
                "ribbons",
                "Afganistan Campaign",
                "Ribbons Database",
                "E",
            )],
        ));
        app.modal = Some(Modal::PasteAdd(PasteAddModal::default()));
        app.handle_paste(
            "ROBLOX Username: Nevazaku_u\n\
             ROBLOX ID: 1881585077\n\
             Current Division & Rank: JFKSWCS, Brigadier General\n\
             Ribbon Requested: Afganistan Campaign x1\n\
             Proof: https://docs.google.com/spreadsheets/d/1Y8jEcLpRb6lDDhe7Axb5Z27dotsb3p-QKMtpvFATEjY/edit?gid=1708955154#gid=1708955154"
                .to_string(),
        );
        app.handle_key(key(KeyCode::Enter));

        match &app.modal {
            Some(Modal::Add(add)) => {
                assert!(matches!(add.step, AddStep::Suffix));
                assert_eq!(
                    add.chosen.as_ref().map(|def| def.base_name.as_str()),
                    Some("Afganistan Campaign")
                );
                assert_eq!(
                    add.suffix.value(),
                    "x1",
                    "FR-011: the count indicator must carry through"
                );
            }
            other => panic!("expected a pre-filled Add modal, got {other:?}"),
        }
    }

    #[test]
    fn confirming_a_paste_prefilled_add_reaches_the_same_write_dispatch_as_manual_add() {
        let (mut app, _rx) = test_app();
        app.data = Some(awards_data_with_catalog(
            "torba_f",
            Vec::new(),
            vec![award_def(
                "badges",
                "Army Parachutist Badge",
                "Badges Database",
                "C",
            )],
        ));
        app.modal = Some(Modal::PasteAdd(PasteAddModal::default()));
        app.handle_paste("ROBLOX Username: torba_f\nBadge Requested: Army Parachutist Badge".into());
        app.handle_key(key(KeyCode::Enter));
        assert!(
            matches!(app.modal, Some(Modal::Add(_))),
            "expected the paste to resolve into Modal::Add first"
        );

        // Confirming the Suffix step is identical to what a manually-entered Add already does.
        app.handle_key(key(KeyCode::Enter));
        assert!(
            app.modal.is_none(),
            "commit_add closes the modal exactly like a manual Add (FR-009)"
        );
        assert!(
            app.busy,
            "the write dispatch went through the same begin_busy/commit_add path"
        );
        assert!(
            app.status.starts_with("Writing Army Parachutist Badge"),
            "status: {}",
            app.status
        );
    }

    #[test]
    fn paste_add_with_no_catalog_match_opens_the_picker_prefiltered() {
        let (mut app, _rx) = test_app();
        app.data = Some(awards_data_with_catalog(
            "torba_f",
            Vec::new(),
            vec![award_def(
                "badges",
                "Army Parachutist Badge",
                "Badges Database",
                "C",
            )],
        ));
        app.modal = Some(Modal::PasteAdd(PasteAddModal::default()));
        app.handle_paste("ROBLOX Username: torba_f\nBadge Requested: Not A Real Badge".into());
        app.handle_key(key(KeyCode::Enter));

        match &app.modal {
            Some(Modal::Add(add)) => {
                assert!(matches!(add.step, AddStep::Pick));
                assert_eq!(add.filter.value(), "Not A Real Badge");
                assert!(
                    add.filtered.is_empty(),
                    "the filter matches nothing in the catalog"
                );
            }
            other => panic!("expected the Pick-step picker, got {other:?}"),
        }
        assert_eq!(app.results_username.as_deref(), Some("torba_f"));
    }

    #[test]
    fn paste_add_with_multiple_catalog_matches_opens_the_picker_prefiltered() {
        let (mut app, _rx) = test_app();
        app.data = Some(awards_data_with_catalog(
            "torba_f",
            Vec::new(),
            vec![
                award_def("badges", "Army Parachutist Badge", "Badges Database", "C"),
                award_def("badges", "Army Air Assault Badge", "Badges Database", "D"),
            ],
        ));
        app.modal = Some(Modal::PasteAdd(PasteAddModal::default()));
        app.handle_paste("ROBLOX Username: torba_f\nBadge Requested: Army".into());
        app.handle_key(key(KeyCode::Enter));

        match &app.modal {
            Some(Modal::Add(add)) => {
                assert!(matches!(add.step, AddStep::Pick));
                assert_eq!(add.filter.value(), "Army");
                assert_eq!(
                    add.filtered.len(),
                    2,
                    "both Army-prefixed badges should still be listed for the clerk to choose"
                );
            }
            other => panic!("expected the Pick-step picker, got {other:?}"),
        }
    }

    #[test]
    fn paste_add_with_no_username_line_keeps_the_modal_open_with_an_error() {
        let (mut app, _rx) = test_app();
        app.data = Some(AwardsData::default());
        app.modal = Some(Modal::PasteAdd(PasteAddModal::default()));
        app.handle_paste("Badge Requested: Army Parachutist Badge".into());
        app.handle_key(key(KeyCode::Enter));

        match &app.modal {
            Some(Modal::PasteAdd(paste)) => {
                assert!(paste.error.is_some(), "expected an inline error");
                assert!(
                    paste.buffer.contains("Army Parachutist Badge"),
                    "the buffer must be preserved so the clerk can correct and resubmit"
                );
            }
            other => panic!("expected Modal::PasteAdd to stay open, got {other:?}"),
        }
    }

    #[test]
    fn paste_add_with_completely_unparseable_text_gets_the_same_generic_error() {
        let (mut app, _rx) = test_app();
        app.data = Some(AwardsData::default());
        app.modal = Some(Modal::PasteAdd(PasteAddModal::default()));
        app.handle_paste("just some unrelated text with no labeled lines at all".into());
        app.handle_key(key(KeyCode::Enter));

        match &app.modal {
            Some(Modal::PasteAdd(paste)) => assert!(paste.error.is_some()),
            other => panic!("expected Modal::PasteAdd to stay open, got {other:?}"),
        }
    }

    #[test]
    fn esc_from_paste_add_closes_it_and_reports_cancelled() {
        let (mut app, _rx) = test_app();
        app.modal = Some(Modal::PasteAdd(PasteAddModal::default()));
        app.handle_key(key(KeyCode::Esc));
        assert!(app.modal.is_none());
        assert_eq!(app.status, "Dialog cancelled");
    }
}
