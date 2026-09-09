# Quickstart: Validating USAR Award Logging

This walks through each user story's acceptance scenarios against the real `awards-tui` binary,
plus the automated test suites that already cover the same behavior offline. Run the automated
checks first — they need no credentials or network — then the manual CLI smoke steps against a
real (ideally test) spreadsheet/account if you want to validate the live Google-integrated path.

## Prerequisites

- Rust toolchain matching the workspace (`cargo build --workspace` succeeds).
- For the manual write steps only: either `credentials.json` (OAuth desktop client) or
  `service_account.json`, placed per `awards-sheets::auth::project_root()`'s search order (cwd,
  `AWARDS_ROOT`, or `~/.config/awards-tui`), and network access to `sheets.googleapis.com` /
  `accounts.google.com`.
- For the manual read step only: network access to the sheet's public CSV export — no
  credentials needed.

## Automated validation (offline, no credentials)

```sh
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

What this proves, mapped to the spec:

- `crates/awards-sheets/src/edit.rs` unit tests (`stale_message_shape`,
  `column_foreign_username_skips_allowed_rows`, `rename_rejects_non_bare_new_username`,
  `format_remaining_ranges_truncates`) — FR-006 through FR-009.
- `crates/awards-sheets/src/auth.rs` unit tests (`extract_code_from_request_line`,
  `extract_state_and_error`, `awards_root_env_wins`, `secret_mode_flags_group_or_world`) —
  FR-002/FR-003 and the Security & Data Integrity OAuth-CSRF and secret-permission requirements.
- `crates/awards-tui/src/tui/app.rs`'s `#[cfg(test)] mod tests` (~29 tests) — every TUI
  Add/Edit/Delete/Rename modal transition referenced by User Stories 1 and 3, including the
  typed-confirmation gates for Delete and Rename.

## Manual smoke: User Story 1 — clerk signs in and logs an award

1. `awards-tui --auth-status` → expect `missing` (or `oauth_needs_login`) before first login.
2. `awards-tui --login` → completes the browser OAuth flow (or validates a service account) and
   reports success. Acceptance Scenario 1.
3. `awards-tui --auth-status` again → now reports `oauth_token` (or `service_account`).
   Acceptance Scenario 4 (no repeat sign-in needed).
4. `awards-tui SOME_TEST_USER --add "Army Service"` → reports `Added ... at <col><row>`.
   Acceptance Scenario 2.
5. Look the same user up (Story 2 below) and confirm the new award now appears.

## Manual smoke: User Story 2 — anyone looks up a member's awards

1. `awards-tui SOME_TEST_USER` (no `--login` run first, in a shell with no cached token) →
   prints the member's awards grouped under `Ribbons (...)`, `Badges (...)`, `Foreign Awards
   (...)`. Acceptance Scenario 1.
2. `awards-tui a-user-with-no-awards` → prints `No awards found for a-user-with-no-awards` and
   exits `1` rather than erroring. Acceptance Scenario 2.

## Manual smoke: User Story 3 — clerk corrects the record

1. `awards-tui SOME_TEST_USER --edit "Army Service" --cell "SOME_TEST_USER x2"` → reports
   `Updated <col><row> → SOME_TEST_USER x2`. Acceptance Scenario 1.
2. `awards-tui SOME_TEST_USER --delete "Army Service"` → reports `Removed ... (column shifted
   up)`; re-lookup confirms no gap. Acceptance Scenario 2.
3. `awards-tui OLD_NAME --rename NEW_NAME` → reports `Renamed @OLD_NAME → NEW_NAME in N cell(s)`;
   re-lookup `NEW_NAME` and confirm every previously-OLD_NAME award is now found there, and
   `OLD_NAME` has none. Acceptance Scenario 3.
4. To exercise Acceptance Scenario 4 (concurrent-edit rejection), edit the same cell directly in
   the Google Sheets UI between steps, then retry the CLI edit — expect a `... changed on the
   sheet (now ...; expected ...). Refresh and try again.` failure rather than a silent overwrite.

## Success Criteria checkpoints

- SC-001/SC-002: time the `--login` flow and a lookup by hand against the 2-minute / 10-second
  targets.
- SC-003: attempt a write with an account known to lack sheet edit access; confirm it fails
  (non-zero exit) and the printed message names the rejection.
- SC-004: the concurrent-edit step above.
- SC-005/SC-006: the rename and delete steps above.
