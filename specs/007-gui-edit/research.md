# Research: GUI Edit

Phase 0 output for `specs/007-gui-edit/plan.md`.

## 1. What "Edit" actually does, per the terminal tool's own precedent

**Decision**: Edit corrects the raw Sheets cell text of an already-recorded award — the same
cell the `Award` already points at (`sheet`/`col`/`row` unchanged; only the text written into
that cell changes). It never moves an award to a different column or creates a new one.

**Rationale**: Confirmed by reading the terminal tool's own Edit path end to end:
`awards-tui`'s `action_edit` (`crates/awards-tui/src/tui/app.rs`) opens `EditModal { award,
input }` prefilled from `award.cell` (or `award.name` if the cell is empty), a single Enter
commits via `commit_edit`, which calls `awards_sheets::update_award_cell(&award, &new_cell,
false)` — the exact function this feature's GUI counterpart calls.

**Alternatives considered**: Letting Edit change *which* award a row represents (moving between
catalog columns) — rejected; that's not what the terminal tool's Edit does, and the underlying
function has no such capability.

## 2. Confirmation gate

**Decision**: No typed confirmation phrase. A single Save action (button click or Enter) submits,
gated only on sign-in state and a non-empty field — identical in strength to Add's existing gate.

**Rationale**: Constitution III reserves a typed confirmation phrase for destructive/hard-to-
reverse writes (delete, rename). Edit corrects an existing entry — the old text is still visible
in the field being edited, and a mis-edit is itself correctable by editing again — so it does not
meet that bar, and the terminal tool's own Edit modal submits on a single Enter with no typed
phrase. Matching that is the CLI/TUI-parity-preserving choice.

**Alternatives considered**: A typed-word confirmation like Delete/Rename — rejected as
inconsistent with the terminal tool's own Edit behavior and unnecessary caution for a correction,
not a destruction.

## 3. Auth gating — a deliberate GUI-specific hardening, not a functional gap

**Decision**: Save is disabled unless `auth == SignedIn`, mirroring Add's `can_confirm_add`.

**Rationale**: The terminal tool's own `action_edit`/`commit_edit` has *no* explicit auth check —
it just lets `update_award_cell`'s own `SheetsApi::connect(false)` call fail if there's no token,
and the resulting error surfaces on the terminal's always-visible status line. The GUI has no such
persistent line the clerk is already looking at mid-typing, so (as this project's 004-gui-lookup-
add plan.md already established for Add) gating in advance avoids a confusing round trip. This is
the same reasoning already applied once in this codebase, now applied consistently to Edit.

**Alternatives considered**: No GUI-side gate, matching the TUI literally — rejected for the same
reason 004 rejected it for Add.

## 4. Reconciling the view after a write — recompute, not hand-patch

**Decision**: On a successful edit, patch the shared index (`upsert_award_in_index`, unchanged
from Add's own pattern) and then *recompute* the viewed user's award list from that patched index
(`get_awards_for_username`), rather than hand-patching one entry in the in-memory list the way
`handle_add_done` does.

**Rationale**: Add always adds to the currently-viewed user, so hand-patching is safe there. Edit
can *reassign* a cell's leading username to someone else (the underlying write only guards against
a live collision, not against a deliberate reassignment) — recomputing from the index is what
correctly makes the award disappear from the current view when that happens, mirroring the
terminal tool's own `apply_edit_result`, which does exactly this via `apply_user_view` re-deriving
`self.results` from `get_awards_for_username(&data.index, username)` after the same
`upsert_award_in_index` call.

**Alternatives considered**: Hand-patching by `sheet`+`col` match, with a separate explicit
"did the username change" branch duplicating the terminal tool's `normalize_username` comparison
— rejected as more code for the same outcome; recomputing from the already-correct index is
simpler and can't drift out of sync with what `get_awards_for_username` considers correct.

## 5. Mutual exclusivity with Add

**Decision**: Opening Edit closes any open Add picker; opening Add closes any open Edit. A fresh
Look Up closes both.

**Rationale**: `awards-gui` has no `Modal` enum (unlike the TUI) — each flow is its own
`Option<...>` field — but the terminal tool's single-modal-at-a-time rule (Constitution III) is
still the right invariant for a GUI window: two simultaneously open inline write panels would be
confusing and there is no code reason to allow it.

**Alternatives considered**: Allowing both open simultaneously (two independent panels) —
rejected as unnecessary complexity with no acceptance scenario requesting it.
