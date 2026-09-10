# Research: Audit Fix Selection

Phase 0 output for `specs/002-audit-fix-selection/plan.md`. The feature spec's Technical Context
carried no `[NEEDS CLARIFICATION]` markers, so the research questions here are the implementation
unknowns needed to turn the spec's functional requirements into a concrete design against the
existing `awards-tui` codebase, not open product questions.

## 1. How should the four existing finding kinds be represented for selection?

**Decision**: Add one new, I/O-free `AuditFinding` enum in `awards-core::audit` that wraps
`AuditDuplicateGroup` (split one entry per row so each row is independently selectable),
`AuditSimilarPair`, `AuditMalformed`, and `AuditUnparsed` — the four structs `collect_sheet_audit`
already produces — rather than inventing new parallel data structures. Add a
`fn flatten_audit_findings(report: &AuditReport) -> Vec<AuditFinding>` that flattens
`AuditReport`'s four `Vec<_>` fields into one ordered, groupable list, and a
`fn finding_username(finding: &AuditFinding) -> Option<&str>` helper (returns `None` only for
`AuditUnparsed`, which has no extractable username) used to group findings by person (FR-002) and
to answer spec Edge Case "what happens with a cell no username could be extracted from" (FR-009).

**Rationale**: `AuditReport`'s four vectors already carry every field a fix needs (sheet, col,
row, cell, base_name, and for duplicates/similar-pairs the username(s) involved) — see
`data-model.md`. Reusing them keeps the audit's scan logic (already covered by
`test_dedupe`, `test_duplicate_identical_vs_conflict`, `test_find_duplicates_for_user`, and
friends in `crates/awards-core/tests/offline.rs`) completely unchanged; this feature only adds a
thin, independently-testable flattening/grouping layer on top, satisfying the constitution's
Code Quality principle (`awards-core` stays pure, no new I/O) and Testing Standards principle
(new pure function ships with its own unit tests).

**Alternatives considered**: Redesigning `collect_sheet_audit` to emit one unified list directly
was rejected — it would touch and re-test scan logic that isn't part of this feature's scope and
risks changing the plain-text report's content (spec User Story 2 requires the report stay
byte-identical). Building the flattened list ad hoc inside `awards-tui` (rather than as an
`awards-core` type) was rejected because finding→award mapping is domain logic, not presentation,
per the Code Quality principle's workspace-boundary rule.

## 2. How does a selected finding become a pre-filled fix action?

**Decision**: Add `fn AuditFinding::to_award(&self) -> Option<Award>` (mirrors the existing
`DuplicateHit::to_award()` pattern in `awards-core::types`) that builds the same `Award` shape the
Edit/Delete modals already consume from `sheet`/`col`/`row`/`cell`/`base_name`/`category`, so
`action_edit`/`action_delete`'s existing modal-construction code paths are reused unchanged for
duplicate-row and malformed-cell findings — selecting a finding just supplies the `Award` those
actions already expect, instead of requiring the clerk to have it selected in the Awards pane
first. For a similar-username-pair finding, `to_award()` returns `None` (there's no single row to
target) and the TUI instead opens the existing `RenameModal` pre-filled with whichever of the two
usernames the clerk picks as the "from" name (spec Edge Case: "which of the two usernames is the
mistake" — the clerk decides, per FR-010). For an unparseable-cell finding, no fix action is
offered at all; the finding entry instead shows its sheet/column/row for manual lookup (FR-009).

**Rationale**: Every fix this feature routes to (Edit, Delete, Rename) is already implemented,
tested, and OAuth/stale-write-gated in `awards-sheets::edit` and wired into `app.rs`'s existing
`action_edit`/`action_delete`/rename-modal construction — reusing those call paths verbatim is
what satisfies FR-008 (no new or lesser-guarded write surface) and the constitution's requirement
that destructive writes keep their existing typed-confirmation gate (Delete/Rename already require
typing the word).

**Alternatives considered**: A dedicated "fix from audit" write path bypassing the existing
Edit/Delete/Rename modals was rejected outright — it would duplicate the stale-write re-check and
confirmation-phrase logic those modals already have, directly contradicting FR-008.

## 3. How does the findings list refresh after a fix, without blocking on the network?

**Decision**: Re-run `collect_sheet_audit(&self.data)` locally against the TUI's already-in-memory
`AwardsData` immediately after a write completes, the same moment `apply_edit_result` /
`apply_delete_result` / `apply_rename_result` already patch `data.index` and `data.sheet_rows` in
place (via `upsert_award_in_index`, `patch_sheet_cell`, `reindex_column_after_delete`,
`shift_column_up_in_rows`). No new network fetch is introduced.

**Rationale**: `collect_sheet_audit` is a pure function over `AwardsData.sheet_rows`, and every
existing write-completion handler already keeps `sheet_rows` in sync locally (this is how the
Awards pane already reflects a write instantly without re-fetching the whole sheet). Piggybacking
the audit-list refresh on that same, already-tested local-patch step satisfies FR-006 ("update the
list... without the clerk having to separately re-run the audit") and the constitution's
Performance principle (no UI-blocking network round trip) for free.

**Alternatives considered**: Re-fetching all three sheet tabs via `build_awards_data` after every
fix was rejected as unnecessary network load and latency for something the app can already do
locally; a network refresh remains available anyway via the existing manual Refresh action (F5).

## 4. How does the existing stale-write guard surface for a fix opened from the audit list?

**Decision**: No new mechanism — `find_live_row` / `live_cell_value` / `cell_stale_message`
inside `add_award_to_user` / `update_award_cell` / `remove_award` / `rename_username`
(`awards-sheets::edit`) already re-check the live cell immediately before writing and return a
typed `EditError::Stale` (see `crates/awards-sheets/src/edit.rs`, this session's earlier
`EditError` work) when the sheet has moved on since the app last saw it. A fix launched from a
selected audit finding goes through these exact functions, so a finding whose underlying cell
changed since the audit ran (spec Edge Case) is already caught and reported the same way a stale
write from Edit/Delete/Rename always has been.

**Rationale**: Satisfies FR-007 and the constitution's Security & Data Integrity principle
("every sheet-mutating operation MUST re-check the live cell value immediately before writing")
with zero new code — this is exactly what routing through the existing edit/delete/rename
functions (Decision 2, above) already buys.

**Alternatives considered**: None seriously considered; re-implementing a staleness check
specific to the audit flow would violate the constitution's DRY intent behind that principle and
risk drifting from the existing `cell_stale_message` wording.

## 5. CLI parity

**Decision**: No new CLI flag. The constitution's UX Consistency principle requires a capability
gap between CLI and TUI to be called out explicitly rather than silently accepted — recorded here
and in `plan.md`'s Constitution Check.

**Rationale**: The CLI's `--audit` is a one-shot, scriptable report command; "select an entry from
a list" has no meaning in that context. The clerk already has the equivalent one-shot mutation
flags (`--edit`/`--cell`, `--delete`, `--rename`) to act on any username/cell the `--audit` report
names — this feature's only addition is a faster, pre-filled *path* to those same underlying
actions inside the interactive TUI, not a new capability the CLI lacks.
