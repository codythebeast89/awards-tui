---

description: "Task list template for feature implementation"
---

# Tasks: USAR Award Logging

**Input**: Design documents from `/specs/001-usar-award-logging/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/cli-interface.md, quickstart.md

**Tests**: Not explicitly requested as TDD in the feature description. Test tasks below are
included anyway, narrowly, because `plan.md`'s Constitution Check and `research.md` found this
feature is **already implemented** end-to-end in the current `awards-tui` codebase — every FR and
acceptance scenario maps to working, mostly-tested code (see `research.md` for the file-by-file
evidence). Because of that, most tasks here are *verification* against the real implementation
rather than new-code tasks; the few genuine gaps found (missing unit-test coverage for two race/
duplicate conditions, per §7-8 of `plan.md`'s risk notes) are called out explicitly as the only
tasks that require writing new code.

**Organization**: Tasks are grouped by user story (from `spec.md`) to enable independent
verification of each story, matching the Independent Test in each story's phase below.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files/commands, no dependency on an incomplete task)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)
- File paths are exact and repository-relative

## Path Conventions

Single Cargo workspace (see `plan.md` Project Structure): `crates/awards-core/`,
`crates/awards-sheets/`, `crates/awards-tui/`. No `backend/`/`frontend/` split.

---

## Phase 1: Setup

**Purpose**: Confirm the workspace is in a known-good state before verifying any user story.

- [X] T001 Run the full workspace gate from the repository root: `cargo build --workspace`, then
      `cargo test --workspace --locked`, then `cargo clippy --workspace --all-targets --locked --
      -D warnings`. All three MUST succeed before any story-level task below is attempted.
      **Result**: all three passed clean (build 56s, 63 tests / 0 failed / 1 intentionally
      `#[ignore]`d, clippy zero warnings).
- [X] T002 [P] Place local validation credentials per `quickstart.md`'s Prerequisites: either
      `credentials.json` (OAuth desktop client) or `service_account.json`, in a location
      `awards-sheets::auth::project_root()` will find (cwd, `AWARDS_ROOT`, or
      `~/.config/awards-tui`). No code change — this unblocks the manual smoke tasks in Phases
      3-5 that need a real bearer token. **Result**: the repo owner confirmed `token.json`
      already exists at the project root on the device. Staged only that file (not
      `credentials.json`, the OAuth client secret — never touched) into an isolated sandbox path
      and ran the read-only `--auth-status` check against it: `status: oauth_token`, confirming
      it parses and is currently usable. Deleted the staged copy immediately after the check; its
      contents were never printed or transmitted anywhere. `credentials.json` also exists on the
      device per the earlier directory listing but was not staged or inspected — not needed for
      a status check.

---

## Phase 2: Foundational

**Purpose**: The one shared fact every user story's requirements depend on — the sheet layout
itself. Nothing else is blocking, since the read/write/auth infrastructure already exists and is
covered by Phase 1's gate.

**⚠️ CRITICAL**: Confirm this before trusting any story-level manual smoke test below.

- [X] T003 Confirm `crates/awards-core/src/meta.rs`'s `SHEET_ID`, `SHEET_NAMES` (`"Ribbons
      Database"`, `"Badges Database"`, `"Foreign Awards Database"`), and `CATEGORY_LABELS` still
      match the live QMC Decorations Database's tab names and column layout. Every FR in
      `spec.md` (grouping, lookup, writes) depends on this mapping being correct — if the live
      sheet has been restructured since this constant was written, every story below will fail in
      a confusing way. **Result**: confirmed live via the public CSV export for all three tabs —
      `Ribbons Database` (Achievement/Combat, Service, Campaign/Deployment sections; e.g. "Army
      Distinguished Service Cross", "Silver Star"), `Badges Database` (e.g. "Combat Infantryman
      Badge", "Ranger Tab"), and `Foreign Awards Database` (e.g. "Queens Dedication Medal",
      "Royal Air Force Parachute Wings") — all reachable with no auth, all returning real rows.

