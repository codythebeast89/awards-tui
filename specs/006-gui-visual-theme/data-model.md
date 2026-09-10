# Data Model: GUI Visual Theme

Phase 1 output for `specs/006-gui-visual-theme/plan.md`. This feature has no domain entities — its
"data model" is the design-token set research.md §2 already named, plus how each token maps onto
`egui::Visuals`/`egui::Style` fields. No `struct`, `enum`, or state transition is added anywhere;
`GuiApp` (data-model.md of `004-gui-lookup-add`) is untouched.

## Design tokens → `egui` style fields

| Token | `egui::Visuals`/`Style` field(s) |
|---|---|
| `BG` | `visuals.panel_fill`, `visuals.window_fill` |
| `BG_RAISED` | `visuals.faint_bg_color` (the award picker's `ScrollArea` sits on this) |
| `BG_FIELD` | `visuals.extreme_bg_color` (drives `TextEdit` backgrounds) |
| `TEXT` | `visuals.widgets.noninteractive.fg_stroke.color`, and the default color `RichText` inherits when none is set explicitly |
| `TEXT_MUTED` | `visuals.widgets.noninteractive.bg_stroke.color`; also the color status/secondary labels use explicitly in `ui.rs` |
| `ACCENT` | `visuals.selection.bg_fill`, `visuals.hyperlink_color`, `visuals.widgets.inactive.weak_bg_fill` (enabled-button fill — confirmed via `egui-0.36.2`'s `Style::button_style()`, which builds `ButtonStyle.frame.fill` from `weak_bg_fill`, not `bg_fill`), and the explicit color category headings in `ui.rs` pass to `RichText::color()` |
| `ACCENT_HOVER` | `visuals.widgets.hovered.weak_bg_fill` |
| `BORDER` | `visuals.widgets.noninteractive.bg_stroke.color`, `visuals.window_stroke.color` |
| `ERROR` | Named in `theme.rs`, unused by this feature's own widgets — reserved for `007-gui-edit` and later features' failure-state styling (research.md §2) |

## `theme.rs`'s public surface

```rust
// crates/awards-gui/src/theme.rs
pub const BG: egui::Color32 = ...;
pub const BG_RAISED: egui::Color32 = ...;
pub const BG_FIELD: egui::Color32 = ...;
pub const TEXT: egui::Color32 = ...;
pub const TEXT_MUTED: egui::Color32 = ...;
pub const ACCENT: egui::Color32 = ...;
pub const ACCENT_HOVER: egui::Color32 = ...;
pub const BORDER: egui::Color32 = ...;
pub const ERROR: egui::Color32 = ...;

/// Builds and applies this application's Visuals + Style to `ctx`. Called once, in `main.rs`,
/// before the first frame — see research.md §1.
pub fn apply(ctx: &egui::Context);
```

Every constant is `pub` so `ui.rs` (and later, `007-gui-edit`'s own rendering code) can reference a
token by name — e.g. category headings using `egui::RichText::new(label).strong().color(theme::ACCENT)`
— rather than a file other than `theme.rs` ever writing a raw `Color32::from_rgb(...)` literal
(spec FR-006).
