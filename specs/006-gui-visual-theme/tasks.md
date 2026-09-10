---

description: "Task list template for feature implementation"
---

# Tasks: GUI Visual Theme

**Input**: Design documents from `/specs/006-gui-visual-theme/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, quickstart.md (all present; no contracts/ — see plan.md Project Structure)

**Tests**: No new state or network code is added; the regression gate is "every existing test still passes," per plan.md's Testing section.

**Organization**: A single user story — no Setup/Foundational phase needed, no new crate or dependency.

## Format: `[ID] [P?] [Story] Description`

- **[Story]**: `US1` (the only user story)

## Path Conventions

One new file, two edits: `crates/awards-gui/src/theme.rs` (new), `crates/awards-gui/src/main.rs`, `crates/awards-gui/src/ui.rs`.

---

## Phase 1: User Story 1 - Open a visually coherent, polished application (Priority: P1)

**Goal**: One consistent, deliberately-designed palette applied across the whole window.

**Independent Test**: Launch the app and visually confirm consistent colors, clear category/body distinction, and clear enabled/disabled contrast, across the top bar, results, picker, and status bar.

- [X] T001 [US1] Create `crates/awards-gui/src/theme.rs` with the nine named `egui::Color32` constants from data-model.md (`BG`, `BG_RAISED`, `BG_FIELD`, `TEXT`, `TEXT_MUTED`, `ACCENT`, `ACCENT_HOVER`, `BORDER`, `ERROR`)
- [X] T002 [US1] Implement `theme::apply(ctx: &egui::Context)` in `crates/awards-gui/src/theme.rs`: build an `egui::Visuals` (dark-based) mapping each token per data-model.md's field table (`panel_fill`/`window_fill` from `BG`, `faint_bg_color` from `BG_RAISED`, `extreme_bg_color` from `BG_FIELD`, `selection.bg_fill`/`hyperlink_color`/`widgets.inactive.weak_bg_fill` from `ACCENT`, `widgets.hovered.weak_bg_fill` from `ACCENT_HOVER`, stroke colors from `BORDER`/`TEXT`), widen `Spacing::item_spacing` and `Spacing::button_padding`, bump `TextStyle::Heading` and `TextStyle::Button` sizes, and call `ctx.set_visuals_of(..)` / `ctx.all_styles_mut(..)` (research.md §1, §3 — corrected from `ctx.set_visuals`/`set_style`, which egui 0.36.2 no longer exposes as inherent methods, and from `bg_fill` to `weak_bg_fill` for button fill after confirming against egui 0.36.2's own `Style::button_style()` source)
- [X] T003 [US1] Wire `theme::apply(&cc.egui_ctx)` into `crates/awards-gui/src/main.rs`'s `eframe::run_native` closure, before `GuiApp::new` is constructed
- [X] T004 [US1] Update the category-heading line in `crates/awards-gui/src/ui.rs`'s `render_results` to pass `.color(theme::ACCENT)` on the existing `RichText::new(*label).strong()` call (spec FR-002)
- [X] T005 [US1] Update the status-bar label in `crates/awards-gui/src/ui.rs` to use `theme::TEXT_MUTED` (secondary text, distinguishing it from the primary-text results above it)
- [X] T006 [US1] Confirm `cargo build -p awards-gui --locked` succeeds and `cargo clippy -p awards-gui --locked --all-targets -- -D warnings` is clean

**Checkpoint**: The whole window renders through one named palette; nothing in `ui.rs` sets a raw `Color32` literal outside `theme.rs`.

---

## Phase 2: Polish & Cross-Cutting Concerns

- [X] T007 Run `cargo test --workspace --locked` and `cargo clippy --workspace --all-targets --locked -- -D warnings`, confirming every existing test (including all of `004`'s and `005`'s) passes completely unchanged (spec FR-005)
- [ ] T008 Manually execute `specs/006-gui-visual-theme/quickstart.md`'s four scenarios on a machine with a display — leave unchecked until run live

---

## Dependencies & Execution Order

T001 before T002 (constants must exist before `apply()` references them). T002 before T003 (the function must exist before it's called). T004/T005 are independent of T001-T003's wiring order but depend on T001's constants existing to reference. T006 depends on T001-T005. Polish (T007-T008) depends on everything above.

## Implementation Strategy

Single story — implement T001 through T006 in order, verify with T007, and T008 is the live-display follow-up every prior feature's tasks.md has ended with.

## Notes

- No task in this list adds a dependency, a crate, a new write path, or any new state — this feature is exactly one new presentation-only file plus two small edits to existing presentation code.
