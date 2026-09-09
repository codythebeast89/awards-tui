# Phase 0 Research: USAR Award Logging

All items below were resolved by direct inspection of the existing `awards-tui` workspace rather
than by assumption — this feature largely formalizes already-implemented behavior, so "research"
here means confirming the real implementation satisfies each FR, not picking a new technology.
No `[NEEDS CLARIFICATION]` markers remained in `spec.md`, so there are no open questions to
resolve; this document instead records the grounding for the Technical Context and Constitution
Check in `plan.md`.

## 1. Does the CLI already offer parity with the TUI (Principle III)?

- **Decision**: Yes — treat CLI/TUI parity as already satisfied; no new CLI flags are needed for
  this feature.
- **Rationale**: `crates/awards-tui/src/main.rs`'s `Cli` struct (Clap derive) already exposes
  `--login`, `--auth-status`, `--add AWARD --suffix`, `--edit AWARD --cell`, `--delete AWARD`,
  `--rename NEW`, plus bare `username` / `--cli` for lookup — a 1:1 mapping to User Story 1 (sign
  in, log an award), User Story 2 (lookup), and User Story 3 (correct, remove, rename) via
  `cmd_login`, `cmd_add`, `cmd_edit`, `cmd_delete`, `cmd_rename`, `print_awards`.
- **Alternatives considered**: Treating this as `[NEEDS CLARIFICATION]` and asking the user
  whether the CLI needs new flags. Rejected because the actual `main.rs` source was available to
  read directly — guessing would have been strictly worse than checking.

## 2. What is the real shape of the QMC Decorations Database (FR-010, Key Entity)?

- **Decision**: Three sheet tabs — `"Ribbons Database"`, `"Badges Database"`, `"Foreign Awards
  Database"` — map 1:1 to the spec's three award categories (Ribbons, Badges, Foreign Devices).
- **Rationale**: `awards-core::meta::SHEET_NAMES` and `CATEGORY_LABELS` define exactly these
  three tabs/categories, each with its own header row and data-start row (`meta_map()`). Grouping
  in `print_awards`/`group_awards` already iterates `CATEGORY_LABELS` to produce the grouped
  display FR-010 requires.
- **Alternatives considered**: None — this is a factual property of the existing spreadsheet
  schema, not a design choice.

## 3. How does "stay signed in" (FR-003) actually work?

- **Decision**: Keep the existing OAuth-token-with-refresh model as the FR-003 implementation;
  no session/expiry UI changes are needed.
- **Rationale**: `awards-sheets::auth` persists `token.json` (`AuthorizedUser`, incl.
  `refresh_token`/`expiry`) and `get_access_token` transparently calls `refresh_authorized_user`
  whenever the cached token is expired but a refresh token exists — a clerk is only sent through
  the interactive browser flow (`interactive_login`) when there is no usable token at all
  (matching Acceptance Scenario 4 in User Story 1).
- **Alternatives considered**: A shorter-lived, always-interactive session. Rejected — it
  contradicts FR-003's explicit "not required to sign in again" requirement and the existing,
  already-tested refresh path already satisfies it.

## 4. How is a permission-denied write surfaced to the clerk (FR-004)?

- **Decision**: Keep passing Google's raw `HTTP {status}: {body}` text through `EditResult.message`
  rather than adding a dedicated "you don't have permission" translation layer.
- **Rationale**: `awards-sheets::api::ApiError::Http { status, body }` already carries the full
  Google API error (a 403 response body names `PERMISSION_DENIED` and the caller/account), and
  every `edit.rs` mutator (`add_award_to_user`, `update_award_cell`, `remove_award`,
  `rename_username`) surfaces `Err(e)` as `format!("... failed: {e}")` back to the CLI/TUI, which
  print or display it. This technically satisfies FR-004 ("the clerk is told they don't have
  permission") today, even though the message is Google's raw JSON rather than hand-written copy.
- **Alternatives considered**: Adding a `403 → "You don't have edit access to this sheet"`
  translation in `ApiError`. Rejected for this feature: FR-004 only requires the clerk be told,
  not a specific wording, and inventing new copy here would be scope creep beyond what the user's
  feature description asked for. Flagged as a possible future UX polish, not a requirement.

## 5. How does the existing stale-write guard satisfy FR-009 / SC-004?

- **Decision**: Reuse the existing `find_live_row` / `live_cell_value` / `cell_stale_message`
  pattern unchanged as the FR-009 implementation.
- **Rationale**: Every mutator in `edit.rs` re-reads the target cell from the live sheet
  immediately before writing and compares it against the value it expected (`award.cell`); on a
  mismatch it returns an error naming both the old and the new live value and instructs the clerk
  to refresh and retry, rather than overwriting. `rename_username` goes further and re-derives the
  live row per column before each batch write, so a second clerk's concurrent edit is detected
  per-cell, not just per-username.
- **Alternatives considered**: Sheet-level optimistic-concurrency versioning (e.g. an ETag or
  revision column). Rejected — heavier than needed for a single shared spreadsheet at this scale,
  and the existing per-cell re-check already meets SC-004's 100%-rejection requirement.

## 6. How does a rename cascade across every award entry in one action (FR-008, SC-005)?

- **Decision**: Reuse `rename_username` unchanged.
- **Rationale**: It already groups every award the old username owns by `(sheet, column)`, does
  one live re-read per column, rewrites every matching cell (preserving suffixes via
  `replace_username_in_cell`), and reports partial-failure state (`err_partial`) with exactly
  which ranges still need a retry — satisfying both FR-008 (single clerk action) and the "zero
  entries left referencing the old username" bar in SC-005 once a retry (if needed) completes.
- **Alternatives considered**: A multi-step wizard requiring the clerk to confirm each cell
  individually. Rejected — contradicts FR-008's "single action" requirement and the existing
  batched implementation already handles collision detection (`column_foreign_username`) and
  partial-failure resumption.

## 7. Personal OAuth vs. a shared service account for "signs in with their own Google account"

- **Decision**: Both auth modes already coexist and both satisfy the spec; no change needed.
  `spec.md`'s Assumptions section already documents that a clerk's write access "comes entirely
  from whatever access their own Google account already has" — the service-account path is an
  alternate deployment option (e.g. a shared bot identity USAR leadership could grant edit access
  to) rather than a replacement for personal sign-in, and `auth_status()`/`login()` already prefer
  a service account when one is present, falling back to personal OAuth otherwise.
- **Rationale**: `awards-sheets::auth::get_access_token` checks `service_account_path()` first,
  then falls back to the OAuth `token.json` flow — this is existing, tested behavior
  (`awards_root_env_wins`, `extract_code_from_request_line`, etc.).
- **Alternatives considered**: Forcing personal-OAuth-only. Rejected — would remove an existing,
  working capability without the spec asking for it.
