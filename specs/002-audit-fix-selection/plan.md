# Implementation Plan: Audit Fix Selection

**Branch**: `002-audit-fix-selection` | **Date**: 2026-09-10 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/002-audit-fix-selection/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command; its definition describes the execution workflow.

## Summary

Turn the Internal Audit action's read-only text report into a navigable, selectable findings
list so a Logistics Clerk can pick a specific flagged issue (a duplicate award row, a malformed
cell, a look-alike-username pair) and land directly in the matching, already-existing fix action
(Delete, Edit, or Rename) pre-filled with that member and award — instead of reading the report
and manually re-navigating to Lookup/Edit elsewhere in the app. The existing plain-text report and
its `audits/*.txt` export remain available unchanged, and every fix still goes through the exact
same OAuth-gated, stale-write-checked write path the app already uses for Edit/Delete/Rename — this
feature adds a faster on-ramp to those actions, not a new way to write to the sheet.

## Technical Context

**Language/Version**: Rust, 2021 edition, workspace `version = "2.3.0"` (`Cargo.toml`) — unchanged
by this feature; no new crate, no MSRV change.

**Primary Dependencies**: No new dependencies. Reuses `ratatui`'s existing `ListState`/`List`
widgets (already used by `AddModal`'s candidate picker — same interaction pattern this feature's
findings list follows), the existing `thiserror`-based `EditError` (this session's earlier T021
work) for stale/permission failures surfaced from a fix, and the existing `EditResult` /
`WorkerMsg` background-write plumbing.

**Storage**: Unchanged — the QMC Decorations Database Google Sheet remains the sole system of
record; `audits/audit-<timestamp>.txt` remains the only new-ish file this feature touches, and
its format is explicitly unchanged (spec FR-005).

**Testing**: `cargo test --workspace --locked` and `cargo clippy --workspace --locked -- -D
warnings`, same as every prior feature in this repo. New pure logic (`flatten_audit_findings`,
`AuditFinding::to_award`) lands in `awards-core` with unit tests per the constitution's Testing
Standards principle; new/changed TUI modal-transition tests follow the existing suite's pattern in
`crates/awards-tui/src/tui/app.rs` (~29 tests already there for Add/Edit/Delete/Rename/Assist/
Audit). No new `#[ignore]`d live-network test is needed — this feature adds no new network call
(research.md §3).

**Target Platform**: Unchanged — cross-platform terminal application (Linux/macOS/Windows).

**Project Type**: Unchanged — single-workspace CLI + TUI desktop tool (3 Cargo crates).

**Performance Goals**: The findings-list refresh after a fix MUST NOT introduce a network round
trip — it re-runs the existing pure `collect_sheet_audit` against the TUI's already-locally-patched
in-memory `AwardsData` (research.md §3), consistent with the constitution's Performance principle
and with how the Awards pane already reflects a write instantly today.

**Constraints**: Every fix routed to from a selected finding MUST go through the existing
`add_award_to_user` / `update_award_cell` / `remove_award` / `rename_username` functions in
`awards-sheets::edit` unchanged (FR-008) — this feature introduces no new write function and no
new stale-check logic (research.md §2, §4).