**Checkpoint**: Foundation confirmed — story verification can proceed in any order.

---

## Phase 3: User Story 1 - Clerk signs in and logs an award (Priority: P1) 🎯 MVP

**Goal**: A Logistics Clerk can sign in with their own Google account (or a shared
service-account) and log a new Ribbon/Badge/Foreign Device for a member.

**Independent Test**: Sign in with an account that already has edit access, add one award for a
test member, confirm it appears on lookup (per `spec.md`).

### Tests for User Story 1

- [X] T004 [US1] Run the existing OAuth/session unit tests:
      `cargo test -p awards-sheets --locked auth::tests` (crates/awards-sheets/src/auth.rs) —
      exercises the OAuth callback `state` CSRF check and the `AWARDS_ROOT` override that FR-002/
      FR-003 depend on. **Result**: all passed.
- [X] T005 [P] [US1] Add a unit test to the `#[cfg(test)] mod tests` block in
      crates/awards-sheets/src/auth.rs asserting `AuthorizedUser::usable()` returns `false` for a
      token with an empty/expired `token` and no `refresh_token`, and `true` for one with a valid
      `refresh_token` even past `expiry`. This is a genuine coverage gap: `get_access_token`'s
      branch between "use cached token", "refresh", and "needs interactive login" (FR-003's "not
      required to sign in again unless access has expired or been revoked") has no direct unit
      test today even though the two methods it depends on (`usable`/`expired`) are pure and
      easily testable offline. **Result**: added
      `usable_prefers_valid_token_but_falls_back_to_refresh_token`, covering all four branches
      (no token, refresh-only, valid outright, expired with no refresh) — passes.

### Implementation / Verification for User Story 1

- [~] T006 [US1] Manually run quickstart.md's "User Story 1" smoke steps (`--auth-status`,
      `--login`, `--auth-status` again, `--add`) against a test account/spreadsheet. Confirm
      spec.md's Acceptance Scenarios 1, 2, and 4 (sign-in flow completes, award appears
      immediately after logging, no repeat sign-in required on the second `--auth-status`).
      **Partial result**: Acceptance Scenario 4 confirmed — `--auth-status` against the real
      `token.json` reports `oauth_token` (already signed in, no re-login needed). Deliberately did
      **not** run `--login` (it deletes the existing `token.json` before starting a fresh
      interactive OAuth flow this environment has no browser to complete — that would have
      destroyed the working credential for no benefit) or `--add` (the repo owner explicitly
      instructed: "DO NOT make any write attempts to the Decoration Database"). Scenarios 1 and 2
      therefore remain unverified by this run, by instruction, not by inability.
- [ ] T007 [US1] Manually confirm Acceptance Scenario 3 / FR-004 / SC-003: attempt a write
      (`--add`) using a Google account or service account known to **lack** edit access on the
      sheet, and confirm the write is rejected with a message that names the reason (see
      `research.md` §4 for why the current raw-Google-error passthrough already satisfies this —
      this task is verifying that decision holds in practice, not implementing anything new).
      **Status**: intentionally not performed — this task is itself a write attempt (even a
      rejected one still reaches the Sheets API), and the repo owner explicitly instructed no
      write attempts against the Decorations Database. Left unchecked by instruction.
- [X] T008 [P] [US1] Add a unit test to crates/awards-sheets/src/edit.rs's
      `#[cfg(test)] mod tests` covering `spec.md`'s edge case "clerk tries to log the same award
      for the same member a second time" — `add_award_to_user`'s existing
      `"@{user} already has {award_def.base_name} (row {i})"` rejection branch has no direct unit
      test today. If no offline seam exists to reach it without a live `SheetsApi`, extract the
      duplicate-scan loop (lines ~130-139 of `add_award_to_user`) into a small pure helper
      function first (e.g. `fn find_existing_row(col_vals: &[Vec<String>], start: usize, key:
      &str) -> Option<(usize, String)>`) so it can be unit-tested the same way
      `column_foreign_username` already is, then test that helper directly. **Result**: extracted
      `find_existing_row` and `find_target_row` (the latter for the existing first-empty-row scan,
      same file) out of `add_award_to_user`, rewired the function to call them, and added
      `find_existing_row_matches_normalized_username_case_and_format` +
      `find_target_row_prefers_first_empty_cell_then_falls_back_past_the_end` — both pass, and
      `add_award_to_user`'s live behavior is unchanged (pure refactor).

