# Interaction Contract: GUI Edit

Phase 1 output for `specs/007-gui-edit/plan.md`. Describes the observable request/response shape
of the Edit flow — not a network API, but the same kind of interaction contract
004-gui-lookup-add's own `contracts/gui-lookup-add-interaction.md` established for Lookup/Add.

## Trigger

A per-award "Edit" control in the results view, next to each award's name (one per award shown).

## Inputs

| Field | Source | Constraint |
|---|---|---|
| `award` | The clicked award, from the currently looked-up user's results | Fixed for the life of the flow |
| `input` | Free text field, prefilled from `award.cell` (or `award.name` if empty) | Must be non-empty (trimmed) to enable Save |

## Outputs

| Outcome | Effect |
|---|---|
| Save clicked, gate passes | Background write (`update_award_cell(&award, &input, false)`); Save/Cancel disabled, spinner shown |
| Write succeeds, cell still under the viewed user | Results view shows the updated text; edit panel closes; status shows the write's own message |
| Write succeeds, cell reassigned to a different user | Award disappears from the viewed user's results; edit panel closes; status shows the write's message plus "no longer under @<viewed-username>" |
| Write refused (Validation/Stale/Conflict/Api/Other) | Edit panel stays open, `input` unchanged, `submitting` reset to `false`; status shows the refusal message |
| Cancel clicked | Edit panel closes; no write; no change to the results view |
| A different award's Edit control clicked while one edit panel is already open | The open panel is replaced by a fresh one for the newly clicked award (no write for the abandoned one) |
| Add opened while Edit is open (or vice versa) | The other flow's panel closes with no write |
| A new Look Up submitted while Edit is open | The edit panel closes with no write |

## Out of scope for this contract

Delete, Rename, Assist, Audit, and Discord-paste quick-add — unchanged from
004-gui-lookup-add/005-gui-refresh/006-gui-visual-theme.
