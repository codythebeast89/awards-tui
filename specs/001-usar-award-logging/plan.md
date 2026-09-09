# Implementation Plan: USAR Award Logging

**Branch**: `001-usar-award-logging` | **Date**: 2026-09-09 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/001-usar-award-logging/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command; its definition describes the execution workflow.

## Summary

Let any USAR member look up a person's logged Ribbons, Badges, and Foreign Devices with no
sign-in, while a Logistics Clerk who signs in with their own Google account (or an equivalent
shared service account already granted edit access on the QMC Decorations Database spreadsheet)
can add, correct, remove, or rename award entries. This is substantially an as-built feature:
the `awards-tui` workspace already implements every acceptance scenario in `spec.md` — public
CSV-based lookup, OAuth desktop sign-in with token refresh, per-write live-cell staleness
re-checks, and cascading rename — across its three crates. This plan documents that existing
architecture against the spec's requirements, confirms it satisfies every FR and constitution
gate, and calls out the two small pre-existing gaps (see Constitution Check notes) that are
tracked but out of this feature's scope.

## Technical Context

**Language/Version**: Rust, 2021 edition, workspace `version = "2.3.0"` (`Cargo.toml`); toolchain
matches whatever `.github/workflows/rust.yml` / `release.yml` pin for CI and release builds.