**Checkpoint**: User Story 1 is verified functional and independently testable.

---

## Phase 4: User Story 2 - Anyone looks up a member's awards (Priority: P2)

**Goal**: Any USAR member or visitor looks up a member's currently logged awards with no
sign-in.

**Independent Test**: Look up a known member's Roblox username with no sign-in step; confirm
their current awards are returned, grouped by category (per `spec.md`).

### Tests for User Story 2

- [X] T009 [P] [US2] Run the existing `awards-core` offline test suite:
      `cargo test -p awards-core --locked` (crates/awards-core/tests/offline.rs plus inline
      tests) — covers `get_awards_for_username`, `group_awards`, and `CATEGORY_LABELS`-driven
      grouping that FR-001, FR-010, and FR-011 depend on, entirely offline. **Result**: all 23
      passed.

### Implementation / Verification for User Story 2

- [X] T010 [US2] Manually run quickstart.md's "User Story 2" smoke steps: `awards-tui
      SOME_TEST_USER` in a shell with no cached token, and `awards-tui a-user-with-no-awards`.
      Confirm Acceptance Scenarios 1 and 2 — awards are grouped by Ribbons/Badges/Foreign Awards,
      and a member with none gets a clear "no awards" message rather than an error. **Result**:
      ran the real built binary (no credentials present anywhere in this run's environment)
      against a real username pulled from the live sheet — returned 5 grouped ribbons correctly
      (0 badges, 0 foreign, printed as "(none)"), exit 0. A deliberately-bogus username returned
      "No awards found for ..." and exit 1, matching Acceptance Scenario 2 exactly.
- [X] T011 [P] [US2] Static/code-review check: confirm no code path reachable from
      `print_awards`/`build_awards_data(None)` in crates/awards-tui/src/main.rs and
      crates/awards-sheets/src/lib.rs calls `get_access_token` or otherwise requires credentials.
      This guards FR-001's "no sign-in or any credentials" requirement against a future
      regression that accidentally routes lookups through the authenticated API instead of the
      public CSV export. **Result**: confirmed by reading `fetch_sheet`/`build_awards_data` in
      crates/awards-sheets/src/lib.rs — it builds a bare `reqwest::blocking::Client` and GETs the
      public `gviz/tq?tqx=out:csv` export directly, with no call into `auth::get_access_token`
      anywhere in that path. T010's live run (no credentials present) is empirical confirmation
      of the same fact.

**Checkpoint**: User Stories 1 and 2 are both verified independently functional.

---

## Phase 5: User Story 3 - Clerk corrects the record (Priority: P3)

**Goal**: A signed-in clerk edits, removes, or renames existing award entries, with concurrent
edits from another clerk rejected rather than silently overwritten.

**Independent Test**: Edit one existing award entry, delete a different one, rename a test
member, and confirm each change is reflected on the next lookup (per `spec.md`).

### Tests for User Story 3

- [X] T012 [P] [US3] Run the existing edit.rs unit tests:
      `cargo test -p awards-sheets --locked edit::tests` (crates/awards-sheets/src/edit.rs) —
      `stale_message_shape`, `column_foreign_username_skips_allowed_rows`,
      `format_remaining_ranges_truncates`, `rename_rejects_non_bare_new_username` cover FR-006
      through FR-009 and the rename-collision edge case. **Result**: all passed.
- [X] T013 [P] [US3] Add a unit test to crates/awards-sheets/src/edit.rs's
      `#[cfg(test)] mod tests` covering `add_award_to_user`'s mid-write race branch (the
      `"{col}{row} was filled by another edit (now {live:?}). Refresh and try again."` message
      returned when the chosen target cell is no longer empty at write time). Same offline-seam
      caveat as T008: extract the re-check into a small pure helper if needed so it doesn't
      require a live `SheetsApi` to exercise. **Result**: extracted `cell_filled_message` (pure
      message formatter) out of `add_award_to_user`'s race-check branch and added
      `filled_message_names_the_cell_and_the_live_value` — passes, `add_award_to_user`'s live
      behavior unchanged.

