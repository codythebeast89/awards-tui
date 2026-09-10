# Data Model: Audit Fix Selection

Maps the feature spec's Key Entities to concrete types, building on `awards-core::audit`'s
existing `AuditReport` / `AuditDuplicateGroup` / `AuditSimilarPair` / `AuditMalformed` /
`AuditUnparsed` (all unchanged by this feature — see `research.md` §1).

## `AuditFinding` (new — `awards-core::audit`)

The spec's **Audit Finding** entity: one selectable, individually-addressable issue. A thin,
I/O-free view over the four kinds `collect_sheet_audit` already produces — no new scan logic.

```rust
pub enum AuditFinding {
    DuplicateRow {
        user: String,               // from AuditDuplicateGroup.user
        sheet: String,
        col: String,
        base_name: String,
        kind: String,                // "identical" | "conflict"
        row: i32,                    // one row of AuditDuplicateGroup.rows, per-finding
        cell: String,
    },
    SimilarUsernames {
        a: String,                   // AuditSimilarPair.a
        b: String,                   // AuditSimilarPair.b
        sheet: String,
        col: String,
        base_name: String,
    },
    MalformedCell {
        user: String,                // normalize_username(&cell) — best-effort, may be empty
        sheet: String,
        col: String,
        base_name: String,
        row: i32,
        cell: String,
        issues: Vec<String>,
    },
    UnparseableCell {
        sheet: String,
        col: String,
        base_name: String,
        row: i32,
        cell: String,                // raw, unparsed text
    },
}
```

**Validation / construction rules** (from spec FRs, enforced by `flatten_audit_findings`):

- One `AuditFinding::DuplicateRow` per row in an `AuditDuplicateGroup.rows` (a group with 3
  duplicate rows becomes 3 findings) — each row is independently fixable (removed one at a time),
  matching how `remove_award` already operates on a single row.
- `AuditFinding::SimilarUsernames` carries **both** usernames un-ordered; which one is "the
  mistake" is a clerk decision at selection time (FR-010), not baked into the finding.
- `AuditFinding::UnparseableCell` has no `user` field at all — there is deliberately no username to
  key this on (FR-009); it can only be grouped under an "Unparseable" bucket, never under a person.

**Relationships**:

- `AuditReport` (existing) — `flatten_audit_findings(&AuditReport) -> Vec<AuditFinding>` is the
  sole constructor; findings are always derived, never independently created or mutated.
- `Award` (existing, `awards-core::types`) — `AuditFinding::to_award(&self) -> Option<Award>`
  (new) produces the same shape `EditModal`/`DeleteModal` already key off of, for the two kinds
  that map onto a single row (`DuplicateRow`, `MalformedCell`); returns `None` for
  `SimilarUsernames` and `UnparseableCell`, which have no single target row.

## `AuditFindingsList` (new — `awards-tui::tui::app`, presentation-only)

The spec's **Audit Findings List** entity: the clerk-facing, grouped, navigable presentation.
Not a new domain concept — a TUI-side grouping and selection cursor over `Vec<AuditFinding>`.

> **As-built correction (implementation-time, see `tasks.md` T007/T017 remediation notes)**: the
> sketch below is this document's *original* design. It was corrected during implementation
> because `AuditView::List(AuditFindingsList)` carrying the list payload directly would discard
> that payload the moment the clerk toggled to `AuditView::Report` (which has nowhere to hold it)
> and back. The as-built shapes are:
>
> ```rust
> pub enum AuditView {
>     List,
>     Report,
>     ChooseUsername { a: String, b: String }, // transient SimilarUsernames sub-dialog, see below
> }
>
> pub struct AuditModal {
>     pub path: String,
>     pub lines: Vec<String>,      // unchanged — still backs AuditView::Report
>     pub scroll: u16,             // unchanged — still backs AuditView::Report
>     pub view: AuditView,         // which face is displayed; stateless, separate from `list`
>     pub list: AuditFindingsList, // persistent — survives every List/Report toggle
>     pub list_state: ListState,
> }
>
> pub struct FindingGroup {
>     pub username: Option<String>,   // None => the "(unattributed)" bucket (UnparseableCell)
>     pub findings: Vec<AuditFinding>,
> }
>
> pub struct AuditFindingsList {
>     pub groups: Vec<FindingGroup>,  // findings grouped by finding_username(), stable order
> }
> ```
>
> Selection lives on `AuditModal.list_state`, indexed over *rendered rows* (one non-selectable
> header row per group, followed by its findings) rather than over `AuditFindingsList.findings`
> directly; `AuditFindingsList::selectable_rows()`/`finding_at_row()` bridge a row index to the
> `AuditFinding` it holds (or `None` for a header row), and a `move_audit_row` helper walks
> `selectable_rows()` so navigation always skips headers. The rest of this section's narrative
> (grouping rule, fix-selection behavior, post-fix refresh) is unchanged by this correction — only
> the concrete field layout differs from the original sketch.

