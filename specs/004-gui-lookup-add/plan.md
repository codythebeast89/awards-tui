# Implementation Plan: GUI Lookup & Add

**Branch**: `004-gui-lookup-add` | **Date**: 2026-09-10 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/004-gui-lookup-add/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command; its definition describes the execution workflow.

## Summary

Add a new `awards-gui` binary crate — a native desktop application built with `egui`/`eframe` —
alongside the existing `awards-tui` binary in the same Cargo workspace. It offers exactly two
capabilities in this first milestone: look up a Roblox username and view their current awards
across all three categories, and add an award to that user from a searchable catalog list,
confirmed as an explicit second step. Every byte of business logic and I/O is reused unchanged
from `awards-core` (parsing/matching/index-building) and `awards-sheets` (public CSV fetch, OAuth
sign-in, and the guarded `add_award_to_user` write path with its built-in stale-cell re-check) —
`awards-gui` is presentation-only, mirroring `awards-tui`'s existing role, just rendered with a
window instead of a terminal. Network work (sheet sync, sign-in, the add write) runs on background
threads and reports back through an mpsc channel exactly as `awards-tui`'s `WorkerMsg` pattern
already does, so the UI thread is never blocked.

## Technical Context

**Language/Version**: Rust, 2021 edition, workspace `version = "2.3.0"` (`Cargo.toml`) — unchanged;
`awards-gui` is a new workspace member, not a version bump.

