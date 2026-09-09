# Contract: `awards-tui` CLI Surface

This is the feature's externally-consumable interface — the Clap-derived command surface in
`crates/awards-tui/src/main.rs` that scripts, Discord bots, or a clerk's terminal can call
directly (as opposed to the interactive TUI, which is a presentation layer over the same
underlying functions and is out of scope for a machine-readable contract). All commands below
already exist; this table documents the contract so a future change can be checked against it,
per the Constitution's CLI/TUI-parity principle.

## Read (User Story 2 — no auth)

| Command | Behavior | Exit code |
|---|---|---|
| `awards-tui USERNAME` (or `--cli USERNAME`) | Prints USERNAME's awards grouped by Ribbons/Badges/Foreign Awards (FR-001, FR-010) | `0` on awards found, `1` if none found |

## Identity (User Story 1)

| Command | Behavior | Exit code |
|---|---|---|
| `awards-tui --login` | Runs the interactive OAuth desktop flow (or validates a service account) and caches credentials (FR-002, FR-003) | `0` success, `1` failure |
| `awards-tui --auth-status` | Reports which auth mode is active (`service_account` / `oauth_token` / `oauth_needs_login` / `missing`) and where credentials live | `0` always |

## Write (User Stories 1 and 3 — requires a usable token; each subject to FR-004 and FR-009)

| Command | Behavior | Exit code |
|---|---|---|
| `awards-tui USERNAME --add AWARD [--suffix SUFFIX]` | Logs a new award for USERNAME (FR-005) | `0` success, `1` no/ambiguous match or write failure, `2` missing USERNAME |
| `awards-tui USERNAME --edit AWARD --cell CELL` | Corrects USERNAME's existing AWARD entry to CELL (FR-006) | `0` success, `1` no/ambiguous match or write failure, `2` missing USERNAME/CELL |
| `awards-tui USERNAME --delete AWARD` | Removes USERNAME's existing AWARD entry, shifting the column up (FR-007) | `0` success, `1` no/ambiguous match or write failure, `2` missing USERNAME |
| `awards-tui USERNAME --rename NEW` | Rewrites every award cell USERNAME owns to NEW in one action (FR-008) | `0` all cells renamed, `1` partial/total failure (message names remaining ranges — retry the same command) |

## Out of scope for this feature (already exist, unrelated to USAR Award Logging's FRs)

`--audit [--audit-out FILE]`, `--check AWARD`, `--grant AWARD` implement duplicate-scanning and
clerk-assist eligibility respectively; `spec.md`'s Assumptions explicitly exclude both from this
spec's scope. They are listed here only so a future change to the shared `Cli` struct doesn't
accidentally regress this feature's four write commands above.

## Error contract

Every command returns human-readable text on stdout (success) or stderr (failure) — there is no
JSON/machine-readable output mode today. A write failure's message text is whatever
`EditResult.message` or `anyhow::Error` produced, which for a permission or staleness failure
includes Google's own error detail (see `research.md` §4–5) rather than a fixed error code.
Scripts that need to distinguish failure *reasons* must currently parse this text; introducing a
structured error output is out of scope for this feature.
