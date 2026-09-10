//! The GUI's single visual identity — one dark, warm-neutral palette with a brass accent.
//!
//! `apply()` is called exactly once, in `main.rs`, before the first frame renders (see
//! `specs/006-gui-visual-theme/research.md` §1). Every panel in `ui.rs` then inherits this
//! look automatically through `egui`'s ambient `Style`; only category headings and the status
//! bar reference a token by name explicitly (spec FR-002, FR-006). No other file in this crate
//! may introduce its own `Color32` literal for a role named here.

use eframe::egui;

/// Window/panel background.
pub const BG: egui::Color32 = egui::Color32::from_rgb(0x1B, 0x1E, 0x22);
/// The award picker's surface — one step "up" from `BG`, same family.
pub const BG_RAISED: egui::Color32 = egui::Color32::from_rgb(0x24, 0x28, 0x2D);
/// Text-input backgrounds.
pub const BG_FIELD: egui::Color32 = egui::Color32::from_rgb(0x14, 0x16, 0x1A);
/// Primary text.
pub const TEXT: egui::Color32 = egui::Color32::from_rgb(0xE8, 0xE6, 0xDF);
/// Secondary/status text, and the base a disabled control fades toward.
pub const TEXT_MUTED: egui::Color32 = egui::Color32::from_rgb(0x8A, 0x8F, 0x97);
/// Category headings, enabled-button fill, selection highlight.
pub const ACCENT: egui::Color32 = egui::Color32::from_rgb(0xC9, 0xA2, 0x27);
/// Hovered interactive elements.
pub const ACCENT_HOVER: egui::Color32 = egui::Color32::from_rgb(0xDF, 0xC2, 0x4A);
/// Widget outlines/separators.
pub const BORDER: egui::Color32 = egui::Color32::from_rgb(0x3A, 0x3F, 0x46);
/// Reserved for `007-gui-edit` and later features' failure-state styling — unused by this
/// feature's own widgets (research.md §2), named now so a later feature has one red to pull
/// from rather than inventing a second (spec FR-006).
#[allow(dead_code)]
pub const ERROR: egui::Color32 = egui::Color32::from_rgb(0xC0, 0x45, 0x3A);

/// Builds and applies this application's `Visuals` + `Style` to `ctx`. Called once, in
/// `main.rs`, before the first frame — see `specs/006-gui-visual-theme/research.md` §1.
pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();

    visuals.override_text_color = Some(TEXT);
    visuals.hyperlink_color = ACCENT;
    visuals.faint_bg_color = BG_RAISED;
    visuals.extreme_bg_color = BG_FIELD;
    visuals.window_fill = BG;
    visuals.panel_fill = BG;
    visuals.window_stroke = egui::Stroke::new(1.0, BORDER);

    visuals.selection.bg_fill = ACCENT;
    visuals.selection.stroke = egui::Stroke::new(1.0, BG);

    // Non-interactive surfaces (window/panel backgrounds, the normal text color).
    visuals.widgets.noninteractive.bg_fill = BG;
    visuals.widgets.noninteractive.weak_bg_fill = BG;
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, BORDER);
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, TEXT);

    // Interactive widgets at rest (buttons, text edits) — `weak_bg_fill` is what `Button`
    // actually paints (confirmed against egui 0.36.2's `Style::button_style()`); `bg_fill`
    // is reserved for widgets that must always show a background, like sliders/checkboxes.
    visuals.widgets.inactive.bg_fill = BG_FIELD;
    visuals.widgets.inactive.weak_bg_fill = ACCENT;
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, BORDER);
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, BG);

    // Hovered.
    visuals.widgets.hovered.bg_fill = BG_FIELD;
    visuals.widgets.hovered.weak_bg_fill = ACCENT_HOVER;
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, ACCENT_HOVER);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, BG);

    // Active (pressed/dragged/focused).
    visuals.widgets.active.bg_fill = BG_FIELD;
    visuals.widgets.active.weak_bg_fill = ACCENT_HOVER;
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.5, ACCENT_HOVER);
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, BG);

    // Open (e.g. an expanded ComboBox) — mirror `active` for consistency.
    visuals.widgets.open.bg_fill = BG_FIELD;
    visuals.widgets.open.weak_bg_fill = ACCENT;
    visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0, ACCENT);
    visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, BG);

    // This app has no theme switcher and no OS dark/light detection (spec Assumptions) — force
    // dark and apply the same fixed `visuals` to both `egui::Theme` slots, so nothing about the
    // host OS's own light/dark setting can leak a different palette in.
    ctx.set_theme(egui::Theme::Dark);
    ctx.set_visuals_of(egui::Theme::Dark, visuals.clone());
    ctx.set_visuals_of(egui::Theme::Light, visuals);

    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(10.0, 10.0);
        style.spacing.button_padding = egui::vec2(12.0, 6.0);

        if let Some(heading) = style.text_styles.get_mut(&egui::TextStyle::Heading) {
            heading.size = 24.0;
        }
        if let Some(button) = style.text_styles.get_mut(&egui::TextStyle::Button) {
            button.size = 15.0;
        }
    });
}