**Primary Dependencies**: `eframe` (pinned `0.36`, egui's application/windowing framework — pulls
in `egui` itself) is the only new dependency, added to `awards-gui`'s own `Cargo.toml` only (per
the constitution's Code Quality principle: a dependency MUST be used only by the crate that needs
it). `awards-gui` otherwise depends on the existing `awards-core` and `awards-sheets` workspace
crates exactly as `awards-tui` does, plus `anyhow` (workspace dependency, already used by
`awards-tui`'s own binary-crate error handling) for its own top-level error handling.

**Storage**: Unchanged — the QMC Decorations Database Google Sheet remains the sole system of
record; no new local storage. Session credential state (`token.json`, `service_account.json`,
`credentials.json`) is read from the same existing locations (`AWARDS_ROOT` / cwd /
`~/.config/awards-tui/`) via the same unchanged `awards_sheets::auth` functions.

**Testing**: `cargo test --workspace --locked` and `cargo clippy --workspace --locked -- -D
warnings` (and `--all-targets`), same as every prior feature. `awards-gui`'s own state/transition
logic lives in a plain, windowing-free module (mirroring `awards-tui/src/tui/app.rs`'s split from
`ui.rs`) so it is unit-testable without a display, the same way the TUI's `app.rs` is tested
without a real terminal. The `eframe::App::update` rendering function itself, like `awards-tui`'s
`ui.rs` render functions, is thin and not directly unit-tested — this environment cannot open a
window at all (no X11/Wayland), so a live visual run is out of reach here exactly as an
interactive TUI run already was for `003-discord-paste-quick-add` (see that feature's `T028`).

**Target Platform**: Cross-platform native desktop application (Linux/macOS/Windows), matching
`awards-tui`'s existing supported platforms. `eframe`'s default `glow` (OpenGL) backend is used
rather than opting into `wgpu`, for the widest driver compatibility on clerks' existing machines
without requiring Vulkan/DirectX 12 support.

**Project Type**: Desktop application — a new binary crate in the existing single-workspace
layout (was CLI+TUI only; now CLI+TUI+GUI, all three binary crates sharing the same two library
crates).

**Performance Goals**: Sheet sync reuses `build_awards_data`'s existing concurrent per-tab fetch
(three `thread::spawn` calls, unchanged) — no regression versus the TUI's sync time. The write
that finishes an add reuses `add_award_to_user` unchanged, including its live-cell re-check
immediately before writing. Neither adds a new network round trip.

**Constraints**: The add write MUST go through the exact same `add_award_to_user` call the TUI's
Add flow already uses, with `interactive_auth: false` (matching the TUI's own call site) — the add
action itself never silently pops an OAuth browser window; sign-in is a separate, explicit action
(see research.md §4) so the clerk is never surprised by an unprompted browser tab mid-add.

**Scale/Scope**: Same one shared spreadsheet, same clerk corps, same award catalog size as the
existing TUI. This milestone is deliberately two actions (Lookup, Add) out of the TUI's eight —
Edit, Delete, Rename, Assist, Audit, and Paste are out of scope (spec FR-010) and land in later
features once this foundation ships.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Gate | Status | Evidence |
|---|---|---|---|
| I. Code Quality | Workspace boundaries preserved; no `unsafe`; no speculative deps; typed errors at library boundaries | **PASS** | `awards-gui` is presentation-only, exactly mirroring `awards-tui`'s existing role — it calls `awards_core`/`awards_sheets` functions directly and contains no parsing, matching, eligibility, or Sheets-API logic of its own (research.md §2, §5, §6). `eframe`/`egui` are declared only in `crates/awards-gui/Cargo.toml`, not the workspace-shared dependency list, since no other crate needs them. No `unsafe` is introduced. Errors surfaced to the clerk are the existing typed `EditError`/`AuthError`/`SheetsError` from `awards-sheets`/`awards-core`, not new stringly-typed errors; `anyhow` is used only for `awards-gui`'s own top-level binary error handling, matching `awards-tui`'s existing precedent. |
| II. Testing Standards | New/changed modal transitions tested; network code offline-testable; CI runs `--locked` test+clippy | **PASS** | `awards-gui`'s state module (research.md §7) is a plain Rust struct with unit tests covering every state transition (idle → looking-up → user-in-view, catalog-selection → confirming → add-written, sign-in-required → signing-in → signed-in), following the exact pattern `awards-tui/src/tui/app.rs`'s existing test suite already establishes. No new network-touching code is introduced — sync/add/login all reuse existing, already-`#[ignore]`-gated-where-needed functions verbatim. |
| III. UX Consistency | Esc-cancel convention; typed confirm phrase for destructive writes; CLI/TUI parity; theme-only colors | **PASS**, with one documented, justified gap | The literal "Esc cancels, sets 'Dialog cancelled'" convention is a terminal-specific mechanic that doesn't map onto a windowed GUI; this feature satisfies the *principle behind it* instead: selecting an award and confirming it are two explicit, separate steps (spec FR-004) — no single click both selects and writes — matching the TUI's own Pick-then-Suffix-then-confirm shape for Add. This feature introduces no destructive (delete/rename-class) write at all, so the typed-confirmation-phrase rule doesn't apply here. **Constitution scope gap, not a feature violation**: the constitution's CLI/TUI parity language predates a third front-end existing at all. This feature adds no new capability beyond what the CLI/TUI already jointly provide (Lookup, Add) — it is a new *way to reach* existing capability, not a capability gap — so no CLI change is needed; whether the constitution's parity language should be widened to explicitly name the GUI is a governance question for a future `/speckit-constitution` pass, not something this plan resolves unilaterally. Theme/color values are not yet defined for a GUI at all (`Theme`/`awards-tui.toml` are TUI-specific); `awards-gui` uses `egui`'s own default visual style for this milestone rather than inventing a new, undocumented color source — a design-system pass is left to a later feature once more GUI surface exists to make one meaningful. |
| IV. Performance | Concurrent independent fetches; explicit HTTP timeouts; sub-quadratic scans; non-blocking writes | **PASS** | Reuses `build_awards_data`'s existing concurrent per-tab fetch and `fetch_sheet`'s existing 60s `reqwest` timeout unchanged (research.md §6). Award-catalog filtering reuses `match_catalog_entries` (already `awards-core`, added by `003-discord-paste-quick-add`) — same sub-quadratic linear scan the TUI's own Add picker already uses. The add write and sign-in both run on background threads reporting through an `mpsc` channel, with the worker thread calling `egui::Context::request_repaint()` after sending so the UI updates promptly without a TUI-style fixed poll loop (research.md §3) — the render thread is never blocked waiting on either. |
| Security & Data Integrity | Secrets gitignored + mode 600; live stale-write re-check before every mutation; OAuth `state` CSRF check | **PASS** | No new secret-bearing file type is introduced; credential resolution and file permissions are entirely unchanged (`awards_sheets::auth`, untouched). `add_award_to_user`'s existing live-cell re-check immediately before write is reused verbatim — this feature adds no new write path that could weaken or duplicate it. The sign-in button (research.md §4) calls the existing `login()` function verbatim, including its existing OAuth `state` CSRF validation inside `interactive_login` — nothing about the OAuth flow itself changes. |

No violations requiring justification beyond the documented, non-blocking Constitution-scope
observation above. Complexity Tracking table is omitted (N/A).

## Project Structure

### Documentation (this feature)

```text
specs/004-gui-lookup-add/
├── plan.md               # This file (/speckit-plan command output)
├── research.md           # Phase 0 output (/speckit-plan command)
├── data-model.md         # Phase 1 output (/speckit-plan command)
├── quickstart.md         # Phase 1 output (/speckit-plan command)
├── contracts/            # Phase 1 output (/speckit-plan command)
│   └── gui-lookup-add-interaction.md
└── tasks.md               # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
Cargo.toml                      # add "crates/awards-gui" to workspace members

crates/
├── awards-core/                # unchanged by this feature — pure domain logic, no I/O
├── awards-sheets/               # unchanged by this feature — Sheets REST + OAuth I/O
├── awards-tui/                  # unchanged by this feature — existing Clap CLI + Ratatui TUI
└── awards-gui/                  # NEW binary crate — presentation-only, mirrors awards-tui's role
    ├── Cargo.toml                # depends on awards-core, awards-sheets, eframe, anyhow (own deps only)
    └── src/
        ├── main.rs                # eframe::run_native entry point; not unit-tested (needs a display)
        ├── app.rs                 # NEW: GuiApp state + WorkerMsg-equivalent channel/thread plumbing;
        │                          # state-transition unit tests live here, mirroring awards-tui/app.rs
        └── ui.rs                  # NEW: egui widget rendering, reading/driving GuiApp state; thin,
                                    # not directly unit-tested (mirrors awards-tui/ui.rs)
```

**Structure Decision**: A fourth workspace member (`crates/awards-gui`) is added, following the
exact same pure-logic / I/O / presentation split the existing three crates already establish —
`awards-gui` takes on the same role `awards-tui` already occupies (presentation only), just for a
second, windowed front-end. No existing crate's structure changes; `awards-core` and
`awards-sheets` are consumed as-is.

## Complexity Tracking

*No entries — Constitution Check reported no violations requiring justification.*
