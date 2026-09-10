---

description: "Task list template for feature implementation"
---

# Tasks: GUI Edit

**Input**: Design documents from `/specs/007-gui-edit/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md (all present)

**Note on ordering**: Implementation for this feature was built directly (in response to the
clerk's explicit "start the edit" instruction, alongside a requested layout polish pass) and the
planning docs written immediately after to preserve this project's established spec→plan→tasks→
implement traceability, rather than strictly before. Every task below reflects what was actually
built and verified, not a forward plan.

**Tests**: New `GuiApp` state (per data-model.md) is unit-tested; `ui.rs` remains untested
directly per this crate's established Testing precedent (no display in this build environment).

**Organization**: A single user story — no Setup/Foundational phase needed, no new crate or
dependency.

## Format: `[ID] [P?] [Story] Description`

- **[Story]**: `US1` (the only user story)

## Path Conventions

Two edited files: `crates/awards-gui/src/app.rs`, `crates/awards-gui/src/ui.rs`.

---

## Phase 1: User Story 1 - Correct a malformed or outdated award entry (Priority: P1)

**Goal**: A clerk can correct one award's raw cell text from the results view, through the same
guarded write path the terminal tool's own Edit action uses.

**Independent Test**: Look up a user, edit one award's text, save, and confirm the results view
reflects the change without a full re-sync.

- [X] T001 [US1] Add `EditFlow { award, input, submitting }` and `GuiMsg::EditDone { username, result }` to `crates/awards-gui/src/app.rs` (data-model.md)
- [X] T002 [US1] Add `pub edit: Option<EditFlow>` to `GuiApp`, initialized `None` in both `GuiApp::new` and the test helper `no_data_app`
- [X] T003 [US1] Implement `open_edit(award)` in `crates/awards-gui/src/app.rs`: prefill `input` from `award.cell` (or `award.name` if empty), clear `add_picker` (spec FR-001, FR-007; research.md §1, §5)
- [X] T004 [US1] Implement `set_edit_input`, `cancel_edit`, `can_confirm_edit` (gated on `SignedIn` + non-empty trimmed input + not already submitting) in `crates/awards-gui/src/app.rs` (spec FR-002; research.md §3)
- [X] T005 [US1] Implement `confirm_edit`, spawning a background thread that calls `update_award_cell(&award, &new_cell, false)` and sends `GuiMsg::EditDone` (spec FR-003)
- [X] T006 [US1] Implement `handle_edit_done`: on success, `upsert_award_in_index` then recompute the viewed user's award list via `get_awards_for_username`, closing `edit` and noting "no longer under @user" if the award moved away; on failure, reset `submitting` and keep `edit`/`input` intact (spec FR-004, FR-005, FR-006; research.md §4)
- [X] T007 [US1] Wire `GuiMsg::EditDone` into `handle_msg`'s dispatch in `crates/awards-gui/src/app.rs`
- [X] T008 [US1] Add `self.edit = None` to `submit_lookup` and `open_add_picker` for mutual exclusivity (spec FR-007; research.md §5)
- [X] T009 [US1] Add 14 unit tests in `crates/awards-gui/src/app.rs` covering: prefill from cell / fallback to name, mutual exclusivity with Add in both directions, `set_edit_input`, `cancel_edit`, `can_confirm_edit`'s three false-cases, `confirm_edit`'s signed-out no-op, `handle_edit_done`'s success/reassignment/failure branches, and a fresh lookup closing an open edit
- [X] T010 [US1] Add a per-award "Edit" control in `crates/awards-gui/src/ui.rs`'s `render_results` (calls `app.open_edit(award.clone())`)
- [X] T011 [US1] Implement `render_edit` in `crates/awards-gui/src/ui.rs`: heading, read-only award name, text field bound to `edit.input`, Save (enabled by `can_confirm_edit`, also triggered by Enter), Cancel, sign-in hint, spinner while submitting
- [X] T012 [US1] Wire `render_edit` into `render`'s central panel, shown when `app.edit.is_some()` (mirrors the existing `add_picker` panel)
- [X] T013 [US1] Confirm `cargo build -p awards-gui --locked` succeeds and `cargo clippy -p awards-gui --locked --all-targets -- -D warnings` is clean

**Checkpoint**: Edit works end-to-end against the same write path the terminal tool uses, with no
change to Lookup/Refresh/Add's existing behavior.

---

## Phase 2: Layout polish (requested alongside this feature, not a separate spec)

- [X] T014 Add a `PANEL_MARGIN` constant and wrap the top bar, status bar, and central panel's
      content in `egui::Frame::new().inner_margin(PANEL_MARGIN)` in `crates/awards-gui/src/ui.rs`,
      so panel content no longer sits flush against the window edge
- [X] T015 Change `render_results` in `crates/awards-gui/src/ui.rs` to lay out the three award
      categories (Badges/Ribbons/Foreign Awards) as side-by-side columns via `ui.columns(..)`
      instead of one stacked column, so a wide window spreads content across its width

**Checkpoint**: The results view uses the window's available width instead of stacking everything
against the left edge, with no change to what any control does.

---

## Phase 3: Polish & Cross-Cutting Concerns

- [X] T016 Run `cargo test --workspace --locked` and `cargo clippy --workspace --all-targets --locked -- -D warnings`, confirming every existing test (including all of `004`'s, `005`'s, and `006`'s) passes completely unchanged, plus the 14 new Edit tests (spec FR-008)
- [ ] T017 Manually execute `specs/007-gui-edit/quickstart.md`'s six scenarios on a machine with a display — leave unchecked until run live

---

## Dependencies & Execution Order

T001-T002 before T003-T008 (state must exist before the methods that use it). T003-T008 before
T009 (tests exercise the methods). T010-T012 depend on T001-T009's `app.rs` surface existing.
T013 depends on T001-T012. T014-T015 are independent of T001-T013 (pure `ui.rs` layout, no new
state) but were done in the same pass. T016 depends on everything above. T017 depends on T016.

## Implementation Strategy

Single story — T001 through T013 implement and verify Edit; T014-T015 are the layout polish done
alongside it; T016 is the workspace-wide regression gate; T017 is the live-display follow-up
every prior feature's tasks.md has ended with.

## Notes

- No task in this list adds a dependency, a new crate, or a new write function — Edit calls
  `update_award_cell`, which already existed in `awards-sheets` for the terminal tool.
