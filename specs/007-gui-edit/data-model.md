# Data Model: GUI Edit

Phase 1 output for `specs/007-gui-edit/plan.md`.

## New state

### `EditFlow` (`crates/awards-gui/src/app.rs`)

```rust
pub struct EditFlow {
    pub award: Award,      // fixed for the life of the flow
    pub input: String,     // free-typed; starts as award.cell (or award.name if cell is empty)
    pub submitting: bool,  // true while update_award_cell runs on a background thread
}
```

`GuiApp` gains one new field: `pub edit: Option<EditFlow>` — `Some` only while the inline edit
panel is open, the same shape `AddPicker` already uses.

### `GuiMsg::EditDone` (`crates/awards-gui/src/app.rs`)

```rust
EditDone {
    username: String,   // the viewed user at the time Save was clicked
    result: EditResult, // the same EditResult/EditError type Add already consumes
},
```

## State transitions

| From | Action | To |
|---|---|---|
| `edit: None` | `open_edit(award)` | `edit: Some(EditFlow { award, input: prefilled, submitting: false })`; `add_picker` cleared |
| `edit: Some(_)` | `set_edit_input(text)` | `edit.input` updated |
| `edit: Some(_)` | `cancel_edit()` | `edit: None` |
| `edit: Some(_)` | `confirm_edit()` (gated by `can_confirm_edit()`) | `edit.submitting: true`; background thread spawned |
| `edit: Some(_)`, submitting | `handle_edit_done(username, Ok result)` | index patched, viewed list recomputed, `edit: None` |
| `edit: Some(_)`, submitting | `handle_edit_done(username, Err result)` | `edit.submitting: false`, `edit.input` untouched |
| any | `open_add_picker()` | `edit: None` |
| any | `submit_lookup()` | `edit: None` (alongside the existing `add_picker: None`) |

## Functions touched

- `crates/awards-gui/src/app.rs`: `open_edit`, `set_edit_input`, `cancel_edit`,
  `can_confirm_edit`, `confirm_edit`, `handle_edit_done` (new); `handle_msg`, `submit_lookup`,
  `open_add_picker` (small additions, no behavior change to their existing paths).
- `crates/awards-gui/src/ui.rs`: `render_edit` (new); `render_results` gains a per-award "Edit"
  button and a three-column layout (spacing/layout polish, no new state).
- No change to `awards-core` or `awards-sheets` — `update_award_cell` already existed and is
  reused unchanged.
