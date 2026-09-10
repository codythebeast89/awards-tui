# Implementation Plan: GUI Edit

**Branch**: `007-gui-edit` | **Date**: 2026-09-10 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/007-gui-edit/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command; its definition describes the execution workflow.

## Summary

Add an `EditFlow` state and its own inline panel to `awards-gui`, letting a clerk correct one
award's raw cell text from the results view. Reuses `awards-sheets::update_award_cell` — the same
guarded write function the terminal tool's own Edit modal calls — unchanged. No new dependency,
no new crate, no change to Lookup/Refresh/Add's own behavior. Paired in this same change with a
layout pass on the results view (a three-column category layout plus consistent panel margins),
addressing feedback that the single-column, unpadded layout from 006-gui-visual-theme left the
window's width unused — a presentation-only change riding along with Edit's own new per-award
control, not a separately-specified feature.

## Technical Context

**Language/Version**: Rust, 2021 edition — unchanged.

**Primary Dependencies**: None added — `update_award_cell` already exists in `awards-sheets`
(used by the terminal tool since before this GUI existed) and is simply imported for the first
time by `awards-gui`.

**Storage**: N/A — no new persisted state; `EditFlow` lives only in memory, like `AddPicker`.

**Testing**: Every new `GuiApp` state transition (`open_edit`, `set_edit_input`, `cancel_edit`,
`can_confirm_edit`, `confirm_edit`'s no-op guards, `handle_edit_done`'s success/failure/reassignment
branches, and the two new mutual-exclusivity interactions with `open_add_picker`/`submit_lookup`)
has a corresponding unit test in `app.rs`, following the same headless-testable pattern
004-gui-lookup-add and 005-gui-refresh established. `ui.rs` remains untested directly (unchanged
rationale: no display in this environment).

**Target Platform**: Unchanged.

**Project Type**: Unchanged — edits to the existing `awards-gui` crate, no new workspace member.

**Performance Goals**: N/A — a single guarded write per Save, identical in cost/shape to Add's.

**Constraints**: Edit MUST call the exact same `update_award_cell` the terminal tool uses, with
`interactive_auth: false` — no new or differently-guarded write path (spec FR-003).

**Scale/Scope**: Two file edits (`app.rs`, `ui.rs`), no new file, no new crate.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Gate | Status | Evidence |
|---|---|---|---|
| I. Code Quality | Workspace boundaries preserved; no `unsafe`; no speculative deps; typed errors at library boundaries | **PASS** | No new dependency, no new crate. `update_award_cell` already lives in `awards-sheets` with its existing `EditResult`/`EditError` typed-error shape; `awards-gui` only calls it. |
| II. Testing Standards | New/changed modal transitions tested; network code offline-testable; CI runs `--locked` test+clippy | **PASS** | 14 new `app.rs` unit tests cover every new state transition offline (no network); `cargo test --workspace --locked` and `cargo clippy --workspace --all-targets --locked -- -D warnings` both clean. |
| III. UX Consistency | Esc-cancel convention; typed confirm phrase for destructive writes; CLI/TUI parity; theme-only colors | **PASS** | Edit is not destructive (a correction, not a delete/rename) — no typed confirmation phrase required, matching the terminal tool's own single-Enter Edit modal (research.md §2). Sign-in gating deliberately goes further than the TUI's own Edit (which has no explicit auth check) — documented as a GUI-specific hardening consistent with Add's already-established pattern, not a parity gap (research.md §3). All new UI colors come from `theme.rs` (006-gui-visual-theme) — no new `Color32` literal is introduced. |
| IV. Performance | Concurrent independent fetches; explicit HTTP timeouts; non-blocking writes | **PASS** | The edit write runs on a background thread via the same `mpsc`/`thread::spawn` pattern Add already uses; the UI thread is never blocked. |
| Security & Data Integrity | Secrets gitignored; live stale-write re-check; OAuth `state` CSRF check | **PASS** | `update_award_cell` already performs its own live-cell recheck (`find_live_row` + cell-value comparison) before writing — unchanged, reused as-is. |

No violations requiring justification.

## Project Structure

### Documentation (this feature)

```text
specs/007-gui-edit/
├── plan.md               # This file (/speckit-plan command output)
├── research.md           # Phase 0 output (/speckit-plan command)
├── data-model.md         # Phase 1 output (/speckit-plan command)
├── contracts/
│   └── gui-edit-interaction.md
└── quickstart.md         # Phase 1 output (/speckit-plan command)
```

### Source Code (repository root)

```text
crates/awards-gui/src/
├── app.rs   # EDIT: EditFlow, GuiMsg::EditDone, open_edit/set_edit_input/cancel_edit/
│            #       can_confirm_edit/confirm_edit/handle_edit_done, small mutual-exclusivity
│            #       additions to open_add_picker/submit_lookup
└── ui.rs    # EDIT: render_edit (new), render_results gains a per-award Edit control and a
             #       three-column category layout plus consistent panel margins (layout polish)
```

**Structure Decision**: Two edits to the existing crate, no new file, no new crate — the smallest
structure that adds Edit without touching Lookup/Refresh/Add's own code paths.

## Complexity Tracking

*No entries — Constitution Check reported no violations requiring justification.*
