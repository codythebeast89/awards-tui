# Implementation Plan: GUI Visual Theme

**Branch**: `006-gui-visual-theme` | **Date**: 2026-09-10 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/006-gui-visual-theme/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command; its definition describes the execution workflow.

## Summary

Add a single, self-contained `theme.rs` module to `awards-gui` that builds one `egui::Visuals` +
`egui::Style` (colors, spacing, rounding, and text sizes) and applies it once, at startup, before
the first frame renders. Every existing panel (`ui.rs`'s top bar, results, award picker, status
bar) already renders through `egui`'s panel/widget machinery, so setting the context-wide style
once makes every one of them consistent automatically (spec FR-001, FR-004) — no per-widget color
literal is added anywhere in `ui.rs`. Purely presentational: no function in `app.rs` changes, no
message, no state, no new dependency.

## Technical Context

**Language/Version**: Rust, 2021 edition — unchanged.

**Primary Dependencies**: None added — `egui::Visuals`/`egui::Style` are part of `eframe`'s
existing `egui` re-export, already a dependency since `004-gui-lookup-add`. No new font asset: the
theme changes text *sizes* via `style.text_styles`, not the font family, so no font file needs
loading or bundling.

**Storage**: N/A — no new state, no new file read at runtime. The palette is a compile-time
constant module, not a config file (spec Assumptions: no user-facing theme switcher this
milestone).

**Testing**: `theme.rs`'s `apply()` function takes an `&egui::Context` and returns nothing
testable in the constitution's usual sense (no state transition to assert) — consistent with
`004-gui-lookup-add`'s own precedent that rendering code is thin and not directly unit-tested
(that plan's Testing section). What *is* tested: every existing `awards-gui` unit test in `app.rs`
must keep passing completely unchanged, proving this feature touched no behavior (spec FR-005).

**Target Platform**: Unchanged.

**Project Type**: Unchanged — one new file (`crates/awards-gui/src/theme.rs`) in the existing
crate, no new workspace member.

**Performance Goals**: N/A — style is computed once at startup, not per frame.

**Constraints**: The palette MUST be defined in exactly one place (spec FR-006) — `theme.rs` is
that place; no other file may introduce its own `egui::Color32` literal for a role this module
already names.

**Scale/Scope**: One new file, one call site (`main.rs`, before `eframe::run_native`'s app is
built). No change to `app.rs`. Small edits to `ui.rs` only where a color needs to be *referenced*
by name (category headings) rather than left to the ambient style.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Gate | Status | Evidence |
|---|---|---|---|
| I. Code Quality | Workspace boundaries preserved; no `unsafe`; no speculative deps; typed errors at library boundaries | **PASS** | No new dependency, no new crate, no I/O. `theme.rs` is pure presentation, same role as `ui.rs`. |
| II. Testing Standards | New/changed modal transitions tested; network code offline-testable; CI runs `--locked` test+clippy | **PASS** | No state or network code is added or changed; every existing `app.rs` test must still pass unchanged, which is this feature's actual regression gate. |
| III. UX Consistency | Esc-cancel convention; typed confirm phrase for destructive writes; CLI/TUI parity; theme-only colors | **PASS, with one documented, deliberate divergence** | The constitution's "colors MUST come from the `Theme` struct / `awards-tui.toml`" rule is written for the *terminal* tool's rendering code and names that specific configuration path. This feature deliberately does not read or derive from `awards-tui.toml` — the user explicitly chose a GUI-specific palette distinct from the TUI's (spec Assumptions), which `004-gui-lookup-add`'s own plan.md already flagged as a deferred design-system decision left for "a later feature once more GUI surface exists to make one meaningful." This feature satisfies the *principle behind* the rule — one named, single-sourced palette, not scattered ad-hoc literals (spec FR-006) — for the GUI's own single source of truth (`theme.rs`), just not the *same* source the TUI uses. As with `004`'s Esc-cancel gap, this is a **Constitution scope gap, not a feature violation**: the rule predates a second front-end with its own deliberately distinct visual identity. Widening the rule's wording to acknowledge two single-sourced palettes (one per front-end) is a governance question for a future `/speckit-constitution` pass, not resolved unilaterally here. |
| IV. Performance | Concurrent independent fetches; explicit HTTP timeouts; non-blocking writes | **N/A** | No network code in this feature. |
| Security & Data Integrity | Secrets gitignored; live stale-write re-check; OAuth `state` CSRF check | **N/A** | No credential, write, or OAuth code in this feature. |

No violations requiring justification beyond the documented, non-blocking Constitution-scope
observation above (mirroring the same class of observation `004-gui-lookup-add`'s plan.md already
made and left to a future governance pass). Complexity Tracking table is omitted (N/A).

## Project Structure

### Documentation (this feature)

```text
specs/006-gui-visual-theme/
├── plan.md               # This file (/speckit-plan command output)
├── research.md           # Phase 0 output (/speckit-plan command)
├── data-model.md         # Phase 1 output (/speckit-plan command) — design tokens, not data entities
└── quickstart.md         # Phase 1 output (/speckit-plan command)
```

No `contracts/` directory: this feature adds no new interaction and changes no existing one (spec
FR-005) — there is nothing for an interaction contract to describe that `004`'s and `005`'s
existing contracts don't already cover unchanged.

### Source Code (repository root)

```text
crates/awards-gui/src/
├── theme.rs   # NEW: apply(ctx: &egui::Context) — builds and sets one Visuals + Style
├── main.rs    # EDIT: call theme::apply(&cc.egui_ctx) before constructing GuiApp
└── ui.rs      # EDIT: category headings reference theme::ACCENT by name instead of ui.strong()'s default color
```

**Structure Decision**: One new file, two small edits, no new crate, no workspace change — the
smallest structure that gives the palette exactly one source of truth (spec FR-006).

## Complexity Tracking

*No entries — Constitution Check reported no violations requiring justification.*