**Scale/Scope**: Same one shared spreadsheet, same clerk corps as feature 001. Findings-list size
is bounded by however many issues the audit already finds — no new pagination concern beyond what
the existing Report view's scrolling already handles for a larger data set.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Gate | Status | Evidence |
|---|---|---|---|
| I. Code Quality | Workspace boundaries preserved; no `unsafe`; no speculative deps; typed errors at library boundaries | **PASS** | New `AuditFinding` type and `flatten_audit_findings`/`to_award` logic live in `awards-core::audit`/`types` (pure, no I/O) per `data-model.md`; the TUI-side `AuditFindingsList`/`FindingGroup` grouping is presentation-only in `awards-tui`. No changes to `awards-sheets`'s `EditError`/`ApiError`/`AuthError` typed-error boundary — fixes reuse it as-is. No new dependency added. |
| II. Testing Standards | New/changed modal transitions tested; network code offline-testable; CI runs `--locked` test+clippy | **PASS** | `flatten_audit_findings`/`to_award` are pure and ship with unit tests (Testing Standards principle, first paragraph). New Audit-modal transitions (List↔Report toggle, per-kind selection→modal handoff) get transition tests following the existing `app.rs` suite pattern (second paragraph). No new network-touching code is introduced, so no new `#[ignore]`d test is needed. |
| III. UX Consistency | Esc-cancel convention; typed confirm phrase for destructive writes; CLI/TUI parity; theme-only colors | **PASS**, with one documented, justified gap | `Esc` from either Audit sub-view keeps the constitution's existing documented exception (closes without overwriting status) — this feature doesn't change that rule, only what's inside the modal. Delete/Rename opened from a selected finding still require the existing typed `"delete"`/`"rename"` confirmation phrase — no new one-keypress destructive path is introduced. **CLI/TUI parity gap**: the CLI gains no new flag for this feature (research.md §5) — "select from a list" has no meaning in a one-shot CLI command, and the clerk already has equivalent one-shot flags (`--edit`/`--cell`, `--delete`, `--rename`) to act on anything the existing `--audit` report names. This is the explicit call-out the principle requires when parity isn't practical. New list rendering reuses the existing `ListState`/`List` widget pattern already used by `AddModal`, not a new UI pattern. |
| IV. Performance | Concurrent independent fetches; explicit HTTP timeouts; sub-quadratic scans; non-blocking writes | **PASS** | No new network calls are introduced by this feature at all (research.md §3) — the findings-list refresh after a fix is a local, pure re-computation over already-in-memory data, strictly cheaper than the existing per-write reconcile pattern. Writes routed to from a selected finding still run through the existing background-thread `WorkerMsg` write path — the render loop is never blocked. |
| Security & Data Integrity | Secrets gitignored + mode 600; live stale-write re-check before every mutation; OAuth `state` CSRF check | **PASS** | No new secret handling. Every fix opened from a selected finding goes through `awards-sheets::edit`'s existing live-cell re-check unchanged (research.md §4) — this feature adds zero new write paths, so it can neither weaken nor duplicate that guard. |

No violations requiring justification. Complexity Tracking table is omitted (N/A).

## Project Structure

### Documentation (this feature)

```text
specs/002-audit-fix-selection/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (/speckit-plan command)
├── data-model.md         # Phase 1 output (/speckit-plan command)
├── quickstart.md        # Phase 1 output (/speckit-plan command)
├── contracts/           # Phase 1 output (/speckit-plan command)
│   └── tui-audit-interaction.md
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
crates/
├── awards-core/                 # pure domain logic — no network or filesystem I/O
│   └── src/
│       ├── audit.rs             # collect_sheet_audit (unchanged) + NEW: AuditFinding,
│       │                        # flatten_audit_findings, finding_username
│       ├── types.rs             # Award (unchanged) + NEW: AuditFinding::to_award
│       └── ...                  # (all other modules unchanged by this feature)
└── awards-tui/                    # binary crate: Clap CLI + Ratatui TUI presentation
    └── src/
        └── tui/
            ├── app.rs             # AuditModal extended with `view: AuditView`; NEW:
            │                      # AuditFindingsList, FindingGroup, selection→modal handoff,
            │                      # post-fix local re-flatten (no CLI/main.rs changes)
            └── ui.rs              # render_audit_modal split to render List or Report view
```

**Structure Decision**: The existing 3-crate workspace layout is reused unchanged — this feature
adds one new pure type (`AuditFinding`, `awards-core`) and its presentation-only TUI counterpart
(`AuditFindingsList`, `awards-tui`); no new crate, module boundary, or top-level directory, and
`awards-sheets`/`main.rs` (the CLI) are untouched per the CLI-parity gap documented above.

## Complexity Tracking

*No entries — Constitution Check reported no violations.*
