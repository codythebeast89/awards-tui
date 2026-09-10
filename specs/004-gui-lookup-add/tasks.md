---

description: "Task list template for feature implementation"
---

# Tasks: GUI Lookup & Add

**Input**: Design documents from `/specs/004-gui-lookup-add/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/gui-lookup-add-interaction.md, quickstart.md (all present)

**Tests**: Not explicitly requested in the spec, but the constitution's Testing Standards principle (and this repo's established convention across features 002/003) require new modal/state transitions to be unit tested, so test tasks are included alongside their implementation tasks rather than as a separate up-front phase.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2)
- Every task includes an exact file path

## Path Conventions

Single Cargo workspace, new binary crate: `crates/awards-gui/{Cargo.toml, src/{main.rs, app.rs, ui.rs}}`, added as a fourth workspace member alongside `awards-core`, `awards-sheets`, `awards-tui` (plan.md Project Structure).

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Stand up the new crate so it compiles before any state or UI logic exists.

- [X] T001 Add `"crates/awards-gui"` to the `[workspace] members` list in the repository root `Cargo.toml`
- [X] T002 [P] Create `crates/awards-gui/Cargo.toml` declaring `eframe = "0.36"` (this crate's own dependency only, per Constitution I — no other crate needs it), plus `awards-core`, `awards-sheets` (existing workspace crates) and `anyhow` (existing workspace dependency), matching `awards-tui`'s own binary-crate `Cargo.toml` shape
- [X] T003 [P] Create stub `crates/awards-gui/src/main.rs`, `crates/awards-gui/src/app.rs`, and `crates/awards-gui/src/ui.rs` (empty modules wired together) so `cargo check -p awards-gui` succeeds

**Checkpoint**: `awards-gui` exists as an empty, compiling workspace member.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The shared `GuiApp` state machine, message channel, and auth/sync plumbing every user story depends on.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T004 Define `AuthState` enum (`Unknown`, `SignedOut`, `SignedIn`, `SigningIn`) in `crates/awards-gui/src/app.rs` per data-model.md
- [X] T005 Define `GuiMsg` enum (`SyncDone(Result<AwardsData, String>)`, `LoginDone(Result<String, String>)`, `AddDone { username: String, result: EditResult }`) in `crates/awards-gui/src/app.rs` per data-model.md — `EditResult` is `awards_sheets::edit::EditResult`, reused unchanged
- [X] T006 Define the `GuiApp` struct (`tx`, `rx`, `ctx`, `data: Option<AwardsData>`, `syncing: bool`, `sync_error: Option<String>`, `username_input: String`, `looked_up: Option<LookedUpUser>`, `auth: AuthState`, `add_picker: Option<AddPicker>`, `status: String`) in `crates/awards-gui/src/app.rs` per data-model.md
- [X] T007 Implement `GuiApp::refresh_auth_state(&mut self)`, mapping `awards_sheets::auth_status()`'s four string values (`"service_account"` / `"oauth_token"` → `SignedIn`; `"oauth_needs_login"` / `"missing"` → `SignedOut`) into `AuthState` — the single call site that interprets the raw string (research.md §4, data-model.md)
- [X] T008 Implement `GuiApp::new(ctx: egui::Context) -> Self` in `crates/awards-gui/src/app.rs`: create the `mpsc` channel, store the cloned context, call `refresh_auth_state()` for the initial value, set `syncing = true` and `status = "Syncing..."`, and spawn a background thread calling `awards_sheets::build_awards_data(None)` that sends `GuiMsg::SyncDone` and calls `ctx.request_repaint()` (research.md §3, §6)
- [X] T009 Implement `GuiApp::handle_msg(&mut self, msg: GuiMsg)` in `crates/awards-gui/src/app.rs` as the single dispatch point (called once per drained message at the top of `eframe::App::update`); wire the `SyncDone(Ok)` / `SyncDone(Err)` arms now per data-model.md's State Transitions (`data`/`syncing`/`sync_error` updates); leave the `LoginDone`/`AddDone` arms as placeholders to be filled in by Phase 4
- [X] T010 [P] Unit tests in `crates/awards-gui/src/app.rs` for `GuiApp::new` (auth set from the initial `auth_status()` mapping; `syncing` starts `true`) and for `handle_msg`'s `SyncDone` arm (`Ok` sets `data`/clears `syncing`; `Err` sets `sync_error`/clears `syncing`), mirroring `awards-tui/src/tui/app.rs`'s existing headless-testable pattern (plan.md Testing section)

**Checkpoint**: Foundation ready — `GuiApp` compiles, syncs on startup, and tracks auth state. User story implementation can now begin.

---

## Phase 3: User Story 1 - Look up a user in a graphical window (Priority: P1) 🎯 MVP

**Goal**: A clerk can type a Roblox username and see that user's current awards grouped by category, or a clear "not found" message — entirely read-only.

**Independent Test**: Launch the application, enter a known username, and confirm the displayed awards match what the terminal tool's Lookup shows for the same username, with no write action involved.

### Implementation for User Story 1

- [X] T011 [US1] Define `LookedUpUser { username: String, awards: Vec<Award>, not_found: bool }` in `crates/awards-gui/src/app.rs` per data-model.md (`Award` is `awards_core::Award`, reused unchanged)
- [X] T012 [US1] Implement `GuiApp::submit_lookup(&mut self)`: read `username_input`, look the user up purely locally via the existing `AwardsData` index (`get_awards_for_username`, research.md §6 — no network call, mirroring `awards-tui`'s `apply_user_view`), and set `looked_up` to `Some(LookedUpUser { not_found: false, .. })` on a hit or `Some(LookedUpUser { awards: vec![], not_found: true, .. })` on a miss (spec FR-008)
- [X] T013 [US1] Guard `submit_lookup()` (or its call site) so lookup is disabled while `syncing == true`, since the index isn't queryable until the initial sync completes (spec Edge Case: no freeze/conflicting-state while a network operation is in flight)
- [X] T014 [P] [US1] Unit tests in `crates/awards-gui/src/app.rs` for `submit_lookup`: a known username yields `not_found == false` with the matching awards; an unknown username yields `not_found == true` with `awards == []`; calling it before `data` is populated does not panic
- [X] T015 [US1] Implement the top panel in `crates/awards-gui/src/ui.rs`: a username text field bound to `GuiApp.username_input`, a Look Up control (button and Enter-to-submit) calling `submit_lookup()`, and a "Syncing..." indicator shown while `syncing == true` with lookup disabled (contract: Lookup table)
- [X] T016 [US1] Implement the results panel in `crates/awards-gui/src/ui.rs`: when `looked_up` is `Some` and `not_found == false`, render the awards grouped by category (Badges / Ribbons / Foreign Awards) matching the terminal tool's own grouping; when `not_found == true`, render a plain "no records found" message instead of an empty list (spec Acceptance Scenario 1.2, FR-008)
- [X] T017 [US1] Implement `crates/awards-gui/src/main.rs`'s `eframe::run_native` entry point: construct `GuiApp::new(cc.egui_ctx.clone())` and implement `eframe::App::update` to drain the `mpsc` receiver via `handle_msg`, then render through `ui.rs` (plan.md Project Structure)
- [X] T018 [US1] Confirm `cargo build -p awards-gui --locked` succeeds and `cargo clippy -p awards-gui --locked -- -D warnings` is clean

**Checkpoint**: User Story 1 is fully functional and independently testable — a clerk can look up any username and see accurate results or a clear not-found message, with no write capability yet.

---

## Phase 4: User Story 2 - Add an award to a looked-up user (Priority: P2)

**Goal**: From a looked-up user, the clerk picks an award from a searchable catalog and confirms, writing it through the same guarded path the terminal tool's Add already uses.

**Independent Test**: With a user already looked up, select an award they don't currently hold, confirm the add, and verify it now appears both in the application's own view and in the underlying spreadsheet.

### Implementation for User Story 2

- [X] T019 [US2] Define `AddPicker { candidates: Vec<AwardDef>, filter: String, filtered: Vec<AwardDef>, selected: Option<AwardDef>, suffix: String, submitting: bool }` in `crates/awards-gui/src/app.rs` per data-model.md (`AwardDef` is `awards_core::AwardDef`, reused unchanged)
- [X] T020 [US2] Implement `GuiApp::open_add_picker(&mut self)`: compute `candidates` once as `data.catalog` minus this user's already-owned awards (`owned_award_columns`, research.md §6); if `candidates` is empty, set `status = "No remaining awards to add for this user"` and do not open the picker (contract: Add table); otherwise set `add_picker = Some(AddPicker { .. })`
- [X] T021 [P] [US2] Unit test in `crates/awards-gui/src/app.rs` for `open_add_picker`: a user missing some awards opens the picker with the correct candidate set; a user already holding every award leaves the picker closed and sets `status` (spec Edge Case: "user already holds every available award")
- [X] T022 [US2] Implement `GuiApp::update_filter(&mut self, text: String)`: set `add_picker.filter = text` and recompute `add_picker.filtered` via `awards_core::parse::match_catalog_entries(&candidates, &filter)` (research.md §6 — identical behavior to the TUI's own Add picker filter)
- [X] T023 [P] [US2] Unit test for `update_filter`: a substring of a known award's `base_name`/category label narrows `filtered` to the matching subset, mirroring `match_catalog_entries`'s existing case-insensitive substring behavior
- [X] T024 [US2] Implement `GuiApp::select(&mut self, def: AwardDef)` setting `add_picker.selected = Some(def)` only — no write is triggered, keeping selecting and confirming separate steps (spec FR-004)
- [X] T025 [US2] Implement `GuiApp::confirm_add(&mut self)`: guarded on `add_picker.selected.is_some() && auth == AuthState::SignedIn`; sets `add_picker.submitting = true` and spawns a background thread calling `awards_sheets::edit::add_award_to_user(username, &selected, &suffix, /* interactive_auth */ false)` verbatim (research.md §5, matching the TUI's own call site), sending `GuiMsg::AddDone { username, result }` and calling `ctx.request_repaint()` when done
- [X] T026 [US2] Extend `GuiApp::handle_msg`'s `AddDone` arm in `crates/awards-gui/src/app.rs`: when `result.ok`, push `result.award` (unwrap) onto `looked_up.awards`, close `add_picker` (`None`), and set `status = result.message` (display updates immediately, no re-sync); when `!result.ok` (e.g. `EditError::Stale`, `EditError::Conflict`), set `add_picker.submitting = false` and `status = result.message` while leaving the picker open with the clerk's selection intact (spec Acceptance Scenario 2.4, data-model.md State Transitions)
- [X] T027 [P] [US2] Unit tests in `crates/awards-gui/src/app.rs` for `handle_msg`'s `AddDone` arm: an `Ok` result appends the award and closes the picker; a `Stale`/`Conflict` result keeps the picker open, clears `submitting`, and preserves the existing selection
- [X] T028 [US2] Implement the Add control in `crates/awards-gui/src/ui.rs`: an "Add Award" button enabled only when `looked_up` is `Some` and not empty-catalog, calling `open_add_picker()`; when disabled by an empty candidate set, show the "No remaining awards to add for this user" status text instead (contract: Add table)
- [X] T029 [US2] Implement the inline award picker in `crates/awards-gui/src/ui.rs` (not a separate window, contract: Window layout): a search box bound to `add_picker.filter` calling `update_filter()` on change, a list of `add_picker.filtered` awards each selectable via `select()`, an optional suffix text field bound to `add_picker.suffix`, a Confirm button enabled only when `selected.is_some() && auth == SignedIn` calling `confirm_add()` (disabled with a spinner while `submitting`), and a Cancel control that sets `add_picker = None` with no side effect (contract: Add table)
- [X] T030 [US2] Implement the Sign In control in `crates/awards-gui/src/ui.rs`: visible only when `auth != AuthState::SignedIn` (hidden entirely when `SignedIn`, contract: Sign-in table); clicking it sets `auth = AuthState::SigningIn` and spawns a background thread calling `awards_sheets::auth::login()` verbatim, sending `GuiMsg::LoginDone(..)` and calling `ctx.request_repaint()` (research.md §4)
- [X] T031 [US2] Extend `GuiApp::handle_msg`'s `LoginDone` arm: on `Ok(msg)`, set `status = msg` and call `refresh_auth_state()` (becomes `SignedIn` on genuine success); on `Err(msg)`, set `status = msg` and call `refresh_auth_state()` (stays `SignedOut` so the clerk can retry) (contract: Sign-in table)
- [X] T032 [P] [US2] Unit tests in `crates/awards-gui/src/app.rs` for `handle_msg`'s `LoginDone` arm (`Ok` → `SignedIn` path, `Err` → stays `SignedOut`) and for `confirm_add`'s guard (no thread spawned / no-op when `selected` is `None` or `auth != SignedIn`)
- [X] T033 [US2] Wire the Add button's and Sign In control's disabled/hidden states in `ui.rs` to spec FR-007 ("clearly indicate ... cannot perform a write, without blocking the read-only lookup capability") — lookup must stay usable regardless of auth state

**Checkpoint**: User Stories 1 AND 2 both work independently — a clerk can look up a user, sign in if needed, and add an award end-to-end, matching the terminal tool's guarantees exactly.

---

## Phase 5: Polish & Cross-Cutting Concerns

- [X] T034 [P] Run `cargo fmt` across `crates/awards-gui`, matching the workspace's existing formatting convention
- [X] T035 [P] Run `cargo test --workspace --locked` and `cargo clippy --workspace --locked --all-targets -- -D warnings`, confirming all pre-existing `awards-core`/`awards-sheets`/`awards-tui` tests remain unchanged alongside the new `awards-gui` unit tests (plan.md Testing section, Constitution II)
- [ ] T036 Manually execute `specs/004-gui-lookup-add/quickstart.md`'s six scenarios on a machine with a display (out of reach in this headless build/test environment — the same class of limitation as feature 003's T028) — leave unchecked until run live
- [X] T037 [P] Update the repository README's entry-point documentation (if present) to mention the new `cargo run -p awards-gui` entry point alongside the existing CLI/TUI ones

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately.
- **Foundational (Phase 2)**: Depends on Setup — BLOCKS both user stories.
- **User Story 1 (Phase 3)**: Depends on Foundational only. Delivers the MVP on its own.
- **User Story 2 (Phase 4)**: Depends on Foundational, and functionally depends on User Story 1 being in view before an add can start (spec: "It depends on User Story 1 ... but is independently verifiable once that exists") — implement after Phase 3.
- **Polish (Phase 5)**: Depends on both user stories being complete.

### Within Each User Story

- State/logic in `app.rs` before rendering in `ui.rs` (the render code reads and drives `GuiApp` state, so it needs the state shape to exist first).
- Unit tests for a given piece of state logic can be written alongside (marked `[P]` where they touch only `app.rs`'s test module and no other in-flight task's code).

### Parallel Opportunities

- T002 and T003 (Phase 1) touch different files and can run in parallel.
- T010, T014, T021, T023, T027, T032 (state-logic unit tests) are each `[P]` — independent test functions in `app.rs`'s test module, safe to write alongside their sibling non-test tasks once the function under test exists.
- T034 and T035 and T037 (Phase 5) touch independent concerns and can run in parallel.

---

## Parallel Example: Phase 1

```bash
# Launch Phase 1's independent setup tasks together:
Task: "Create crates/awards-gui/Cargo.toml with eframe, awards-core, awards-sheets, anyhow"
Task: "Create stub crates/awards-gui/src/{main.rs,app.rs,ui.rs}"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup.
2. Complete Phase 2: Foundational (blocks both stories).
3. Complete Phase 3: User Story 1 — Lookup.
4. **STOP and VALIDATE**: run Scenario 1/2 of quickstart.md once a display is available; confirm parity with `awards-tui <username>`.
5. This alone is a shippable, presentation-only read-only GUI.

