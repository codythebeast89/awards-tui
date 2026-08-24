use crate::config::AppConfig;
use awards_core::{
    awards_excluding_duplicate_rows, col_to_index, collect_sheet_audit, find_duplicates_for_user,
    flatten_awards_sorted, format_audit_report, get_awards_for_username, normalize_username,
    owned_award_columns, reindex_column_after_delete, row_offset, shift_column_up_in_rows,
    upsert_award_in_index, Award, AwardDef, AwardsData, CATEGORY_LABELS,
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
    Action::Edit,
    Action::Delete,
    Action::Rename,
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
    Edit,
    Delete,
    Rename,
    Refresh,
    Audit,
}

impl Action {
    pub fn label(self) -> &'static str {
        match self {
            Self::Lookup => "Lookup",
            Self::Add => "Add",
            Self::Edit => "Edit",
            Self::Delete => "Delete",
            Self::Rename => "Rename",
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
    Edit(EditModal),
    Delete(DeleteModal),
    Rename(RenameModal),
    Audit(AuditModal),
}

#[derive(Debug)]
pub struct AuditModal {
    pub path: String,
    pub lines: Vec<String>,
    pub scroll: u16,
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
                        self.modal = Some(Modal::Audit(AuditModal {
                            path: outcome.path,
                            lines: outcome.body.lines().map(str::to_string).collect(),
                            scroll: 0,
                        }));
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
            KeyCode::Char('e') if self.focus != FocusArea::Username => self.action_edit(),
            KeyCode::Char('d') if self.focus != FocusArea::Username => self.action_delete(),
            KeyCode::Char('n') if self.focus != FocusArea::Username => self.action_rename(),
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
        if !result.ok {
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
            self.apply_user_view(&username, result.awards.first(), Some(result.message));
        } else {
            self.status = result.message;
        }
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
            let closing_audit = matches!(self.modal, Some(Modal::Audit(_)));
            self.modal = None;
            if !closing_audit {
                self.status = "Dialog cancelled".to_string();
            }
            return;
        }

        if let Some(Modal::Audit(audit)) = self.modal.as_mut() {
            match key.code {
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
                _ => {}
            }
            return;
        }

        let mut add_filter_changed = false;
        let mut add_confirm: Option<(AwardDef, String)> = None;
        let mut edit_confirm: Option<(Award, String)> = None;
        let mut delete_confirm: Option<Award> = None;
        let mut rename_confirm: Option<String> = None;
        let mut status: Option<String> = None;

        if let Some(modal) = self.modal.as_mut() {
            match modal {
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
                            if let Some(new_key) = normalize_username(Some(raw)) {
                                if new_key == rename.from {
                                    status = Some(
                                        "New username is the same as the current name".to_string(),
                                    );
                                } else {
                                    rename.step = RenameStep::Confirm;
                                }
                            } else {
                                status = Some("Enter the new Roblox username".to_string());
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
                let new_key = normalize_username(Some(rename.input.value())).unwrap_or_default();
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
            Action::Edit => self.action_edit(),
            Action::Delete => self.action_delete(),
            Action::Rename => self.action_rename(),
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
            let view_user = normalize_username(Some(&new_username))
                .unwrap_or_else(|| new_username.trim().trim_start_matches('@').to_string());
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
        let query = self.filter.value().trim().to_ascii_lowercase();
        self.filtered = self
            .all_candidates
            .iter()
            .filter(|def| {
                query.is_empty()
                    || def.base_name.to_ascii_lowercase().contains(&query)
                    || category_label(&def.category)
                        .to_ascii_lowercase()
                        .contains(&query)
            })
            .cloned()
            .collect();
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
