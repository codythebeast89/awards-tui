# Implementation Plan: Discord Paste Quick-Add

**Branch**: `003-discord-paste-quick-add` | **Date**: 2026-09-10 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/003-discord-paste-quick-add/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command; its definition describes the execution workflow.

## Summary

Let a Logistics Clerk paste the raw text of a manually-forwarded Discord badge/ribbon request
message straight into the TUI, extract the requester's username and the requested award from it,
and land directly in the existing Add flow with both pre-filled — instead of retyping the username
into Lookup and hand-picking the award from the full catalog list. No Discord account, bot, or API
access of any kind is introduced or required (that path was ruled out earlier: no admin access to
invite a bot, and automating a personal account would violate Discord's ToS); the clerk still reads
the Discord channel and copies the message themselves. Proof verification stays entirely manual —
this feature never fetches, opens, or evaluates whatever the pasted text names as proof. Every
write still goes through the exact same OAuth-gated `add_award_to_user` path the existing Add flow
already uses; this feature only changes how a clerk gets to that flow's pre-filled starting point.

## Technical Context

**Language/Version**: Rust, 2021 edition, workspace `version = "2.3.0"` (`Cargo.toml`) — unchanged
by this feature; no new crate, no MSRV change.

**Primary Dependencies**: No new dependencies. Reuses `crossterm`'s existing bracketed-paste
support (`EnableBracketedPaste`/`DisableBracketedPaste`, the `Event::Paste(String)` variant — the
crate is already a workspace dependency; this feature is simply the first to enable and handle
that mode) so a multi-line Discord message pastes into the TUI as one atomic string instead of a
burst of individual keystrokes. Reuses the existing `regex` dependency (already used throughout
`awards-core::parse`) for the labeled-line extraction, the existing `parse_bare_username` validator,
and the existing `AddModal`/`AddStep::{Pick,Suffix}` machinery and `add_award_to_user` write path
verbatim (research.md §2, §5, §6).

**Storage**: Unchanged — the QMC Decorations Database Google Sheet remains the sole system of
record. Pasted request text is never written to disk or to the sheet in its raw form (spec Key
Entities: "Pasted Request Text" is transient) — it exists only in memory for the duration of one
paste-to-prefill action.

**Testing**: `cargo test --workspace --locked` and `cargo clippy --workspace --locked -- -D
warnings`, same as every prior feature in this repo. New pure parsing/matching logic
(`extract_paste_fields`, `split_award_suffix`, `match_catalog_entries`) lands in `awards-core` with
unit tests per the constitution's Testing Standards principle, exercised against the two real
Discord message samples gathered for this feature (see `quickstart.md`) plus edge cases from the
spec (missing username, unmatched award text, unparseable paste). New TUI modal-transition tests
(paste → prefilled Add, paste → Pick-with-filter fallback, paste → error status) follow the
existing suite's pattern in `crates/awards-tui/src/tui/app.rs`. No new `#[ignore]`d live-network
test is needed — this feature adds no new network call.

**Target Platform**: Unchanged — cross-platform terminal application (Linux/macOS/Windows).
Bracketed-paste support is a terminal-emulator feature, not a network or platform dependency; a
terminal that doesn't support it still lets the clerk fall back to the existing manual Lookup + Add
flow, so this is a graceful-degradation gap, not a blocker (research.md §1).

**Project Type**: Unchanged — single-workspace CLI + TUI desktop tool (3 Cargo crates).

**Performance Goals**: Adds no network call. Username lookup reuses the existing, already-local
`apply_user_view` (operates on the in-memory `AwardsData` already loaded at startup, the same path
`action_lookup` uses today — no fetch). Award-catalog matching is a linear scan over the same
bounded catalog `AddModal`'s existing Pick-step filter already scans, so no new complexity class is
introduced (constitution Performance principle).

**Constraints**: The write that finishes a paste-prefilled Add MUST go through the exact same
`add_award_to_user` call and `AddStep::Suffix` confirmation step the manual Add flow already uses
(FR-009 equivalent — spec FR states the same OAuth-gated path) — this feature introduces no new
write function and no new stale-check logic.

**Scale/Scope**: Same one shared spreadsheet, same clerk corps as prior features. One paste is one
Discord message (a few short labeled lines); no batching or multi-message handling is in scope.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Gate | Status | Evidence |
|---|---|---|---|
| I. Code Quality | Workspace boundaries preserved; no `unsafe`; no speculative deps; typed errors at library boundaries | **PASS** | New parsing/matching functions (`extract_paste_fields`, `split_award_suffix`, `match_catalog_entries`) live in `awards-core::parse` (pure, no I/O) per `data-model.md`. `AddModal::reload`'s existing inline filter predicate in `awards-tui` is refactored to call the new shared `match_catalog_entries` rather than duplicating the substring-match rule in two places (research.md §5) — this keeps business logic in `awards-core`, not presentation. The new `PasteAddModal` and `Action::PasteAdd` are presentation-only, in `awards-tui`. No new dependency added — `crossterm`'s bracketed-paste API is already part of the existing `crossterm` dependency, just newly enabled. |
| II. Testing Standards | New/changed modal transitions tested; network code offline-testable; CI runs `--locked` test+clippy | **PASS** | `extract_paste_fields`, `split_award_suffix`, and `match_catalog_entries` are pure and ship with unit tests covering both real Discord message samples plus the spec's edge cases (unmatched award text, missing username, fully unparseable paste, trailing `x1`/`x2` suffix). New Add-modal-entry transitions (paste → prefilled `Suffix` step, paste → filtered `Pick` step, paste → error status) get transition tests following the existing `app.rs` suite pattern. No new network-touching code is introduced, so no new `#[ignore]`d test is needed. |
| III. UX Consistency | Esc-cancel convention; typed confirm phrase for destructive writes; CLI/TUI parity; theme-only colors | **PASS**, with one documented, justified gap | `Esc` from the new `PasteAddModal` follows the existing default convention unchanged — closes the modal and sets status "Dialog cancelled" — no new exception is needed (unlike `002-audit-fix-selection`'s nested-modal case). When a paste resolves into `Modal::Add`, that modal's own existing Esc/Enter/typed-confirmation behavior is completely unchanged; this feature only changes how the modal is constructed (pre-filled `chosen`/`suffix`/`filter`), never how it's driven once open. **CLI/TUI parity gap**: the CLI gains no new flag for this feature — pasting a multi-line Discord message via bracketed paste is an inherently interactive terminal operation with no equivalent in a scripted one-shot command; the clerk already has `--add <AWARD> --suffix <SUFFIX>` (once they've read the username and award off the message themselves) for scripted use. This is the explicit call-out the principle requires when parity isn't practical. New modal reuses the existing typed-input/status-line presentation pattern already used throughout the app, not a new UI pattern. |
| IV. Performance | Concurrent independent fetches; explicit HTTP timeouts; sub-quadratic scans; non-blocking writes | **PASS** | No new network calls are introduced by this feature at all. Username lookup and award-catalog matching are both local, in-memory operations over already-loaded data (research.md §5, §7), strictly no slower than the existing manual Lookup + Add path. The write that finishes a paste-prefilled Add still runs through the existing background-thread `WorkerMsg` path — the render loop is never blocked. |
| Security & Data Integrity | Secrets gitignored + mode 600; live stale-write re-check before every mutation; OAuth `state` CSRF check | **PASS** | No new secret handling. A paste-prefilled Add still goes through `awards-sheets::edit`'s existing live-cell re-check unchanged before writing — this feature adds zero new write paths, so it can neither weaken nor duplicate that guard. Pasted text (which may include a Roblox ID or a proof link) is held only in the transient `PasteAddModal` buffer and is never logged, written to `audits/*.txt`, or persisted anywhere (spec FR-008, Key Entities). |

No violations requiring justification. Complexity Tracking table is omitted (N/A).

## Project Structure

### Documentation (this feature)

```text
specs/003-discord-paste-quick-add/
├── plan.md               # This file (/speckit-plan command output)
├── research.md           # Phase 0 output (/speckit-plan command)
├── data-model.md         # Phase 1 output (/speckit-plan command)
├── quickstart.md         # Phase 1 output (/speckit-plan command)
├── contracts/            # Phase 1 output (/speckit-plan command)
│   └── tui-paste-interaction.md
└── tasks.md              # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
crates/
├── awards-core/                 # pure domain logic — no network or filesystem I/O
│   └── src/
│       ├── parse.rs             # NEW: extract_paste_fields, split_award_suffix,
│       │                        # match_catalog_entries (existing parse.rs helpers unchanged)
│       └── ...                  # (all other modules unchanged by this feature)
└── awards-tui/                    # binary crate: Clap CLI + Ratatui TUI presentation
    └── src/
        └── tui/
            ├── mod.rs             # Enable/disable bracketed paste alongside the existing
            │                      # raw-mode/alternate-screen setup; handle new Event::Paste
            │                      # in the run loop and route it to an open PasteAddModal
            ├── app.rs             # NEW: Action::PasteAdd, Modal::PasteAdd(PasteAddModal),
            │                      # paste→prefilled-Add / paste→filtered-Pick / paste→error
            │                      # transitions; AddModal::reload refactored to call the new
            │                      # shared match_catalog_entries instead of its inline filter
            └── ui.rs              # NEW: render_paste_add_modal (paste buffer + status hint)
```

**Structure Decision**: The existing 3-crate workspace layout is reused unchanged — this feature
adds pure parsing/matching functions to `awards-core::parse` and their presentation-only TUI
counterpart (`PasteAddModal`, `Action::PasteAdd`) in `awards-tui`; no new crate, module boundary,
or top-level directory, and `awards-sheets`/`main.rs` (the CLI) are untouched per the CLI-parity
gap documented above.

## Complexity Tracking

*No entries — Constitution Check reported no violations.*