### Incremental Delivery

1. Setup + Foundational → foundation ready.
2. User Story 1 → validate independently → MVP.
3. User Story 2 → validate independently (Scenarios 3-6 of quickstart.md) → full first-milestone scope (spec FR-001 through FR-011 satisfied).
4. Polish → `cargo fmt`/`clippy`/`test --workspace` clean, quickstart run live, docs updated.

## Notes

- `[P]` tasks touch different files (or independent test functions within `app.rs`'s test module) with no dependency on an incomplete task.
- `[Story]` labels (`US1`, `US2`) map each task to its user story for traceability; Setup, Foundational, and Polish tasks carry no story label by convention.
- No task in this list introduces a new write path, duplicates parsing/matching/eligibility logic, or adds a dependency outside `crates/awards-gui/Cargo.toml` — enforced by Constitution I and spec FR-005/FR-006/FR-009.
- Commit after each task or logical group; stop at either checkpoint to validate a story independently before continuing.

---

## Phase 6: Convergence

Appended by `/speckit-converge` after Phase 3 (User Story 1) shipped and the app was confirmed running live. Two narrow gaps found against spec.md/plan.md/tasks.md; no constitution violations.

- [X] T038 Update `GuiApp::handle_add_done`'s success branch to call `awards_core::upsert_award_in_index(&mut data.index, &award)` (in addition to the existing `looked_up.awards.push`), so a subsequent lookup of the same user reflects a just-added award without waiting for the next full sync, in `crates/awards-gui/src/app.rs` per FR-002, US1 Independent Test (partial)
- [X] T039 Fix `GuiApp::submit_lookup`'s no-data guard in `crates/awards-gui/src/app.rs` to say the sync failed (using `sync_error`) rather than "Still syncing" when `syncing == false` and `data` is still `None`, per FR-002 (partial)
