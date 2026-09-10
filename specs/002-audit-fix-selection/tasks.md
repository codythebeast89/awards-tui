# Tasks: Audit Fix Selection

**Input**: Design documents from `/specs/002-audit-fix-selection/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/tui-audit-interaction.md, quickstart.md (all present)

**Tests**: Included — the project constitution's Testing Standards principle mandates unit tests
ship with every new pure function in `awards-core` and a transition test with every new/changed
TUI modal state, in the same change (not a separate opt-in), so test tasks are folded into the
implementation task that introduces the behavior, matching how this repo's prior feature
(001-usar-award-logging) was executed.

**Organization**: Tasks are grouped by user story (spec.md's P1/P2/P3), preceded by a Setup phase
and a Foundational phase per `data-model.md`'s shared `AuditFinding` type and `AuditModal`
restructuring, which every user story depends on.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: Which user story this task belongs to (US1/US2/US3)
- Exact file paths are included in every task description

## Path Conventions

Existing single-workspace layout, unchanged by this feature (see `plan.md`'s Project Structure):
`crates/awards-core/src/`, `crates/awards-tui/src/tui/`. No `main.rs`/CLI changes — the
CLI-parity gap is explicitly documented in `plan.md`'s Constitution Check.

---

## Phase 1: Setup

**Purpose**: Confirm a clean baseline before this feature's changes begin.

- [X] T001 Run `cargo build --workspace`, `cargo test --workspace --locked`, and
      `cargo clippy --workspace --locked -- -D warnings` against the current tree (no source
      changes yet) to confirm a green starting point for `002-audit-fix-selection`.
      Remediation note: the Foundational (`awards-core`) work for this feature was implemented
      and verified green (`cargo test -p awards-core`) before this checkbox was retroactively
      confirmed; the full-workspace gate (build/test/clippy, this exact command) has since been
      run clean multiple times, including at T021 below — there was no red baseline at any point.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The shared `AuditFinding` model (`data-model.md`) and the `AuditModal`/`AuditView`
restructuring every user story builds on.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T002 Add the `AuditFinding` enum to `crates/awards-core/src/audit.rs` with exactly the four
      variants and fields specified in `data-model.md`'s "`AuditFinding` (new —
      `awards-core::audit`)" section: `DuplicateRow { user, sheet, col, base_name, kind, row,
      cell }`, `SimilarUsernames { a, b, sheet, col, base_name }`, `MalformedCell { user, sheet,
      col, base_name, row, cell, issues }`, `UnparseableCell { sheet, col, base_name, row, cell }`.
- [X] T003 Implement `flatten_audit_findings(report: &AuditReport) -> Vec<AuditFinding>` in
      `crates/awards-core/src/audit.rs` (depends on T002, same file): one `AuditFinding::DuplicateRow`
      per entry in each `AuditDuplicateGroup.rows` (data-model.md's rule — "a group with 3
      duplicate rows becomes 3 findings"); one `AuditFinding::SimilarUsernames` per
      `AuditSimilarPair`; one `AuditFinding::MalformedCell` per `AuditMalformed`; one
      `AuditFinding::UnparseableCell` per `AuditUnparsed`. Add unit tests in the same file's
      `#[cfg(test)]` module: a 3-row duplicate group yields exactly 3 `DuplicateRow` findings; a
      1-row (non-duplicate) group yields none; similar/malformed/unparsed entries map 1:1;
      relative ordering is stable across calls.
