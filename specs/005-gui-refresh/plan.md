# Implementation Plan: GUI Refresh

**Branch**: `005-gui-refresh` | **Date**: 2026-09-10 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/005-gui-refresh/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command; its definition describes the execution workflow.

## Summary

Add a Refresh control to `awards-gui` that re-triggers the exact same sync path the application
already runs once at startup (`GuiApp::start_sync`, added by `004-gui-lookup-add`). No new state
type, no new message variant, and no new dependency: `start_sync()` already sets `syncing = true`
before spawning the background fetch, and every place that already guards on `syncing` (the Look
Up button, the Add button) already goes inert the same way a startup sync would. The only new
code is a button in `ui.rs` that calls `start_sync()` when `!app.syncing`, and a one-line guard
inside `start_sync()` itself so a second refresh can never be started while one is already running
even if the button were somehow clicked twice in the same frame.

## Technical Context

**Language/Version**: Rust, 2021 edition — unchanged; no new crate, no workspace member added.

**Primary Dependencies**: None added. This feature is implemented entirely with functions and
state `004-gui-lookup-add` already shipped (`GuiApp::start_sync`, `GuiApp.syncing`,
`GuiApp.sync_error`).

**Storage**: Unchanged — same Decorations Database Google Sheet, same `build_awards_data(None)`
call, same public-CSV fetch with no auth required (spec FR-007).

**Testing**: One new unit test in `crates/awards-gui/src/app.rs`'s existing test module: calling
`start_sync()` while `syncing == true` MUST NOT spawn a second background thread (asserted via the
existing `counting_repaint` test helper — a guarded call must not invoke the repaint callback a
second time). No new UI-only behavior needs a test beyond what `ui.rs` already leaves untested by
convention (plan.md of `004-gui-lookup-add`, Testing section).

**Target Platform**: Unchanged — same native desktop binary.

**Project Type**: Unchanged — no new crate, no new file beyond edits to the existing
`crates/awards-gui/src/app.rs` and `crates/awards-gui/src/ui.rs`.

**Performance Goals**: Identical to the existing startup sync (`004-gui-lookup-add` plan.md) —
`build_awards_data`'s existing concurrent per-tab fetch, unchanged.

**Constraints**: `start_sync()` MUST be a no-op while `syncing == true` (spec FR-004) — the guard
lives in `start_sync()` itself, not only in the button's enabled state, so the invariant holds
regardless of how `start_sync()` is ever called from in the future.

**Scale/Scope**: One button, one guard clause. The smallest possible next increment after
`004-gui-lookup-add`, by design (see that feature's spec Assumptions: "first of several planned
GUI milestones").

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Gate | Status | Evidence |
|---|---|---|---|
| I. Code Quality | Workspace boundaries preserved; no `unsafe`; no speculative deps; typed errors at library boundaries | **PASS** | No new dependency, no new crate. `start_sync()` and `AwardsData`/`SheetsError` are all reused unchanged from `awards-sheets`/`awards-core` exactly as `004-gui-lookup-add` already established. |
| II. Testing Standards | New/changed modal transitions tested; network code offline-testable; CI runs `--locked` test+clippy | **PASS** | The one behavior change (`start_sync()` refusing to start a second sync while one is running) gets a direct unit test with no network access, following `004`'s established pattern of testing `app.rs` state transitions headlessly. |
| III. UX Consistency | Esc-cancel convention; typed confirm phrase for destructive writes; CLI/TUI parity | **PASS** | Refresh is a non-destructive read, so no confirmation phrase applies (matches the terminal tool's own unconfirmed F5 Refresh). CLI/TUI parity: the terminal tool already has Refresh (F5); this feature closes that specific gap for the GUI rather than opening one. |
| IV. Performance | Concurrent independent fetches; explicit HTTP timeouts; non-blocking writes | **PASS** | Refresh reuses `build_awards_data`'s existing concurrent fetch and `fetch_sheet`'s existing 60s timeout, unchanged. It runs on a background thread exactly as the startup sync already does — the UI is never blocked. |
| Security & Data Integrity | Secrets gitignored; live stale-write re-check; OAuth `state` CSRF check | **PASS / N/A** | Refresh performs no write and touches no credential file — it is the unauthenticated public-CSV read `004-gui-lookup-add`'s research.md §6 already documented as requiring no sign-in. |

No violations requiring justification. Complexity Tracking table is omitted (N/A).

## Project Structure

### Documentation (this feature)

```text
specs/005-gui-refresh/
├── plan.md               # This file (/speckit-plan command output)
├── research.md           # Phase 0 output (/speckit-plan command)
├── data-model.md         # Phase 1 output (/speckit-plan command)
├── quickstart.md         # Phase 1 output (/speckit-plan command)
├── contracts/            # Phase 1 output (/speckit-plan command)
│   └── gui-refresh-interaction.md
└── tasks.md               # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
crates/awards-gui/src/
├── app.rs   # EDIT: add a guard to start_sync() so it's a no-op while syncing == true
└── ui.rs    # EDIT: add a Refresh button next to the username field, calling start_sync()
```

**Structure Decision**: No new files, no new crate, no workspace change — this feature is two
small edits to files `004-gui-lookup-add` already created.

## Complexity Tracking

*No entries — Constitution Check reported no violations requiring justification.*