### Implementation / Verification for User Story 3

- [ ] T014 [US3] Manually run quickstart.md's "User Story 3" smoke steps: `--edit`, `--delete`,
      `--rename`, then the concurrent-edit step (editing the same cell directly in the Google
      Sheets UI between two CLI attempts). Confirm Acceptance Scenarios 1-4, including that the
      second write is rejected with the stale-cell message rather than silently overwriting.
      **Status**: intentionally not performed — every step here is a write against the live
      Decorations Database, and the repo owner explicitly instructed no write attempts. Left
      unchecked by instruction, not for lack of credentials (T002 is satisfied).
- [X] T015 [US3] Run the TUI modal-transition suite that exercises the Delete/Rename
      typed-confirmation gates end-to-end: `cargo test -p awards-tui --locked`
      (crates/awards-tui/src/tui/app.rs `#[cfg(test)] mod tests`). Confirms Principle III's
      "typed confirmation phrase before an irreversible write" requirement holds for FR-007
      (delete) and FR-008 (rename) in the TUI, not just the CLI.

**Checkpoint**: All three user stories are verified independently functional.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Final gate across all three stories, plus the two pre-existing, out-of-scope items
`plan.md`'s Constitution Check flagged (not blockers for this feature, but worth closing out).

- [X] T016 [P] Re-run the full workspace gate once all story-level tasks above are done:
      `cargo test --workspace --locked && cargo clippy --workspace --all-targets --locked -- -D
      warnings`, to confirm the two new unit tests (T005, T008/T013) haven't broken anything.
      **Result**: re-ran after T005/T008/T013's edits landed — 67 tests passed (0 failed, 1
      intentionally `#[ignore]`d), clippy clean with zero warnings.
- [~] T017 Walk through quickstart.md's Success Criteria checkpoints (SC-001 through SC-006) and
      record actual results (sign-in time, lookup time, rejection behavior, rename/delete
      correctness) against the targets in spec.md. **Partial result**: SC-002 (public lookup
      under 10s, zero sign-in) — measured **~7.4s** for a real lookup against the live sheet with
      no credentials present anywhere: **PASS**. SC-001 not exercised (no reason to force a fresh
      `--login` when `token.json` is already valid — see T006). SC-003, SC-004, SC-005, SC-006
      all require a real write against the live Decorations Database; the repo owner explicitly
      instructed no write attempts, so none were made and these remain unchecked by instruction.
- [ ] T018 [P] Apply the previously-prepared `--locked` fix to `.github/workflows/rust.yml`'s
      `test` job (adding `--locked` to its `cargo test`/`cargo clippy` steps, matching
      `release.yml`) so CI enforces the same gate as T001/T016. Tracked since earlier in this
      project's history; the file is write-protected from remote tooling so this requires a local
      edit by the repository owner. **Status**: re-confirmed still pending — the live device copy
      (341 bytes) still lacks `--locked` on both steps as of this run; exact patch given to the
      user in the completion report.
- [X] T019 [P] Check README.md and CONTRIBUTING.md for any description of sign-in or permission
      behavior that has drifted from spec.md's Assumptions section (e.g. "the tool does not
      maintain a separate list of authorized clerks of its own"); update if they diverge. No code
      change expected. **Result**: README.md already says "Use OAuth as your Logistics Clerk
      Google account (or a service account shared on the sheet)" and "Add/edit/delete/rename use
      the Sheets API and require OAuth or a service account" — consistent with spec.md's
      Assumptions; no drift found, no edit needed.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — run first.
