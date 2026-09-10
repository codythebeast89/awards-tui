# Contract: GUI Lookup & Add Interaction

This feature's external interface is a new native desktop window (`awards-gui`) — not a new CLI
surface (research.md §8). This document is the interaction contract `crates/awards-gui/src/app.rs`
and `ui.rs` must satisfy, in the same role `contracts/tui-paste-interaction.md` played for feature
`003`'s paste modal and `contracts/tui-audit-interaction.md` played for feature `002`'s Audit
browser.

## Window layout (first milestone)

A single window, no modal dialogs (unlike the TUI's modal-stack model — `egui` panels/side-areas
are the natural equivalent here): a username input + submit control, a read-only awards list for
the looked-up user (grouped by category), a Sign In control (visible only when `auth != SignedIn`),
and an Add control that opens the award picker inline (not a separate window).

## Lookup

| Input | Behavior |
|---|---|
| Typing in the username field | Updates `GuiApp.username_input` live; no lookup yet. |
| Submitting (Enter, or a Look Up button) | Calls `submit_lookup()` (data-model.md) — purely local, no network call, matching `awards-tui`'s `apply_user_view` (research.md §6). |
| A username with existing records | `looked_up` is populated with that user's awards, grouped by category for display. |
| A username with no records | `looked_up.not_found` is `true`; the UI states plainly that nothing was found (spec FR-008) — never an empty, unexplained list. |
| Sync still in progress (`syncing == true`) | Lookup is disabled with a "Syncing..." indicator rather than searching an incomplete or absent index. |

## Sign-in

| Input | Behavior |
|---|---|
| `auth == SignedIn` | The Sign In control is hidden entirely — there is nothing to do. |
| `auth != SignedIn` and the clerk clicks "Sign In" | `auth` becomes `SigningIn` (button disabled, shows a spinner); spawns a background thread calling `login()` verbatim (research.md §4). |
| `LoginDone(Ok(msg))` | `status = msg`; `refresh_auth_state()` re-derives `auth` from `auth_status()` — becomes `SignedIn` on genuine success. |
| `LoginDone(Err(msg))` | `status = msg` (the existing `AuthError` message, e.g. "No credentials found..."); `auth` returns to `SignedOut` — the clerk can retry. |

## Add (only reachable once a user is looked up)

| Input | Behavior |
|---|---|
| Clerk clicks "Add Award" (only enabled when `looked_up` is `Some` and not empty-catalog) | `open_add_picker()` computes `candidates` once (`data.catalog` minus this user's already-owned awards, research.md §6) and opens the inline picker. If `candidates` is empty, the button instead shows "No remaining awards to add for this user" and does not open a picker (mirrors the TUI's existing `action_add` precondition message). |
| Typing in the picker's search box | `update_filter(text)` recomputes `filtered` via `match_catalog_entries` (research.md §6) — identical behavior to the TUI's own Add picker filter. |
| Selecting an award from `filtered` | `select(def)` sets `add_picker.selected` — this alone does **not** write anything (spec FR-004: selecting and confirming are separate steps). |
| Clerk edits the optional suffix field | Free text, same convention the TUI's `AddStep::Suffix` already uses (e.g. `x1`, `x2`, or a detail string) — not validated client-side beyond what `build_cell_value`/`add_award_to_user` already accept. |
| Clerk clicks "Confirm" (only enabled when `selected.is_some()` and `auth == SignedIn`) | `confirm_add()` spawns a background thread calling `add_award_to_user(username, &selected, &suffix, interactive_auth: false)` verbatim (research.md §5); `add_picker.submitting = true` disables the Confirm button and the filter/selection controls while in flight. |
| `AddDone { result }` where `result.ok` | The new award is appended to `looked_up.awards` (the display updates immediately, no re-sync needed); `add_picker` is closed (`None`); `status = result.message`. |
| `AddDone { result }` where `!result.ok` (e.g. `EditError::Stale`, `EditError::Conflict`) | `add_picker.submitting = false`; `status = result.message`; the picker stays open with the clerk's selection intact — spec Acceptance Scenario 2.4 requires the stale-write refusal to be a correctable moment, not a dead end that silently discards their in-progress choice. |
| Clerk closes the picker without confirming (a Cancel control, or navigating away) | `add_picker = None` — no write, no side effect, matching the TUI's Esc-cancel behavior's *outcome* (nothing happens) even though the literal keybinding doesn't apply to a window. |

## Out of scope for this contract

No Edit, Delete, Rename, Assist, Audit, or Paste surface of any kind (spec FR-010) — those remain
exclusively TUI/CLI capabilities until a later feature adds them to the GUI. No new CLI flags
(research.md §8). No new network endpoints beyond the three already-existing calls this feature
reuses (`build_awards_data`, `login`, `add_award_to_user`).
