//! `egui` rendering for `awards-gui`. Thin by design (mirrors `awards-tui/src/tui/ui.rs`'s role):
//! every widget here reads or drives `GuiApp` state (`app.rs`) and contains no business logic of
//! its own — parsing, matching, and the write path all live in `awards-core`/`awards-sheets` and
//! are called from `app.rs`, never from here. Not directly unit-tested (plan.md Testing section)
//! since this environment cannot open a display; `app.rs`'s tests cover the state this renders.

use crate::app::{AuthState, GuiApp, StatusKind};
use crate::theme;
use awards_core::{Award, CATEGORY_LABELS};
use eframe::egui;

/// Horizontal breathing room applied to every panel (006-gui-visual-theme's spacing pass widened
/// spacing *between* widgets; this widens the margin *around* each panel's content so a wide
/// window doesn't read as everything jammed against the left edge — 007-gui-edit polish pass).
const PANEL_MARGIN: egui::Margin = egui::Margin {
    left: 20,
    right: 20,
    top: 12,
    bottom: 12,
};

/// A dismissive/secondary action (Cancel, etc.) — critique follow-up: every enabled button used
/// to share the same accent fill as the one action per screen that actually commits a write
/// (Confirm/Save), making the two indistinguishable except by reading the label. An
/// outline/ghost treatment (background fill, a `gui-border` stroke, ordinary text color) reserves
/// the accent for the primary action.
fn secondary_button(text: &str) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(text.to_string()).color(theme::TEXT))
        .fill(theme::BG)
        .stroke(egui::Stroke::new(1.0, theme::BORDER))
}

/// Renders the whole window (contract: "single window, no modal dialogs" — the award picker and
/// edit flow are inline panels, not separate windows). `ui` is the root `Ui` eframe hands to
/// `App::ui` for this pass — top-level panels attach to it directly (egui 0.36's unified `Panel`
/// API), and the central panel must be added last (egui's own panel-ordering rule).
pub fn render(app: &mut GuiApp, ui: &mut egui::Ui) {
    // 007-gui-edit polish (critique follow-up): a windowed app's only equivalent of the TUI's
    // Esc-cancel is a real key binding, not just an outcome the doc comments described but
    // nothing wired up — dismiss whichever single write flow is open (Constitution III: only one
    // is ever open at a time), edit taking priority since it's the one opened over an existing
    // picker (`open_edit` closes `add_picker`, never the reverse).
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        if app.edit.is_some() {
            app.cancel_edit();
        } else if app.add_picker.is_some() {
            app.cancel_add();
        }
    }

    egui::Panel::top("lookup_bar").show(ui, |ui| {
        egui::Frame::new()
            .inner_margin(PANEL_MARGIN)
            .show(ui, |ui| {
                render_lookup_bar(app, ui);
            });
    });

    egui::Panel::bottom("status_bar").show(ui, |ui| {
        egui::Frame::new()
            .inner_margin(PANEL_MARGIN)
            .show(ui, |ui| {
                // Critique follow-up: a refused write and a completed one used to render as the
                // same muted gray; StatusKind now carries which one this is.
                let color = match app.status_kind {
                    StatusKind::Info => theme::TEXT_MUTED,
                    StatusKind::Success => theme::ACCENT,
                    StatusKind::Error => theme::ERROR,
                };
                ui.label(egui::RichText::new(app.status.clone()).color(color));
            });
    });

    egui::CentralPanel::default().show(ui, |ui| {
        egui::Frame::new()
            .inner_margin(PANEL_MARGIN)
            .show(ui, |ui| {
                render_sign_in(app, ui);
                ui.separator();
                ui.add_space(4.0);
                render_results(app, ui);
                if app.add_picker.is_some() {
                    ui.separator();
                    render_add_picker(app, ui);
                }
                if app.edit.is_some() {
                    ui.separator();
                    render_edit(app, ui);
                }
            });
    });
}

fn render_lookup_bar(app: &mut GuiApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label("Roblox username:");
        let response = ui.text_edit_singleline(&mut app.username_input);
        let submitted = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let look_up_clicked = ui
            .add_enabled(!app.syncing, egui::Button::new("Look Up"))
            .clicked();
        // 005-gui-refresh: re-runs the same sync the app already performs once at startup, and
        // is also the clerk's recovery path after a failed startup sync (spec FR-004).
        if ui
            .add_enabled(!app.syncing, egui::Button::new("Refresh"))
            .clicked()
        {
            app.start_sync();
        }
        if app.syncing {
            ui.label("Syncing...");
        }
        if !app.syncing && (submitted || look_up_clicked) {
            app.submit_lookup();
        }
    });
}

fn render_sign_in(app: &mut GuiApp, ui: &mut egui::Ui) {
    match app.auth {
        // Critique follow-up: previously rendered nothing at all once signed in — a clerk had no
        // way to see who a write would be attributed to, or to end the session. `can_sign_out` is
        // false under a service account (a shared standing credential, not a per-session login).
        AuthState::SignedIn => {
            ui.horizontal(|ui| {
                match &app.account_label {
                    Some(label) => {
                        ui.label(egui::RichText::new(format!("Signed in as {label}")).color(theme::TEXT_MUTED));
                    }
                    None => {
                        ui.label(egui::RichText::new("Signed in").color(theme::TEXT_MUTED));
                    }
                }
                if app.can_sign_out && ui.button("Sign Out").clicked() {
                    app.sign_out();
                }
            });
        }
        AuthState::SigningIn => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Signing in...");
            });
        }
        AuthState::Unknown | AuthState::SignedOut => {
            if ui.button("Sign In").clicked() {
                app.start_sign_in();
            }
        }
    }
}

