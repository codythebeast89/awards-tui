# Contract: TUI Paste-to-Prefill Interaction

This feature's external interface is a new interactive entry point inside `awards-tui`'s TUI — not
a new CLI surface (see `research.md` §9 for why the CLI is unchanged). This document is the
interaction contract `crates/awards-tui/src/tui/app.rs`, `mod.rs`, and `ui.rs` must satisfy, in the
same role `contracts/tui-audit-interaction.md` played for feature 002's Audit browser.

## Entry point

`Action::PasteAdd` (new — Actions pane entry, plus direct hotkey `p`) opens `Modal::PasteAdd` with
an empty buffer. Unlike `Action::Add`, this does **not** require a username to already be looked
up (`results_username`) — that is the entire point of the feature (research.md §2).

## View: Paste buffer (`Modal::PasteAdd`)

| Input | Behavior |
|---|---|
| `Event::Paste(text)` | Appends `text` to `buffer` (research.md §1). This is the primary way content arrives — a clerk pasting a Discord message in a terminal with bracketed-paste support delivers the whole multi-line message as one such event. |
| Character keys, `Backspace` | Edits `buffer` directly (fallback for terminals without bracketed-paste support, or minor manual correction of a pasted value — research.md §1). |
| `Enter` | Submits `buffer` for extraction (see "Submit behavior" below). A `Enter` that arrives as part of a bracketed paste is already consumed as buffer content by the `Event::Paste` handling above, so this only fires for a genuine, separate keypress after the paste completes. |
| `Esc` | Closes the modal entirely and sets status to `"Dialog cancelled"` — the existing default convention (no new constitution exception needed, unlike `002-audit-fix-selection`'s Audit modal). |

Rendering (`ui.rs`): shows the accumulated buffer (or a placeholder hint when empty, e.g. "Paste
the Discord message, then press Enter") and, when `error` is set, an inline message below it.

## Submit behavior

On `Enter`, call `extract_paste_fields(&buffer)` (data-model.md), then:

1. **No username extracted or validated** (`ExtractedRequest.username == None`): stay in
   `Modal::PasteAdd`, set `error` to a clerk-facing message (e.g. "Couldn't find a username in
   that text — check it and try again, or Esc to cancel"), and keep `buffer` unchanged so the
   clerk can fix and resubmit rather than losing their paste (spec User Story 2, Acceptance
   Scenario 2, and the Edge Case for a totally unparseable paste).

2. **Username extracted and validated**: call `apply_user_view(username, None, None)` (local, no
   network — research.md §7), then resolve the award:
   - Split any trailing count indicator via `split_award_suffix` (research.md §4) and match the
     base text against `data.catalog` via `match_catalog_entries` (research.md §5).
   - **Exactly one match**: close `Modal::PasteAdd` and open `Modal::Add(AddModal { chosen:
     Some(matched), step: AddStep::Suffix, suffix: <extracted suffix>, all_candidates,
     filtered: all_candidates, .. })` — landing exactly where a clerk who manually picked that
     award and pressed Enter would land (spec User Story 1, Acceptance Scenarios 1–3).
   - **Zero or multiple matches, or no award text extracted at all**: close `Modal::PasteAdd` and
     open the normal `Modal::Add(AddModal::new(candidates))` (`AddStep::Pick`) with `filter`
     pre-filled to the raw extracted award text (or left empty if none was found), so the clerk
     starts from an already-narrowed (or full) list instead of retyping from scratch (spec User
     Story 2, Acceptance Scenario 1).
   - If the user has no remaining awards to add (same precondition `action_add` already checks),
     surface the same "No remaining awards to add for this user" status `action_add` already
     gives, instead of opening an empty picker.

## Post-resolution behavior

Once resolved into `Modal::Add`, every existing behavior of that modal — Pick-step filtering,
Suffix-step editing, `Enter` to confirm and write via `add_award_to_user`, `Esc` to cancel with
"Dialog cancelled" — is **completely unchanged** by this feature. This feature only changes how
`AddModal` is constructed on entry, never how it is driven once open.

## Out of scope for this contract

No new CLI flags (research.md §9), no new network endpoints, no new persisted file format, and no
change to how the Proof portion of a pasted message is handled — it is never extracted or acted on
at all (research.md §8).
