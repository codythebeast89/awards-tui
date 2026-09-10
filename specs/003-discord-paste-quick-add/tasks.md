# Tasks: Discord Paste Quick-Add

**Input**: Design documents from `/specs/003-discord-paste-quick-add/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/tui-paste-interaction.md, quickstart.md (all present)

**Tests**: Included. The project constitution's Testing Standards principle *requires* unit tests
for every new pure `awards-core` function and modal-transition tests for every new TUI state
change — these are not optional for this project, so test tasks are included throughout rather
than only on request.

**Organization**: Tasks are grouped by user story (spec.md) to enable independent implementation
and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files/functions, no dependency on an incomplete task)
- **[Story]**: Which user story this task belongs to (US1, US2)
- File paths are exact, relative to the repository root

## Path Conventions

Existing 3-crate workspace (`plan.md` Project Structure): `crates/awards-core/src/` (pure logic),
`crates/awards-tui/src/tui/` (TUI presentation). No new crate or directory.

---

## Phase 1: Setup

**Purpose**: Confirm a clean starting point. No new dependencies, crate, or project scaffolding is
needed (plan.md Technical Context) — the workspace already has everything this feature reuses.

- [X] T001 Run `cargo test --workspace --locked` and `cargo clippy --workspace --locked -- -D warnings` from the repo root and confirm both pass cleanly before making any change, establishing a known-good baseline to diff behavior against later (constitution Development Workflow & Quality Gates)

**Checkpoint**: Baseline confirmed clean — safe to start.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The shared parsing/matching logic and paste-ingestion plumbing both User Story 1 and
User Story 2 depend on (data-model.md, research.md §1–§5). No user-story-specific behavior lives
here — only the building blocks both stories need.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T002 [P] Add `ExtractedRequest { pub username: Option<String>, pub award_text: Option<String> }` struct to `crates/awards-core/src/types.rs` per data-model.md
- [X] T003 [P] Implement `extract_paste_fields(raw: &str) -> ExtractedRequest` in `crates/awards-core/src/parse.rs`: case-insensitive line scan recognizing username-label variants "ROBLOX Username" and bare "Username" (FR-002), and award-label variants "Badge Requested", "Ribbon Requested", and "Award Requested" (FR-003); strip Markdown `**`/`_` emphasis and a leading `@` mention sigil from each matched value before returning it (spec Edge Case: Discord copy-paste artifacts); when more than one line matches the same field's label, keep only the *first* match (spec Edge Case: stacked/quoted requests); validate the username candidate through the existing `parse_bare_username`, treating an invalid value the same as "not found" (data-model.md `ExtractedRequest`)
- [X] T004 [P] Add unit tests for `extract_paste_fields` in `crates/awards-core/src/parse.rs` (or its test module) covering: the real `torba_f` badge sample and the real `Nevazaku_u` ribbon sample from `spec.md`/`quickstart.md`; a bare `"Username"` label combined with `**bold**`/`_italic_`/`@mention` artifacts (quickstart.md Scenario 6); a paste missing the username line; a paste missing the award line; two stacked `"ROBLOX Username:"` lines (confirm the first wins, not the second); and text with no recognizable labeled lines at all (`ExtractedRequest { username: None, award_text: None }`)
- [X] T005 [P] Implement `split_award_suffix(text: &str) -> (String, String)` in `crates/awards-core/src/parse.rs` per data-model.md: strips a trailing case-insensitive `x<digits>` token, returning `(base_name_query, suffix)` with `suffix == ""` when no such token is present
- [X] T006 [P] Add unit tests for `split_award_suffix` in `crates/awards-core/src/parse.rs` covering `"Afghanistan Campaign x1"` → `("Afghanistan Campaign", "x1")`, plain text with no suffix, and an award name that merely ends in the letter `x` not followed by digits (must NOT be treated as a suffix)
- [X] T007 [P] Implement `match_catalog_entries(catalog: &[AwardDef], query: &str) -> Vec<AwardDef>` in `crates/awards-core/src/parse.rs` per data-model.md: returns every entry whose `base_name` or category label contains `query` case-insensitively — the same predicate `AddModal::reload` (`crates/awards-tui/src/tui/app.rs`) currently inlines; an empty `query` matches everything
- [X] T008 [P] Add unit tests for `match_catalog_entries` in `crates/awards-core/src/parse.rs` covering: a query matching exactly one catalog entry, a query matching zero entries, and a query matching more than one entry
- [X] T009 Export `ExtractedRequest`, `extract_paste_fields`, `split_award_suffix`, and `match_catalog_entries` from `crates/awards-core/src/lib.rs`'s `pub use parse::{...}` / `pub use types::{...}` lists (depends on T002, T003, T005, T007)
- [X] T010 Refactor `AddModal::reload`'s inline filter closure in `crates/awards-tui/src/tui/app.rs` to call the newly-exported `match_catalog_entries` instead of duplicating the same substring predicate (research.md §5) — behavior must stay identical to today (depends on T007, T009)
- [X] T011 [P] Enable `crossterm`'s bracketed-paste mode (`EnableBracketedPaste` on setup, `DisableBracketedPaste` on restore) in `crates/awards-tui/src/tui/mod.rs`'s `TerminalSession::new`/`restore`, alongside the existing `enable_raw_mode`/`EnterAlternateScreen` setup (research.md §1)
- [X] T012 [P] Add `Action::PasteAdd` variant to the `Action` enum in `crates/awards-tui/src/tui/app.rs`, with `label()` returning `"Paste"`
- [X] T013 Add `Modal::PasteAdd(PasteAddModal)` variant to the `Modal` enum and the `PasteAddModal { pub buffer: String, pub error: Option<String> }` struct to `crates/awards-tui/src/tui/app.rs` per data-model.md
- [X] T014 Handle the new `Event::Paste(String)` variant in `run_app`'s event-read match in `crates/awards-tui/src/tui/mod.rs` (currently only `Event::Key` is matched): when `Modal::PasteAdd` is open, append the pasted string to `PasteAddModal.buffer`; route other key events (`Backspace`, character keys) to the same buffer as a fallback for terminals without bracketed-paste support (research.md §1) (depends on T011, T013)
- [X] T015 Wire `Action::PasteAdd` into the Actions pane list and a direct hotkey `p` (unused today) in `crates/awards-tui/src/tui/app.rs`'s action dispatch, mirroring the existing `a`/`e`/`d`/`n`/`c` pattern (research.md §2): opens `Modal::PasteAdd` with an empty buffer, with **no** `results_username` precondition, unlike `action_add` (depends on T012, T013)

**Checkpoint**: Foundation ready — pure parsing/matching logic and paste ingestion both exist and are tested; user story implementation can now begin.

---

## Phase 3: User Story 1 - Paste a request straight into the Add flow (Priority: P1) 🎯 MVP

**Goal**: A clerk pastes a correctly-formatted request naming a known award and lands directly on
the existing Add flow's confirmation step, with username and award already identified, writing
through the unchanged OAuth-gated path (spec User Story 1, FR-001, FR-004, FR-009, FR-011).

**Independent Test**: quickstart.md Scenarios 1 and 2 — paste the real `torba_f` badge sample and
the real `Nevazaku_u` ribbon-with-`x1`-suffix sample; confirm each reaches `AddStep::Suffix` with
the correct award pre-selected (and, for the ribbon sample, `suffix == "x1"`), and that confirming
writes the same as a manual Add.

### Implementation for User Story 1

- [X] T016 [US1] Implement paste-submit handling for `Modal::PasteAdd` in `crates/awards-tui/src/tui/app.rs`: on `Enter`, call `extract_paste_fields(&buffer)`; when a valid username is extracted, call the existing `apply_user_view(username, None, None)` (local, in-memory — no network call, research.md §7) (depends on T009, T013, T014)
- [X] T017 [US1] Implement the confident-match path in `crates/awards-tui/src/tui/app.rs`: split the extracted award text via `split_award_suffix`, match the base text against `data.catalog` via `match_catalog_entries`; when exactly one match is found, close `Modal::PasteAdd` and open `Modal::Add(AddModal { chosen: Some(matched), step: AddStep::Suffix, suffix: input_with_value(<extracted suffix>), all_candidates: candidates.clone(), filtered: candidates, .. })` per `contracts/tui-paste-interaction.md` "Submit behavior" §2 — mirror `action_add`'s existing "No remaining awards to add for this user" status when `candidates` is empty (depends on T016)
- [X] T018 [US1] Add TUI transition tests in `crates/awards-tui/src/tui/app.rs` asserting: pasting the real `torba_f` badge sample (spec.md) lands in `Modal::Add` with `step == AddStep::Suffix`, `chosen` matching "Army Parachutist Badge", and an empty suffix; pasting the real `Nevazaku_u` ribbon sample lands with `chosen` matching the "Afghanistan Campaign" catalog entry and `suffix == "x1"` (FR-011, spec User Story 1 Acceptance Scenario 3) (depends on T017)
- [X] T019 [US1] Add a TUI transition test in `crates/awards-tui/src/tui/app.rs` confirming that confirming the paste-prefilled `AddModal` (pressing `Enter` from `AddStep::Suffix`) reaches the exact same `add_confirm` dispatch — and therefore the same `add_award_to_user` write call — a manually-entered Add reaches, with no new confirmation rule (FR-009, spec User Story 1 Acceptance Scenario 2) (depends on T017)
- [X] T020 [P] [US1] Update `README.md`'s TUI keys table (and the "Notes" section if relevant) to document the new `p` / "Paste" action, per the constitution's UX Consistency principle keeping documentation in sync with new keybindings (depends on T015)

**Checkpoint**: User Story 1 is fully functional and independently testable — a clerk can paste either real sample and reach a working, pre-filled, confirmable Add.

---

## Phase 4: User Story 2 - Handle a request that doesn't parse cleanly (Priority: P2)

**Goal**: A clerk's imperfect paste (unmatched award, missing username, or fully unparseable text)
fails safe — landing on the existing searchable picker or a clear, correctable error — never a
dead end and never a silently-wrong auto-selection (spec User Story 2, FR-005, FR-006, FR-007).

**Independent Test**: quickstart.md Scenarios 3, 4, and 5 — an award name with no catalog match, a
paste with no recognizable username line, and a paste with no recognizable fields at all.

### Implementation for User Story 2

- [X] T021 [US2] Implement the fallback-match path in `crates/awards-tui/src/tui/app.rs`: when `match_catalog_entries` returns zero or more than one match (or no award text was extracted), close `Modal::PasteAdd` and open the normal `Modal::Add(AddModal::new(candidates))` (`AddStep::Pick`) with `filter` pre-filled to the extracted award text (empty if none), per `contracts/tui-paste-interaction.md` "Submit behavior" §2 (spec User Story 2 Acceptance Scenario 1) (depends on T017)
- [X] T022 [US2] Implement the missing/invalid-username fallback in `crates/awards-tui/src/tui/app.rs`: when `extract_paste_fields` yields no valid username, keep `Modal::PasteAdd` open with `buffer` unchanged and set `PasteAddModal.error` to a clerk-facing message (e.g. "Couldn't find a username in that text — check it and try again, or Esc to cancel"), so the clerk can correct and resubmit or back out to manual Lookup (spec User Story 2 Acceptance Scenario 2); a paste matching no recognized labels at all uses the same path with a generic message (spec User Story 2 Acceptance Scenario 3 / Edge Case) (depends on T016)
- [X] T023 [P] [US2] Render `PasteAddModal` in `crates/awards-tui/src/tui/ui.rs`: show the accumulated `buffer` (or a placeholder hint when empty, e.g. "Paste the Discord message, then press Enter"), and the inline `error` message below it when set (depends on T013)
- [X] T024 [US2] Add TUI transition tests in `crates/awards-tui/src/tui/app.rs` covering: an award name with zero catalog matches opens `Modal::Add` at `AddStep::Pick` with `filter` pre-filled to the extracted text; an award name with multiple catalog matches falls through the same way; a paste with no username line leaves `Modal::PasteAdd` open with `error` set and `buffer` unchanged; a paste with no recognizable fields at all gets the same generic-error treatment (spec User Story 2, all three Acceptance Scenarios) (depends on T021, T022)
- [X] T025 [US2] Add a TUI transition test in `crates/awards-tui/src/tui/app.rs` confirming `Esc` from `Modal::PasteAdd` closes the modal and sets status to `"Dialog cancelled"` — the existing default convention requires no new constitution exception (plan.md Constitution Check, Principle III) (depends on T013)

**Checkpoint**: User Stories 1 AND 2 both work independently — every acceptance scenario in `spec.md` has a corresponding passing test.

---

## Phase 5: Polish & Cross-Cutting Concerns

**Purpose**: Final verification against the constitution's mandatory CI gate and the feature's own quickstart validation guide.

- [X] T026 [P] Run `cargo clippy --workspace --locked -- -D warnings` and fix any warnings introduced by this feature's new code (constitution Code Quality / Development Workflow principles)
- [X] T027 [P] Run `cargo test --workspace --locked` and confirm the full suite (pre-existing plus every test added in Phases 2–4) passes, including that the T010 refactor of `AddModal::reload` produced no behavior change (quickstart.md "Regression check — existing Add flow untouched")
- [ ] T028 Manually execute every scenario in `specs/003-discord-paste-quick-add/quickstart.md` (Scenarios 1–6 plus the regression check) against a running `awards-tui`, and specifically confirm no outbound network request occurs for the Google Sheets proof link pasted in Scenario 2 (FR-008)
- [X] T029 [P] Review `crates/awards-core/src/lib.rs`'s module-level doc comments and `README.md`'s "Notes" section for accuracy now that `extract_paste_fields`/`split_award_suffix`/`match_catalog_entries` exist, updating anything now stale

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately.
- **Foundational (Phase 2)**: Depends on Setup completion — BLOCKS both user stories.
- **User Story 1 (Phase 3)**: Depends on Foundational completion. No dependency on User Story 2.
- **User Story 2 (Phase 4)**: Depends on Foundational completion, and specifically on the confident-match path (T017) built in User Story 1, since its fallback logic is the "otherwise" branch of the same submit handler. Independently *testable* per its own acceptance scenarios once T017 exists.
- **Polish (Phase 5)**: Depends on Phases 3 and 4 both being complete.

### Within Each Phase

- Foundational: the three pure-function groups (T002–T004, T005–T006, T007–T008) are independent of each other and can run in parallel; T009 (export) and T010 (refactor) depend on all three landing first. The paste-ingestion group (T011–T015) is independent of the parsing group and can proceed in parallel with it.
- User Story 1: T016 → T017 → {T018, T019} in sequence (each depends on the previous); T020 (docs) only needs T015 and can run any time after Foundational.
- User Story 2: T021 and T022 both depend on T016/T017 from User Story 1 but not on each other — can run in parallel; T023 (rendering) only needs T013; T024 depends on T021+T022; T025 only needs T013.

### Parallel Opportunities

- All Foundational pure-function tasks marked `[P]` (T002–T008, T011, T012) can run in parallel — different functions/files, no shared state.
- T020, T023, T026, T027, T029 are marked `[P]` and can run alongside their phase's other work once their own single dependency is satisfied.

---

## Parallel Example: Foundational Phase

```bash
# Launch the three independent pure-function groups together:
Task: "Implement extract_paste_fields in crates/awards-core/src/parse.rs + its unit tests (T003, T004)"
Task: "Implement split_award_suffix in crates/awards-core/src/parse.rs + its unit tests (T005, T006)"
Task: "Implement match_catalog_entries in crates/awards-core/src/parse.rs + its unit tests (T007, T008)"