**Primary Dependencies**: `ratatui` 0.30 + `crossterm` 0.29 (TUI rendering/input), `clap` 4
(derive-based CLI), `reqwest` 0.12 (blocking, `rustls-tls`) for all Google Sheets REST calls,
`jsonwebtoken` 10.3 (RS256) for service-account JWT signing, `csv` 1 for the public read-only
export, `chrono` 0.4, `tui-input` 0.15, `thiserror` 2 (typed errors at the `awards-core` /
`awards-sheets` boundary), `anyhow` 1 (binary-level error handling in `awards-tui` only),
`strsim` 0.11 (duplicate/similarity scan — out of this feature's scope but shares the crate),
`serde`/`serde_json`, `toml` 0.8 (`awards-tui.toml` config), `dirs` 6, `open` 5 (launches the
OAuth consent URL in a browser), `getrandom` 0.2 (OAuth `state` nonce), `urlencoding` 2, `regex`
1.

**Storage**: No local database. The QMC Decorations Database Google Sheet (`SHEET_ID` constant
in `awards-core::meta`) is the sole system of record for award entries. Local disk holds only
OAuth/service-account secrets (`token.json`, `credentials.json` or `service_account.json`,
written mode `0600` on Unix) and optional local config (`awards-tui.toml`, `award_columns.json`)
— no award data is cached or persisted locally between runs.

**Testing**: `cargo test --workspace --locked` — unit tests already in `crates/awards-core/tests/offline.rs`
and inline `#[cfg(test)]` modules per source file; TUI modal-transition tests in
`crates/awards-tui/src/tui/app.rs` (~29 tests already covering the Add/Edit/Delete/Rename/Assist/
Audit modals this spec's acceptance scenarios exercise). `cargo clippy --workspace --all-targets
--locked -- -D warnings`. Tests that need live credentials or a live network call are marked
`#[ignore]` (e.g. `edit.rs::live_add_then_delete_roundtrip`) and excluded from the default run.

**Target Platform**: Cross-platform terminal application (Linux/macOS/Windows), distributed as
prebuilt binaries via `.github/workflows/release.yml` and installable with `cargo binstall`.

**Project Type**: Single-workspace CLI + TUI desktop tool (3 Cargo crates) — not a web or mobile
app; no frontend/backend split.

**Performance Goals**: Match the constitution's Performance Requirements principle, already
implemented: the three sheet tabs are fetched concurrently rather than sequentially
(`build_awards_data`); every `reqwest` client is built with an explicit 60s timeout
(`SheetsApi::connect`, `http_form_post`); duplicate/similarity scanning (outside this feature's
scope) stays sub-quadratic via prefix bucketing; sheet-mutating writes run off the TUI's render
thread and report back through the `WorkerMsg` channel so the UI never blocks on a network call.

**Constraints**: The lookup path (User Story 2) needs network access to the sheet's public CSV
export but no credentials. The write path (User Stories 1 and 3) needs network access plus a
valid bearer token — either a clerk's own OAuth user token (auto-refreshed via
`refresh_authorized_user`) or a shared service-account JWT. Every mutating write MUST re-read the
live cell immediately beforehand and reject on mismatch (`find_live_row` / `live_cell_value` /
`cell_stale_message` in `awards-sheets::edit`) to satisfy FR-009. Secrets must stay gitignored and
mode-600.

**Scale/Scope**: One shared spreadsheet (3 tabs: Ribbons Database, Badges Database, Foreign
Awards Database) used by a small clerk corps, with unauthenticated read access open to the whole
USAR membership. No throughput targets beyond "feels instant" for a spreadsheet of this size —
see Success Criteria in `spec.md`.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Gate | Status | Evidence |
|---|---|---|---|
| I. Code Quality | Workspace boundaries preserved; no `unsafe`; no speculative deps; typed errors at library boundaries | **PASS** | Feature needs no new crate and crosses no boundary: `awards-core` stays I/O-free (`audit.rs`, `eligibility.rs`, `format.rs`, `index.rs`, `meta.rs`, `parse.rs`, `types.rs`), all Google I/O stays in `awards-sheets` (`api.rs`, `auth.rs`, `edit.rs`), all presentation stays in `awards-tui`. `awards-sheets::api::ApiError` / `auth::AuthError` are `thiserror` enums; `anyhow` is confined to `main.rs`/TUI. |
| II. Testing Standards | New/changed modal transitions tested; network code offline-testable; CI runs `--locked` test+clippy | **PASS** (with a pre-existing, out-of-scope gap noted below) | Every modal this spec references (Add/Edit/Delete/Rename) already has transition tests in `app.rs`. Network-touching code (`live_add_then_delete_roundtrip`) is `#[ignore]`d. |
| III. UX Consistency | Esc-cancel convention; typed confirm phrase for destructive writes; CLI/TUI parity; theme-only colors | **PASS** | Confirmed by direct inspection of `main.rs`'s `Cli` struct: `--add`, `--edit`/`--cell`, `--delete`, `--rename`, `--login`, `--auth-status` give the CLI 1:1 coverage of the TUI's Add/Edit/Delete/Rename modals and the sign-in flow — this spec introduces no interface gap. Delete/Rename already require a typed confirmation phrase in the TUI (tested in `app.rs`). |
| IV. Performance | Concurrent independent fetches; explicit HTTP timeouts; sub-quadratic scans; non-blocking writes | **PASS** | `build_awards_data` fetches the three tabs concurrently; `SheetsApi::connect` sets a 60s `reqwest` timeout; writes already run through the TUI's background-thread `WorkerMsg` pattern. |
| Security & Data Integrity | Secrets gitignored + mode 600; live stale-write re-check before every mutation; OAuth `state` CSRF check | **PASS** | `save_authorized_user` writes `token.json` at mode `0600` (Unix); `interactive_login` rejects a callback whose `state` doesn't match (`OAuth callback state mismatch (possible CSRF)`); `edit.rs`'s `add_award_to_user` / `update_award_cell` / `remove_award` / `rename_username` all re-read the live cell immediately before writing and return `cell_stale_message(...)` on mismatch — this is FR-009 already implemented. |

**Notes (not gate failures, not introduced by this feature, tracked separately)**:
1. `.github/workflows/rust.yml`'s `test` job is missing `--locked` on its `cargo test` /
   `cargo clippy` steps (present in `release.yml`); a diff was already prepared earlier and is
   pending manual application by the repo owner, since this file is write-protected from remote
   tooling. Not a blocker for this feature.
2. FR-004 ("tell the clerk the attempt was rejected") is satisfied today by passing Google's raw
   `HTTP {status}: {body}` error text up through `EditResult.message` (e.g. a 403 response body
   naming `PERMISSION_DENIED`) rather than a purpose-written friendly sentence. The clerk *is*
   told and shown the reason, so this meets the letter of FR-004; see `research.md` for the
   decision to leave this as-is rather than add a translation layer.

No violations requiring justification. Complexity Tracking table is omitted (N/A).

## Project Structure

### Documentation (this feature)

```text
specs/001-usar-award-logging/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (/speckit-plan command)
├── data-model.md         # Phase 1 output (/speckit-plan command)
├── quickstart.md        # Phase 1 output (/speckit-plan command)
├── contracts/           # Phase 1 output (/speckit-plan command)
│   └── cli-interface.md
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
crates/
├── awards-core/                 # pure domain logic — no network or filesystem I/O
│   ├── src/
│   │   ├── audit.rs             # duplicate/similarity scan (out of this feature's scope)
│   │   ├── eligibility.rs       # clerk-assist eligibility rules (out of this feature's scope)
│   │   ├── format.rs            # award display-name formatting
│   │   ├── index.rs             # username -> Award[] index (FR-011)
│   │   ├── lib.rs
│   │   ├── meta.rs              # SHEET_ID, SHEET_NAMES, CATEGORY_LABELS, sheet row layout (FR-010)
│   │   ├── parse.rs             # cell <-> username/suffix parsing
│   │   └── types.rs             # Award, AwardDef, AwardsData
│   └── tests/offline.rs
├── awards-sheets/                # Google Sheets REST + OAuth / service-account I/O
│   └── src/
│       ├── api.rs                # SheetsApi (get/update/batch-update values)
│       ├── auth.rs               # OAuth desktop flow, token refresh, service-account JWT (US1)
│       ├── edit.rs                # add/edit/delete/rename + live stale-write re-check (US1, US3, FR-009)
│       └── lib.rs
└── awards-tui/                    # binary crate: Clap CLI + Ratatui TUI presentation
    └── src/
        ├── main.rs                # Cli struct: --add/--edit/--delete/--rename/--login (US1-3)
        ├── config.rs
        └── tui/
            ├── mod.rs
            ├── app.rs             # Modal state machine (Add/Edit/Delete/Rename/Assist/Audit)
            └── ui.rs
```

**Structure Decision**: The existing 3-crate workspace layout is reused unchanged. This feature
is realized as already-implemented behavior spanning all three crates (`awards-core` for pure
lookup/format/index logic, `awards-sheets` for the Google-authenticated read/write path,
`awards-tui` for the CLI and TUI surfaces) — no new crate, module, or top-level directory is
required.

## Complexity Tracking

*No entries — Constitution Check reported no violations.*
