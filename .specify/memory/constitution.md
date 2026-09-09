<!--
Sync Impact Report
- Version change: (none) → 1.0.0 (initial ratification)
- Modified principles: n/a (first adoption)
- Added sections:
  - Core Principles: I. Code Quality, II. Testing Standards,
    III. User Experience Consistency, IV. Performance Requirements
  - Security & Data Integrity (Section 2)
  - Development Workflow & Quality Gates (Section 3)
  - Governance
- Removed sections: n/a (first adoption)
- Templates requiring updates: plan-template.md, spec-template.md,
  tasks-template.md, checklist-template.md were NOT modified — this command
  is scoped to the constitution only. Their "Constitution Check" gates should
  be reviewed against the four principles below the next time each is used;
  no changes were required to make the scaffold consistent as of this write.
- Deferred / TODO placeholders: none. RATIFICATION_DATE is set to the date
  this constitution was first written and adopted for the project (today),
  since no prior constitution existed.
-->

# awards-tui Constitution

## Core Principles

### I. Code Quality
Workspace boundaries MUST be preserved: pure domain logic (parsing, indexing,
eligibility, audit/duplicate detection) lives in `awards-core` with no network
or filesystem I/O; Google Sheets/OAuth I/O lives in `awards-sheets`; terminal
and CLI presentation lives in `awards-tui`. A change that mixes these concerns
MUST be refactored to the correct crate before merge.

`cargo clippy --workspace --all-targets -- -D warnings` MUST pass with zero
warnings. A lint suppression (`#[allow(...)]`) MUST carry an inline comment
explaining why the lint doesn't apply.

`unsafe` MUST NOT be introduced without a comment at the unsafe block
documenting the invariant being upheld; the workspace currently contains no
`unsafe` code and that is the default state to preserve.

A dependency declared in a crate's `Cargo.toml` MUST be used by that crate —
no speculative or copy-pasted dependencies left over from another crate.

Error handling at library boundaries (`awards-core`, `awards-sheets`) MUST use
typed errors (`thiserror`), not stringly-typed errors or `anyhow`; `anyhow` is
reserved for the `awards-tui` binary crate's top-level error handling.

**Rationale**: this is a three-crate workspace specifically split so the
business rules (award parsing, master-badge eligibility, duplicate detection)
can be tested and reasoned about without a terminal or a network connection.
Letting I/O or presentation leak into `awards-core` erodes the reason the
split exists.

### II. Testing Standards
Every pure function added to `awards-core` (parsing, formatting, eligibility,
indexing, audit logic) MUST ship with unit tests covering its edge cases in
the same change that adds it.

Every new TUI modal or modal state transition in `awards-tui` MUST have a
corresponding test asserting the resulting `App` state (`modal`, `status`,
`busy`, and any modal-specific fields) — following the Modal transition test
suite already established in `crates/awards-tui/src/tui/app.rs`.

Code that performs network I/O (Sheets API calls, OAuth) MUST be structured so
its business logic is testable offline. A test that requires live credentials
or a live network call MUST be marked `#[ignore]` and MUST NOT run in the
default `cargo test` invocation used by CI.

`cargo test --workspace --locked` and `cargo clippy --workspace --locked -- -D
warnings` MUST both pass before a change is merged. `.github/workflows/*.yml`
is the enforcement mechanism for this; CI configuration MUST NOT be weakened
(for example, dropping `--locked` or `-D warnings`) without amending this
principle first.

**Rationale**: the project's most-used paths (lookup, duplicate audit,
master-badge assist, and now the TUI's own modal state machine) are all
covered by tests that run in milliseconds with no network — that's what makes
`cargo test --workspace` a real pre-commit gate instead of a formality.

### III. User Experience Consistency
Modal interaction rules MUST stay uniform across the TUI: `Esc` cancels any
open modal and sets a "Dialog cancelled" status, except the read-only Audit
browser, which simply closes without overwriting the status line. A new modal
MUST follow this convention, or the exception MUST be documented here.

Any write operation that is destructive or hard to reverse (delete, rename)
MUST require a typed confirmation phrase before it reaches the network layer,
matching the existing `"delete"` / `"rename"` pattern — a single keypress MUST
NOT be sufficient to trigger an irreversible sheet write.

