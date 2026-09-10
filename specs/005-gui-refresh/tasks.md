---

description: "Task list template for feature implementation"
---

# Tasks: GUI Refresh

**Input**: Design documents from `/specs/005-gui-refresh/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/gui-refresh-interaction.md, quickstart.md (all present)

**Tests**: One new unit test, per plan.md's Testing section and the project's established convention.

**Organization**: A single user story (spec has only one) — no Setup or Foundational phase is needed since this feature adds no new crate, dependency, or workspace member; every task edits a file `004-gui-lookup-add` already created.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: `US1` (the only user story)

## Path Conventions

Two edits, no new files: `crates/awards-gui/src/app.rs` and `crates/awards-gui/src/ui.rs`.

---

## Phase 1: User Story 1 - Pull the latest data without restarting (Priority: P1)

**Goal**: A Refresh control that re-runs the existing startup sync on demand, never overlapping with itself, never discarding good data on failure.

**Independent Test**: With the app open and synced, add an award to a user through another means, click Refresh, look that user up, and confirm the new award appears.

- [X] T001 [US1] Add a guard clause to the top of `GuiApp::start_sync` in `crates/awards-gui/src/app.rs`: `if self.syncing { return; }`, so a second call while one sync is already running is a no-op regardless of call site (spec FR-004, research.md §2)
- [X] T002 [P] [US1] Unit test in `crates/awards-gui/src/app.rs` asserting a `start_sync()` call while `syncing == true` does not invoke the repaint callback a second time (reusing the existing `counting_repaint` test helper), proving no second background fetch is spawned
- [X] T003 [P] [US1] Unit test in `crates/awards-gui/src/app.rs` asserting a failed `SyncDone` (via `handle_sync_done`) leaves a previously-populated `data` untouched — i.e. sync `Ok` then sync `Err` still has `data.is_some()` afterward (spec FR-006)
- [X] T004 [US1] Add a Refresh button to the top panel in `crates/awards-gui/src/ui.rs`, next to the username field, enabled only while `!app.syncing`, calling `app.start_sync()` on click (contract: Refresh table)
- [X] T005 [US1] Confirm `cargo build -p awards-gui --locked` succeeds and `cargo clippy -p awards-gui --locked --all-targets -- -D warnings` is clean

**Checkpoint**: Refresh is fully functional — a clerk can pull latest data into an already-open window, recover from a failed startup sync, and cannot start two refreshes at once.

---

## Phase 2: Polish & Cross-Cutting Concerns

- [X] T006 Run `cargo test --workspace --locked` and `cargo clippy --workspace --all-targets --locked -- -D warnings`, confirming every `004-gui-lookup-add` test still passes unchanged alongside the two new tests
- [ ] T007 Manually execute `specs/005-gui-refresh/quickstart.md`'s four scenarios on a machine with a display — leave unchecked until run live (same class of limitation as `004`'s T036)

---

## Dependencies & Execution Order

T001 before T002/T003 (tests need the guard to exist to assert against, though T003 only exercises already-existing `handle_sync_done` behavior and could technically run first). T004 depends on T001 existing so the button has a safe function to call, though the button itself would compile without it. T005 depends on T001-T004. Polish (T006-T007) depends on everything above.

## Parallel Example

```bash
# T002 and T003 are independent test functions in the same file's test module:
Task: "Unit test: start_sync() is a no-op while syncing == true"
Task: "Unit test: a failed refresh leaves previously-synced data untouched"
```

## Implementation Strategy

Single story, no MVP-slicing needed — implement T001 through T005 in order, verify with T006, and T007 is a live-display follow-up the same as every prior feature's final manual-validation task.

## Notes

- No task in this list adds a dependency, a crate, a new write path, or duplicates business logic — there is none to add; this feature is two edits to existing, already-correct code.