fn render_results(app: &mut GuiApp, ui: &mut egui::Ui) {
    let Some(looked_up) = app.looked_up.clone() else {
        ui.label("Look up a username to see their current awards.");
        return;
    };

    if looked_up.not_found {
        ui.label(format!("No records found for \"{}\".", looked_up.username));
        return;
    }

    ui.heading(&looked_up.username);
    ui.add_space(6.0);

    // 007-gui-edit polish: one column per category, side by side, so a wide window spreads the
    // three category groups across the available width instead of stacking every award down a
    // single left-hand column (006-gui-visual-theme's palette already made headings/body read
    // clearly — this is the layout half of "looks deliberate", not new state or behavior).
    let mut edit_clicked: Option<Award> = None;
    ui.columns(CATEGORY_LABELS.len(), |columns| {
        for (i, (category, label)) in CATEGORY_LABELS.iter().enumerate() {
            let in_category: Vec<_> = looked_up
                .awards
                .iter()
                .filter(|a| a.category == *category)
                .collect();
            if in_category.is_empty() {
                continue;
            }
            let col = &mut columns[i];
            col.label(egui::RichText::new(*label).strong().color(theme::ACCENT));
            col.add_space(2.0);
            for award in in_category {
                col.horizontal(|ui| {
                    ui.label(format!("• {}", award.name));
                    if ui.small_button("Edit").clicked() {
                        edit_clicked = Some(award.clone());
                    }
                });
            }
        }
    });
    if let Some(award) = edit_clicked {
        app.open_edit(award);
    }

    ui.add_space(8.0);
    let can_add = app.data.is_some();
    if ui
        .add_enabled(can_add, egui::Button::new("Add Award"))
        .clicked()
    {
        app.open_add_picker();
    }
}

fn render_add_picker(app: &mut GuiApp, ui: &mut egui::Ui) {
    ui.heading("Add Award");

    let mut filter_text = app
        .add_picker
        .as_ref()
        .map(|p| p.filter.clone())
        .unwrap_or_default();
    if ui.text_edit_singleline(&mut filter_text).changed() {
        app.update_filter(filter_text);
    }

    let filtered = app
        .add_picker
        .as_ref()
        .map(|p| p.filtered.clone())
        .unwrap_or_default();
    let selected = app.add_picker.as_ref().and_then(|p| p.selected.clone());

    egui::ScrollArea::vertical()
        .max_height(200.0)
        .show(ui, |ui| {
            for def in &filtered {
                let is_selected = selected.as_ref() == Some(def);
                if ui.selectable_label(is_selected, &def.base_name).clicked() {
                    app.select(def.clone());
                }
            }
        });

    let mut suffix_text = app
        .add_picker
        .as_ref()
        .map(|p| p.suffix.clone())
        .unwrap_or_default();
    ui.horizontal(|ui| {
        ui.label("Suffix (optional):");
        if ui.text_edit_singleline(&mut suffix_text).changed() {
            app.set_suffix(suffix_text);
        }
    });

    ui.horizontal(|ui| {
        let submitting = app
            .add_picker
            .as_ref()
            .map(|p| p.submitting)
            .unwrap_or(false);
        if submitting {
            ui.spinner();
        }
        if ui
            .add_enabled(app.can_confirm_add(), egui::Button::new("Confirm"))
            .clicked()
        {
            app.confirm_add();
        }
        if ui.add(secondary_button("Cancel")).clicked() {
            app.cancel_add();
        }
        if app.auth != AuthState::SignedIn {
            ui.label("Sign in to add awards.");
        }
    });
}

/// 007-gui-edit: corrects one award's raw cell text in place — the GUI counterpart of the TUI's
/// `Modal::Edit` (single Enter to submit, no typed confirmation phrase, since this isn't a
/// destructive write — Constitution III reserves that gate for delete/rename).
fn render_edit(app: &mut GuiApp, ui: &mut egui::Ui) {
    ui.heading("Edit Award");

    let award_name = app
        .edit
        .as_ref()
        .map(|e| e.award.name.clone())
        .unwrap_or_default();
    ui.label(egui::RichText::new(award_name).color(theme::TEXT_MUTED));

    let mut input_text = app
        .edit
        .as_ref()
        .map(|e| e.input.clone())
        .unwrap_or_default();
    let response = ui.text_edit_singleline(&mut input_text);
    if response.changed() {
        app.set_edit_input(input_text);
    }
    let submitted = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

    ui.horizontal(|ui| {
        let submitting = app.edit.as_ref().map(|e| e.submitting).unwrap_or(false);
        if submitting {
            ui.spinner();
        }
        let save_clicked = ui
            .add_enabled(app.can_confirm_edit(), egui::Button::new("Save"))
            .clicked();
        if ui.add(secondary_button("Cancel")).clicked() {
            app.cancel_edit();
        }
        if app.auth != AuthState::SignedIn {
            ui.label("Sign in to edit awards.");
        }
        if save_clicked || (submitted && app.can_confirm_edit()) {
            app.confirm_edit();
        }
    });
}