The CLI and TUI MUST offer equivalent coverage of read/write operations. A
capability added to one interface SHOULD be added to the other in the same
change; where that isn't practical, the gap MUST be called out explicitly in
the change description.

All colors and theme values MUST come from the `Theme` struct / the
`awards-tui.toml` configuration path — no ad-hoc `Color::Rgb(...)` literals
scattered through rendering code outside `config.rs`.

**Rationale**: this tool is operated by logistics clerks under time pressure,
often mid-conversation in Discord. Inconsistent confirmation gates or
divergent CLI/TUI behavior are exactly the kind of surprise that causes a
wrong write to a shared, authoritative spreadsheet.

### IV. Performance Requirements
Network calls that can run independently (fetching the three sheet tabs,
resolving live rows for reconciliation) MUST be issued concurrently rather
than sequentially, as `build_awards_data`'s threaded fetch already does.

All outbound HTTP requests MUST set an explicit timeout (currently 60s). An
HTTP client built without a timeout MUST NOT be merged.

Duplicate/similarity scanning over sheet data MUST stay sub-quadratic in the
common case — the existing 3-character-prefix bucketing before running
Levenshtein comparison is the pattern to follow, not an all-pairs comparison
of every username against every other username.

Sheet-mutating operations MUST NOT block the terminal UI thread. Writes MUST
run on a background thread and report back through the existing `WorkerMsg`
channel, keeping the render loop responsive while a write is in flight.

**Rationale**: the Decorations Database is a shared, growing spreadsheet used
interactively; a lookup or audit that starts feeling slow, or a UI that
freezes mid-write, directly costs clerks' time and erodes trust in the tool.

## Security & Data Integrity

Secrets (`credentials.json`, `token.json`, `service_account.json`,
`client_secret*.json`, `*.pem`) MUST remain listed in `.gitignore` and MUST
NOT be committed. A change that introduces a new secret-bearing filename
pattern MUST add that pattern to `.gitignore` in the same change.

Secret files written to disk MUST use restrictive permissions (mode 600 on
Unix, as `save_authorized_user` already does); code that reads a secret file
SHOULD warn when it finds looser permissions, matching
`warn_if_insecure_secret_file`.

Every sheet-mutating operation (add/edit/delete/rename) MUST re-check the live
cell value immediately before writing and refuse the write with a clear
"stale, refresh and retry" message on mismatch, to guard against concurrent
edits by other clerks.

OAuth flows MUST validate the `state` parameter on callback to prevent CSRF,
matching the existing `interactive_login` implementation.

## Development Workflow & Quality Gates

Commits on `master` SHOULD be GPG-signed, per `CONTRIBUTING.md`. This is a
SHOULD, not a MUST, because commits made before signing was adopted are
grandfathered in as Unverified and MUST NOT be rewritten just to sign them.

CI (`cargo test --workspace --locked`, `cargo clippy --workspace --locked --
-D warnings`) is the mandatory gate for every push and pull request. A red CI
run MUST block merge.

Superseded scripts or dead code MUST be removed or moved to `archive/` rather
than left in an active directory with a comment pointing elsewhere.

## Governance

This constitution supersedes ad-hoc practice for the awards-tui repository.
Where this document and unwritten convention disagree, this document wins.

Amendments are made by editing `.specify/memory/constitution.md` directly
(this is a solo-maintained project) and MUST update the Sync Impact Report,
the version number, and the Last Amended date in the same change.

Versioning policy (semantic versioning applied to governance):
- MAJOR: backward-incompatible removal or redefinition of a principle.
- MINOR: a new principle or materially expanded guidance is added.
- PATCH: wording, typo, or clarification changes with no rule change.

Every change SHOULD be checked against the four Core Principles and the
Security & Data Integrity section before merge, including solo self-review.
`CONTRIBUTING.md` SHOULD reference this constitution as the source of truth
for contribution standards.

**Version**: 1.0.0 | **Ratified**: 2026-09-09 | **Last Amended**: 2026-09-09
