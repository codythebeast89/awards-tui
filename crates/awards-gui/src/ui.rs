//! `egui` rendering for `awards-gui`. Thin by design (mirrors `awards-tui/src/tui/ui.rs`'s role):
//! every widget here reads or drives `GuiApp` state (`app.rs`) and contains no business logic of
//! its own — parsing, matching, and the write path all live in `awards-core`/`awards-sheets` and
//! are called from `app.rs`, never from here. Not directly unit-tested (plan.md Testing section)
//! since this environment cannot open a display; `app.rs`'s tests cover the state this renders.

use crate::app::{AuthState, GuiApp};
use crate::theme;
use awards_core::CATEGORY_LABELS;
use eframe::egui;

/// Renders the whole window (contract: "single window, no modal dialogs" — the award picker is
/// an inline panel, not a separate window). `ui` is the root `Ui` eframe hands to `App::ui` for
/// this pass — top-level panels attach to it directly (egui 0.36's unified `Panel` API), and the
/// central panel must be added last (egui's own panel-ordering rule).
pub fn render(app: &mut GuiApp, ui: &mut egui::Ui) {
    egui::Panel::top("lookup_bar").show(ui, |ui| {
        render_lookup_bar(app, ui);
    });

    egui::Panel::bottom("status_bar").show(ui, |ui| {
        ui.label(egui::RichText::new(app.status.clone()).color(theme::TEXT_MUTED));
    });

    egui::CentralPanel::default().show(ui, |ui| {
        render_sign_in(app, ui);
        ui.separator();
        render_results(app, ui);
        if app.add_picker.is_some() {
            ui.separator();
            render_add_picker(app, ui);
        }
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
        AuthState::SignedIn => {}
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
    for (category, label) in CATEGORY_LABELS {
        let in_category: Vec<_> = looked_up
            .awards
            .iter()
            .filter(|a| a.category == *category)
            .collect();
        if in_category.is_empty() {
            continue;
        }
        ui.label(egui::RichText::new(*label).strong().color(theme::ACCENT));
        for award in in_category {
            ui.label(format!("  • {}", award.name));
        }
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
        if ui.button("Cancel").clicked() {
            app.cancel_add();
        }
        if app.auth != AuthState::SignedIn {
            ui.label("Sign in to add awards.");
        }
    });
}