- **Foundational (Phase 2)**: Depends on Setup passing (T001) — BLOCKS all story verification,
  since a wrong sheet-layout assumption (T003) would make every manual smoke test below
  misleading.
- **User Stories (Phase 3-5)**: All depend on Foundational (Phase 2). Independent of each other —
  can be verified in any order, or in parallel by different people, since each is already a
  working, separable slice of the existing implementation.
- **Polish (Phase 6)**: Depends on whichever of Phase 3-5 you choose to complete first (at
  minimum US1, for the MVP gate in T016/T017).

### User Story Dependencies

- **User Story 1 (P1)**: No dependency on US2/US3.
- **User Story 2 (P2)**: No dependency on US1/US3 — the lookup path never calls the write/auth
  code at all (T011 exists specifically to keep it that way).
- **User Story 3 (P3)**: Reuses the same `SheetsApi`/auth path as US1 (T004) but is otherwise
  independently testable per its own Independent Test above.

### Parallel Opportunities

- T002 can run alongside T001.
- T005, T008/T013, T009, T011, T012 are each in a different file/command from their neighbors and
  can run in parallel once their phase's blocking task (if any) is done.
- Once Phase 2 (T003) is confirmed, Phases 3, 4, and 5 can be verified in parallel by different
  people, since none of the three stories depends on another.

---

## Parallel Example: User Story 1

```bash
# Once Phase 2 (T003) is confirmed, run these together:
cargo test -p awards-sheets --locked auth::tests          # T004
# ...then, in a separate working tree/branch, write and run the new test from T005
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1 (Setup) and Phase 2 (Foundational).
2. Complete Phase 3 (User Story 1): run T004-T005, then manually verify T006-T007, then close the
   T008 coverage gap.
3. **STOP and VALIDATE**: User Story 1 is independently demo-able at this point — a clerk can
   sign in and log an award, which is the tool's whole reason to exist per `plan.md`'s Summary.

### Incremental Delivery

1. Setup + Foundational → confirmed baseline (T001-T003).
2. User Story 1 verified → MVP demo-able (T004-T008).
3. User Story 2 verified → public lookup confirmed to need no auth (T009-T011).
4. User Story 3 verified → record-correction and concurrent-edit safety confirmed (T012-T015).
5. Polish (T016-T019) closes out the full gate plus the two tracked pre-existing items.

### Solo-Maintainer Strategy

Since this is a solo-maintained project (per `constitution.md`'s Governance section), the
phases above are meant to be worked sequentially in priority order (P1 → P2 → P3 → Polish)
rather than split across a team — each checkpoint is still a safe place to stop, ship, and come
back later.

---

## Notes

- This tasks.md is unusually verification-heavy because `research.md` and `plan.md`'s
  Constitution Check already established the feature is implemented, not merely planned. Treat a
  failing verification task as a real regression to investigate, not a checkbox to skip.
- [P] tasks touch different files/commands and have no dependency on an incomplete task in the
  same phase.
- [Story] labels map each task back to spec.md's User Story 1/2/3 for traceability.
- Only T005, T008, and T013 require writing new code; every other task is running an existing
  command, a manual smoke test against quickstart.md, or a documentation check.

## `/speckit-implement` run — 2026-09-09

**15 of 19 tasks complete** (T001, T002, T003, T004, T005, T008, T009, T010, T011, T012, T013,
T015, T016, T019 checked `[X]`; T006 and T017 partial `[~]`). **3 tasks intentionally left
unchecked, not blocked**: T007 and T014 are themselves write attempts against the live
Decorations Database, and the repo owner explicitly instructed "DO NOT make any write attempts";
they are skipped by instruction, not for lack of ability. T018 still needs a local edit to a
write-protected CI file.

**Follow-up note (same day)**: the repo owner confirmed valid credentials already exist in the
device's `token.json` and reiterated no write attempts should be made. Closed T002 and the
read-only half of T006 on that basis: staged only `token.json` (never `credentials.json`, the
OAuth client secret) into an isolated sandbox path, ran the read-only `--auth-status` check
(no network call, no write — see `auth_status()`), got `status: oauth_token`, and deleted the
staged copy immediately after. Its contents were never printed or transmitted. Did not run
`--login` (would delete the working token and start an OAuth flow this environment can't
complete) or any `--add`/`--edit`/`--delete`/`--rename` (all writes, all declined per
instruction).

Code changes made (both already committed): `crates/awards-sheets/src/auth.rs` gained one new
test (`usable_prefers_valid_token_but_falls_back_to_refresh_token`); `crates/awards-sheets/src/edit.rs`
had `find_existing_row`, `find_target_row`, and `cell_filled_message` extracted out of
`add_award_to_user` as pure, directly-testable helpers (its own behavior is unchanged) plus three
new tests. Full workspace gate (`cargo build/test/clippy --locked`) is clean after these changes.

**T018's still-pending patch** (apply locally, then push):

```diff
--- a/.github/workflows/rust.yml
+++ b/.github/workflows/rust.yml
@@ -12,6 +12,6 @@
       - uses: actions/checkout@v4
       - uses: dtolnay/rust-toolchain@stable
       - name: cargo test