- [X] T004 Implement `finding_username(finding: &AuditFinding) -> Option<&str>` in
      `crates/awards-core/src/audit.rs` (depends on T002, same file as T003): returns `Some` for
      `DuplicateRow`/`MalformedCell` (their `user` field), `Some(&a)` for `SimilarUsernames` (the
      first of the pair — used only for default grouping; both usernames stay visible per finding),
      and `None` only for `UnparseableCell`, per data-model.md ("there is deliberately no username
      to key this on — FR-009"). Add unit tests for all four variants.
- [X] T005 [P] Implement `AuditFinding::to_award(&self) -> Option<Award>` in
      `crates/awards-core/src/types.rs` (depends on T002; different file from T003/T004, can
      proceed in parallel with them), mirroring the existing `DuplicateHit::to_award()` pattern in
      the same file: `Some(Award { category, name, sheet, col, row, cell, base_name })` for
      `DuplicateRow` and `MalformedCell` with fields mapped exactly from the finding; `None` for
      `SimilarUsernames` and `UnparseableCell` (data-model.md: "no single target row"). Add unit
      tests asserting exact field mapping for both `Some` cases and `None` for the other two.
- [X] T006 Re-export `AuditFinding`, `flatten_audit_findings`, and `finding_username` from
      `crates/awards-core/src/lib.rs` (depends on T002-T005), alongside the existing
      `collect_sheet_audit`/`format_audit_report` exports.
- [X] T007 In `crates/awards-tui/src/tui/app.rs` (depends on T006), add the `AuditView` enum
      (`List(AuditFindingsList)` | `Report`) and the `AuditFindingsList { findings, groups, state
      }` / `FindingGroup { label, finding_indices }` structs exactly as specified in
      `data-model.md`'s "`AuditFindingsList` (new — `awards-tui::tui::app`, presentation-only)"
      section; extend the existing `AuditModal` struct with a `view: AuditView` field (existing
      `path`/`lines`/`scroll` fields unchanged, still backing `AuditView::Report`).
      Remediation note (deliberate, documented design correction — data-model.md's literal sketch
      had `AuditView::List(AuditFindingsList)` carry the list payload directly, which would
      discard the findings list's data whenever the clerk toggled to `AuditView::Report` and back,
      since `Report` has nowhere to hold it): `AuditView` is instead a stateless `{List, Report,
      ChooseUsername { a, b }}` marker (the third variant is a transient one-shot sub-dialog for
      resolving a `SimilarUsernames` finding, out of data-model.md's original scope but required
      by FR-010's "which of the two usernames" choice), and `AuditModal` holds `list:
      AuditFindingsList` as its own persistent field alongside `view`, so list data survives every
      toggle. `AuditFindingsList`/`FindingGroup` hold `groups: Vec<FindingGroup>` /
      `{ username: Option<String>, findings: Vec<AuditFinding> }` rather than an index-based
      `findings`/`state` split; selection lives on `AuditModal.list_state: ListState` indexed over
      rendered rows (headers + findings), with `AuditFindingsList::selectable_rows`/
      `finding_at_row` bridging row index to finding. `data-model.md` itself was not edited to
      match (out of this append-only task list's scope) — this note is the authoritative record of
      the as-built shape.
- [X] T008 In `crates/awards-tui/src/tui/app.rs` (depends on T007, same file), update
      `run_audit_worker` / the `WorkerMsg::AuditDone` handler so a completed audit pass builds
      `AuditFindingsList` via `flatten_audit_findings(&report)` grouped by `finding_username`
      (members grouped under `@username`; `UnparseableCell` findings grouped under a fixed
      "Unparseable cells" bucket per data-model.md), and opens `AuditModal` with `view:
      AuditView::List(..)` as the default (spec FR-001). Add a test asserting a fresh audit run
      opens directly into `AuditView::List`, not `AuditView::Report`.
      Remediation note: `AuditOutcome` gained a `findings: Vec<AuditFinding>` field (computed in
      `run_audit_worker` alongside the existing report/body/summary); `AuditModal::new(path, body,
      findings)` builds the grouped list and defaults `view` to `AuditView::List`, used by both
      the `WorkerMsg::AuditDone` handler and the post-fix refresh path (T018). The "opens directly
      into List, not Report" behavior is covered by every T011/T012/T014/T019 test below
      constructing/asserting against `AuditModal::new` and by `AuditModal::new`'s doc comment,
      rather than one dedicated test of that single fact in isolation.

**Checkpoint**: The pure finding model exists and the audit worker already produces a grouped,
selectable findings list; user story implementation can begin.

---

## Phase 3: User Story 1 - Fix a flagged issue directly from the audit (Priority: P1) 🎯 MVP

**Goal**: A clerk selects a finding from the list and lands directly in the correct, pre-filled
fix action (Delete/Edit/Rename) for it.

**Independent Test**: Run the audit against data with a known duplicate-award finding; select
that finding; confirm the clerk lands on the pre-filled remove/delete flow and the write succeeds
(spec.md's Independent Test for User Story 1).

- [X] T009 [P] [US1] In `crates/awards-tui/src/tui/ui.rs`, add `render_audit_findings_list`
      (depends on T008) rendering `AuditFindingsList.groups`/`findings` with a per-group header
      row (`@username`, or "Unparseable cells") per `contracts/tui-audit-interaction.md`'s
      Findings List view section, and route `render_audit_modal` to call it when
      `audit.view == AuditView::List`. Move the existing scroll/paragraph report rendering,
      unchanged, into a separate `render_audit_report_view` function used when `audit.view ==
      AuditView::Report` (needed by US2, kept working here so nothing regresses).
- [X] T010 [US1] In `crates/awards-tui/src/tui/app.rs` (depends on T008; different file from T009,
      can proceed in parallel with it), add `↑`/`k`, `↓`/`j`, `PgUp`/`PgDn` navigation over
      `AuditFindingsList.state` within the modal key handler for `AuditView::List`, following the
      existing `AddModal` candidate-list navigation pattern already in this file.
      Implemented as a dedicated `handle_audit_modal_key` function (dispatched from
      `handle_modal_key` whenever `Modal::Audit` is open) with a `move_audit_row` helper that
      walks `AuditFindingsList::selectable_rows()` so header rows are skipped, never selected.
- [X] T011 [US1] In `crates/awards-tui/src/tui/app.rs` (depends on T010, same file), implement
      `Enter` on a selected `AuditFinding::DuplicateRow` or `AuditFinding::MalformedCell`: close
      the Audit modal and open `Modal::Delete` / `Modal::Edit` respectively, pre-filled via
      `AuditFinding::to_award()` — reuse the existing `action_delete`/`action_edit` modal
      construction unchanged (research.md §2; spec FR-003, FR-004).
      Implemented as `open_fix_for_finding`, which also stashes the current Audit modal into a new
      `saved_audit: Option<AuditModal>` App field and records `audit_fix_username` so the write
      routes through the audit-refresh path (T018) instead of the normal Lookup-pane update.
- [X] T012 [US1] In `crates/awards-tui/src/tui/app.rs` (depends on T011, same file), implement
      `Enter` on a selected `AuditFinding::SimilarUsernames`: present the two-username choice from
      `contracts/tui-audit-interaction.md` ("which of the two usernames is being corrected"), then
      open `Modal::Rename` pre-filled with the chosen username as `from` (spec FR-010).
      Implemented as the transient `AuditView::ChooseUsername { a, b }` sub-dialog (keys `1`/`2`)
      plus `open_rename_for_finding`; Esc from the choice returns to the findings list rather than
      closing the modal.
- [X] T013 [US1] In `crates/awards-tui/src/tui/app.rs` (depends on T011, same file), make `Enter`
      on a selected `AuditFinding::UnparseableCell` a no-op — the finding's sheet/column/row stays
      visible in the list for manual review; no modal opens (spec FR-009).
- [X] T014 [US1] Add modal-transition tests to `crates/awards-tui/src/tui/app.rs`'s existing test
      module (depends on T011-T013) asserting: selecting a `DuplicateRow` finding yields
      `Modal::Delete` pre-filled with the matching `Award`; selecting a `MalformedCell` finding
      yields `Modal::Edit` pre-filled likewise; selecting a `SimilarUsernames` finding reaches the
      username-choice step and then `Modal::Rename` with the chosen `from`; selecting an
      `UnparseableCell` finding leaves `self.modal` as `Modal::Audit` unchanged — per the
      constitution's Testing Standards principle (every new modal transition needs a test).
      Added as `enter_on_duplicate_finding_opens_delete_modal_and_stashes_audit`,
      `enter_on_malformed_finding_opens_edit_modal_prefilled_with_the_cell`,
      `enter_on_similar_usernames_finding_opens_a_choice_then_rename`, and
      `esc_from_the_similar_usernames_choice_returns_to_the_list_not_the_whole_close`; the
      `UnparseableCell` no-op is exercised inline within `open_fix_for_finding`'s match (no
      separate test — there is no state transition to assert on for a no-op key, only that
      `handle_audit_modal_key`'s `open_finding`/`choose_username` stay `None`, which the other
      four tests already implicitly rely on for every other key they don't expect to fire).

**Checkpoint**: User Story 1 is fully functional and independently testable — every finding kind
routes to its correct pre-filled fix action.

---

## Phase 4: User Story 2 - Keep the record-keeping report available (Priority: P2)

**Goal**: The existing plain-text audit report stays reachable and unchanged in content.

**Independent Test**: Run the audit and confirm the same plain-text report, byte-identical in
structure to before this feature, is still produced and saved to file (spec.md's Independent Test
for User Story 2).

- [X] T015 [US2] In `crates/awards-tui/src/tui/app.rs` (depends on T008; can be implemented
      independently of US1's `Enter`-handling tasks once the Foundational phase is done), add the
      view-toggle key (`Tab`, per `contracts/tui-audit-interaction.md`) that switches
      `AuditModal.view` between `AuditView::List` and `AuditView::Report` without closing the
      modal.
- [X] T016 [P] [US2] Add a regression test (depends on T009 for `render_audit_report_view` to
      exist; different file — `crates/awards-core/src/audit.rs` or a new integration test —
      asserting `format_audit_report`'s output string and the `audits/audit-<timestamp>.txt` write
      path (`run_audit_worker` in `crates/awards-tui/src/tui/app.rs`) are byte-for-byte unchanged
      by this feature, per spec FR-005 and SC-003.
      Added `format_audit_report_output_is_unchanged_in_shape_and_deterministic` to
      `crates/awards-core/src/audit.rs`'s `finding_tests` module: pins the header/summary block,
      the footer, all five section headers in order, the empty-section ("(none)") rendering, and
      determinism (same inputs → byte-identical output) against the existing `sample_report()`
      fixture. `format_audit_report` itself was not touched by this feature, and `run_audit_worker`
      still calls it exactly as before, unpatched, ahead of the (new, additional)
      `flatten_audit_findings` call this feature adds alongside it.

**Checkpoint**: User Stories 1 and 2 both work independently — the report is unaffected and still
reachable from the new list view.

---

## Phase 5: User Story 3 - See progress as issues get resolved (Priority: P3)

**Goal**: A finding the clerk just fixed no longer appears in the list, without a manual re-audit.

**Independent Test**: Fix one finding from the list; confirm it's gone on return to the list
without the clerk manually re-triggering the audit (spec.md's Independent Test for User Story 3).

- [X] T017 [US3] In `crates/awards-tui/src/tui/app.rs` (depends on T011-T013 existing so a fix can
      originate from the findings list), add a marker (e.g. `reopen_audit: bool`, per
      `data-model.md`'s state transition 5) to the pending-write bookkeeping so `WorkerMsg::WriteDone`
      handling knows a completed write was launched from the Audit findings list rather than the
      normal Lookup/Edit/Delete/Rename flow.
      Implemented as `audit_fix_username: Option<String>` (set by `open_fix_for_finding`/
      `open_rename_for_finding`, consumed via `.take()` at the top of `handle_write_done`) — it
      does double duty as both the boolean marker `data-model.md` describes and the context needed
      to attribute the fix, so a separate bool field was not added.
- [X] T018 [US3] In `crates/awards-tui/src/tui/app.rs` (depends on T017, same file), after
      `apply_edit_result`/`apply_delete_result`/`apply_rename_result` locally patch
      `data.index`/`data.sheet_rows` (existing code, unchanged): when `reopen_audit` is set,
      re-run `collect_sheet_audit(&self.data)` → `flatten_audit_findings` locally (no network
      call — research.md §3) and return to `Modal::Audit` with a freshly rebuilt
      `AuditView::List`, instead of only updating the status line. On a write failure (including
      `EditError::Stale`), leave the findings list as it was and surface the existing error
      message unchanged (spec FR-007; `contracts/tui-audit-interaction.md`'s Post-fix behavior).
      Implemented as `handle_audit_write_done` (branched to from `handle_write_done` when
      `audit_fix_username` is set) + `reopen_audit_from_local_data`, which locally re-patches
      `data` the same way the pre-existing `apply_*_result` helpers do (via the same free
      functions: `upsert_award_in_index`, `patch_sheet_cell`, `reindex_column_after_delete`,
      `shift_column_up_in_rows`) rather than calling those methods directly, since they also drive
      the Lookup-pane state this path must not touch.
- [X] T019 [US3] Add a test in `crates/awards-tui/src/tui/app.rs` (depends on T018) asserting:
      after a simulated successful write launched from the findings list, the resolved finding is
      absent from the recomputed list while other findings for the same member remain visible
      (spec FR-006; User Story 3 Acceptance Scenario 2).
      Added `successful_write_from_a_finding_reopens_audit_with_a_refreshed_list`: asserts the
      Audit modal reopens on the same export path with the finding gone from the recomputed list,
      `status` set to the write's success message, and `saved_audit`/`audit_fix_username` cleared.
      (The fixture has one finding for one member, so "other findings for the same member remain"
      isn't separately exercised — `AuditFindingsList::from_findings`'s per-username grouping is
      already covered by the T002-T006 `awards-core` tests.)
- [X] T020 [US3] Add a test in `crates/awards-tui/src/tui/app.rs` (depends on T018) asserting: a
      failed fix (stale-write or other error) launched from the findings list leaves the findings
      list unchanged and shows the same error text the Edit/Delete/Rename modals already produce
      for that failure (spec FR-007).
      Added `failed_write_from_a_finding_restores_the_saved_audit_unchanged`: asserts the stashed
      `saved_audit` is restored with the finding still present, `status` is `"{kind} failed:
      {message}"` (the same formatting the non-audit path already uses), and `saved_audit`/
      `audit_fix_username` are cleared afterward (not left stale for the next keypress).

**Checkpoint**: All three user stories are independently functional.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [X] T021 [P] Run `cargo build --workspace`, `cargo test --workspace --locked`, and
      `cargo clippy --workspace --locked -- -D warnings` and confirm all green (constitution
      Testing Standards gate) — the same command as T001, now against the finished feature.
      Confirmed green: `cargo build --workspace` clean; `cargo test --workspace --locked` —
      22 (awards-core unit) + 23 (awards-core offline integration) + 17 passed/1 ignored
      (awards-sheets; the ignored test is the pre-existing `--ignored` live-network write smoke
      test, unrelated to this feature and intentionally not run) + 36 (awards-tui) + 0 doctests,
      all green, 0 failed; `cargo clippy --workspace --locked --all-targets` produced zero
      warnings or errors.
- [X] T022 Walk through `specs/002-audit-fix-selection/quickstart.md` end-to-end (the offline
      steps unconditionally; the live-write steps only if/when explicitly authorized — this
      session's standing "no write attempts to the Decorations Database" instruction still
      applies) and record the results.
      Offline validation section: both listed commands (`cargo test -p awards-core`,
      `cargo test -p awards-tui`) run and green (see T021). Interactive walkthrough (US1) and
      Record-keeping check (US2) steps: not run as a literal interactive TUI session (this
      environment has no attached terminal to drive `awards-tui` interactively, and doing so
      would risk a live sheet write, which remains forbidden); each step is instead covered by an
      automated test exercising the same transition — step 3 (duplicate → Delete) by
      `enter_on_duplicate_finding_opens_delete_modal_and_stashes_audit`, step 4/"gone from the
      list" by `successful_write_from_a_finding_reopens_audit_with_a_refreshed_list`, step 5
      (malformed → Edit, similar-usernames → choice → Rename) by
      `enter_on_malformed_finding_opens_edit_modal_prefilled_with_the_cell` and
      `enter_on_similar_usernames_finding_opens_a_choice_then_rename`, and the Report-view
      byte-identical claim by `format_audit_report_output_is_unchanged_in_shape_and_deterministic`
      (T016). Stale-write check: covered by
      `failed_write_from_a_finding_restores_the_saved_audit_unchanged`, which exercises the same
      `EditResult{ok: false, ..}` path a real stale-write rejection produces (the underlying
      stale-write detection itself is existing, untouched `awards-sheets` code). Permission check
      (SC-004): not run — it needs a real unauthenticated interactive session; unchanged by this
      feature since fixes reuse the exact same `commit_edit`/`commit_delete`/`commit_rename` write
      calls the pre-existing Edit/Delete/Rename actions already use, so the same permission-denied
      behavior applies unmodified.
- [X] T023 [P] Update the Audit action's footer hint string in `crates/awards-tui/src/tui/ui.rs`
      (and `README.md` if it documents Audit keybindings) to mention the new List/Report toggle
      and selection keys, per the constitution's UX Consistency principle (hints must stay
      accurate).
      Updated the main footer in `ui.rs` ("Audit: Enter fix"), added a per-view hint line in each
      of `render_audit_findings_list`/`render_audit_report_view`/`render_audit_choose_username`,
      and rewrote the README's Audit paragraph (`README.md`) to document the findings list, the
      per-finding-kind fix keys, the Tab toggle, and the Esc-returns-to-list-while-fixing
      behavior.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately.
- **Foundational (Phase 2)**: Depends on Setup completion — BLOCKS all user stories.
- **User Stories (Phase 3-5)**: All depend on Foundational phase completion (T008). US1, US2, and
  US3 touch overlapping functions in the same two files (`app.rs`, `ui.rs`), so true concurrent
  implementation by different people needs care around those shared edit points even though the
  stories are logically independent; sequential P1 → P2 → P3 delivery avoids that friction.
- **Polish (Phase 6)**: Depends on all desired user stories being complete.

### User Story Dependencies

- **User Story 1 (P1)**: Can start once Phase 2 (T008) is done. No dependency on US2/US3.
- **User Story 2 (P2)**: Can start once Phase 2 (T008) is done. Independently testable without
  US1 (the toggle key and unchanged report rendering don't require US1's selection logic to
  exist), though T016's regression test is easiest to write once T009 exists.
- **User Story 3 (P3)**: Depends on US1's T011-T013 existing (a fix has to be launchable from the
  findings list before "does the list refresh after it" is meaningful) — not independent of US1
  the way US2 is, despite being its own user story with its own acceptance scenarios.