# In parallel with the above, the paste-ingestion plumbing:
Task: "Enable crossterm bracketed-paste mode in crates/awards-tui/src/tui/mod.rs (T011)"
Task: "Add Action::PasteAdd variant in crates/awards-tui/src/tui/app.rs (T012)"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup.
2. Complete Phase 2: Foundational (CRITICAL — blocks both stories).
3. Complete Phase 3: User Story 1.
4. **STOP and VALIDATE**: run quickstart.md Scenarios 1 and 2 against a real build.
5. This alone delivers the feature's entire stated value (spec Why-this-priority for User Story 1) — clerks with clean pastes are fully served.

### Incremental Delivery

1. Setup + Foundational → foundation ready.
2. User Story 1 → validate independently → this is the MVP.
3. User Story 2 → validate independently → adds graceful handling for imperfect pastes, without changing User Story 1's behavior.
4. Polish → final CI gate + full quickstart run-through.

## Notes

- `[P]` tasks touch different functions/files with no shared dependency chain.
- `[US1]`/`[US2]` labels map each task to its spec.md user story for traceability.
- Every new pure `awards-core` function ships with its unit tests in the same task group, per the constitution's Testing Standards principle (not deferred to Polish).
- Commit after each task or logical group; verify `cargo test --workspace --locked` stays green throughout, not just at the end.
- Constitution Check in `plan.md` reported no violations — no Complexity Tracking entries to work off of here.