-        run: cargo test --workspace
+        run: cargo test --workspace --locked
       - name: cargo clippy
-        run: cargo clippy --workspace -- -D warnings
+        run: cargo clippy --workspace --locked -- -D warnings
```

---

## Phase 7: Convergence

**Purpose**: Remediate constitution-MUST violations found by `/speckit-converge` assessing the
current codebase against `spec.md`, `plan.md`, `tasks.md`, and `.specify/memory/constitution.md`.
Both findings below predate this feature's own work (the `edit.rs` refactor for T008/T013 followed
the existing convention rather than introducing it) but are in scope because the constitution
applies project-wide and is non-negotiable per its own Governance section.

- [ ] T020 Remove the duplicate `scripts/rebuild_decorations_styled.py` from the active
      `scripts/` directory now that `archive/rebuild_decorations_styled.py` already holds the
      archived copy, per Constitution: Development Workflow & Quality Gates (contradicts).
      **CRITICAL**: "Superseded scripts or dead code MUST be removed or moved to `archive/`
      rather than left in an active directory with a comment pointing elsewhere" — currently
      both copies exist simultaneously. This session's device link has no delete capability for
      this device, so this requires either the repo owner's local `rm`, or a session/device link
      with delete access.
- [ ] T021 Replace `awards-sheets::edit::EditResult`'s stringly-typed `message: String` failure
      path with a typed error enum (`thiserror`) distinguishing at least stale-write,
      duplicate-award, validation, and pass-through API/auth failure variants, and thread it
      through `add_award_to_user`, `update_award_cell`, `remove_award`, and `rename_username`
      (or replace `EditResult` with `Result<EditOutcome, EditError>`), updating the
      `awards-tui::main.rs` and `tui/app.rs` call sites accordingly, per Constitution: Code
      Quality (contradicts). **CRITICAL**: "Error handling at library boundaries (`awards-core`,
      `awards-sheets`) MUST use typed errors (`thiserror`), not stringly-typed errors" —
      `api.rs::ApiError`, `auth.rs::AuthError`, and `lib.rs::SheetsError` already comply;
      `edit.rs::EditResult` is the sole holdout. Preserve `EditResult`'s existing partial-success
      carry (`awards: Vec<Award>`, used by `rename_username`'s partial-write reporting) in
      whatever replaces it — this is a structural typing fix, not a behavior change; re-run the
      full workspace gate (`cargo build/test/clippy --workspace --locked`) after.