```rust
pub struct AuditFindingsList {
    pub findings: Vec<AuditFinding>,   // flatten_audit_findings output, most-recent audit pass
    pub groups: Vec<FindingGroup>,     // findings grouped by finding_username(), stable order
    pub state: ListState,              // ratatui selection cursor, existing pattern (cf. AddModal)
}

pub struct FindingGroup {
    pub label: String,                 // "@username" or "Unparseable cells" bucket
    pub finding_indices: Vec<usize>,   // indices into AuditFindingsList.findings
}
```

**State transitions** (extends the existing `AuditModal`; see `contracts/tui-audit-interaction.md`
for the full interaction contract):

1. `Action::Audit` → `run_audit_worker` (existing, unchanged) → **new**: `AuditModal` gains a
   `view: AuditView` field (`List(AuditFindingsList)` default, or `Report` — the existing
   scrollable text view, still reachable per spec User Story 2) instead of always opening
   directly into the text report.
2. Selecting a `DuplicateRow` or `MalformedCell` finding and confirming → closes the Audit modal,
   opens `Modal::Delete` or `Modal::Edit` pre-filled from `AuditFinding::to_award()`, exactly as
   if that `Award` had been selected in the Awards pane (existing `action_delete`/`action_edit`
   code path, unchanged). As-built: the Audit modal isn't discarded on this transition, it's
   stashed in a new `App.saved_audit: Option<AuditModal>` field so Esc or a failed write can
   restore it unchanged (see transition 6 below and `tasks.md` T011/T017-T018).
3. Selecting a `SimilarUsernames` finding → a small in-place choice of which of the two usernames
   is "the mistake" (FR-010), then opens `Modal::Rename` pre-filled with that username as `from`
   (existing rename-modal construction, unchanged). As-built: the choice is the transient
   `AuditView::ChooseUsername { a, b }` sub-state on the *same* Audit modal (keys `1`/`2`), not a
   separate dialog — Esc from it returns to `AuditView::List` rather than closing anything.
4. Selecting an `UnparseableCell` finding → no modal transition; the location (sheet/col/row) is
   shown, matching FR-009.
5. On `WorkerMsg::WriteDone` success from a fix opened this way (tracked via an added
   `reopen_audit: bool` flag on the pending write, mirroring the existing pattern in
   `WorkerMsg::WriteDone`) → after the existing `apply_*_result` local patch of `data.index` /
   `data.sheet_rows` (research.md §3), findings are recomputed via `flatten_audit_findings` over
   the same already-fresh `AuditReport` from `collect_sheet_audit(&self.data)`, and the modal
   returns to `AuditView::List` with the resolved finding gone (FR-006) — no network call.
   As-built: the marker is `App.audit_fix_username: Option<String>` (doubles as the username
   context for status/attribution), consumed by `handle_audit_write_done` /
   `reopen_audit_from_local_data`; on failure the stashed `saved_audit` is restored unchanged
   instead of a fresh recompute, so a stale-write rejection can never silently drop a finding that
   is, in fact, still unresolved.
6. `Esc` from `AuditView::List` closes the modal without overwriting `status`, matching the
   constitution's existing documented exception for the Audit browser (Principle III). As-built:
   `Esc` is context-sensitive across three cases — from `AuditView::List`/`Report`, closes the
   modal (as originally specified); from `AuditView::ChooseUsername`, returns to
   `AuditView::List` (transition 3); from a fix modal (`Modal::Edit`/`Delete`/`Rename`) opened via
   transition 2/3 (i.e. `App.saved_audit.is_some()`), restores the stashed Audit modal instead of
   closing everything, so backing out of a fix returns the clerk to the list they were browsing.

**Relationships**:

- `AuditModal` (existing) — extended with the `view` field above; `path`/`lines`/`scroll` are
  unchanged and still back `AuditView::Report`.
- `EditModal` / `DeleteModal` / `RenameModal` (existing, unchanged) — the fix-action targets this
  feature opens into; no new modal type is introduced for applying a fix, only for browsing and
  selecting findings.