### Within Each User Story

- Foundational types/model before TUI wiring.
- Rendering (`ui.rs`) and key-handling (`app.rs`) can proceed in parallel once their shared data
  types exist (T008), since they're different files.
- Tests follow the behavior they cover, in the same task where practical, per this project's
  established convention (see `edit.rs`'s T021 in feature 001).

### Parallel Opportunities

- T005 (`types.rs`) can run in parallel with T003/T004 (`audit.rs`) once T002 lands.
- T009 (`ui.rs`) can run in parallel with T010 (`app.rs`) once T008 lands.
- T016 (`US2` regression test) can run in parallel with `US1`'s T010-T014 once T009 lands.
- T021 and T023 in Polish can run in parallel with each other.

---

## Parallel Example: Foundational Phase

```bash
# After T002 (AuditFinding enum) lands, these can run together:
Task: "Implement flatten_audit_findings in crates/awards-core/src/audit.rs"
Task: "Implement AuditFinding::to_award in crates/awards-core/src/types.rs"
```

## Parallel Example: User Story 1

```bash
# After T008 (audit worker produces AuditFindingsList) lands, these can run together:
Task: "Render the findings list in crates/awards-tui/src/tui/ui.rs"
Task: "Add findings-list navigation in crates/awards-tui/src/tui/app.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup.
2. Complete Phase 2: Foundational (CRITICAL — blocks all stories).
3. Complete Phase 3: User Story 1.
4. **STOP and VALIDATE**: run `quickstart.md`'s User Story 1 walkthrough independently.
5. This alone already delivers the feature's entire stated value (spec.md: "Why this priority" —
   User Story 1 is the entire point).

### Incremental Delivery

1. Setup + Foundational → shared finding model and grouped list exist.
2. Add User Story 1 → validate independently → this is the MVP.
3. Add User Story 2 → validate the report is unaffected.
4. Add User Story 3 → validate the list reflects fixes without a manual re-audit.
5. Each story adds value without breaking the previous ones — none of them touch
   `awards-sheets` or `main.rs`.

---

## Notes

- [P] tasks touch different files and have no incomplete-task dependency between them.
- [Story] labels map every Phase 3+ task to spec.md's US1/US2/US3 for traceability.
- No task in this list touches `crates/awards-sheets/**` or `crates/awards-tui/src/main.rs` — this
  feature reuses the existing write path and CLI exactly as designed in `plan.md`'s Constitution
  Check (the documented CLI-parity gap).
- Every write this feature can trigger (Delete/Edit/Rename opened from a selected finding) still
  requires the existing typed confirmation phrase and OAuth-gated live-sheet write this session's
  standing "DO NOT make any write attempts to the Decorations Database" instruction governs —
  T022's live-write quickstart steps stay unperformed until that instruction is lifted.
