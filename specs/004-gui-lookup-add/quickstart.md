# Quickstart: GUI Lookup & Add

Manual validation scenarios for `awards-gui`, to be run against a real window on a machine with a
display (this feature was built and automated-tested in a headless environment — see
`plan.md`'s Testing section and research.md §7 for why a live run isn't possible there). Run these
after `cargo build -p awards-gui --release` from the repo root, from a directory with valid
credentials configured (same setup `awards-tui`'s README already documents).

## Prerequisites

- A working `awards-tui` setup already exists (credentials resolved via `AWARDS_ROOT` / cwd /
  `~/.config/awards-tui/`), since `awards-gui` reuses it unchanged.
- A test username known to exist in the Decorations Database with at least one award, and at least
  one award the test user does *not* already have (for the Add scenarios).
- To exercise the Sign In scenarios from a signed-out state, temporarily rename `token.json` out of
  the way (or use a scratch `AWARDS_ROOT` with no `token.json`) — do not delete a real one.

## Scenario 1 — Look up a known user

1. Run `cargo run -p awards-gui --release`.
2. Wait for the initial sync to finish (a "Syncing..." indicator should disappear).
3. Enter the test username and submit.
4. **Expected**: the user's current awards appear, grouped by Badges / Ribbons / Foreign Awards,
   matching what `awards-tui <username>` (or the TUI's own Lookup) shows for the same user.

## Scenario 2 — Look up a username with no records

1. Enter a username known not to exist in the sheet and submit.
2. **Expected**: a plain "no records found" message — never an empty, unexplained list (spec
   FR-008).

## Scenario 3 — Sign in from inside the GUI

1. With no usable session (see Prerequisites), confirm the Sign In control is visible and the Add
   control is either hidden or clearly marked as requiring sign-in.
2. Click Sign In.
3. **Expected**: the OS default browser opens to the Google OAuth consent screen (same page
   `awards-tui --login` already opens); after granting access, the GUI window itself reports a
   success status and the Sign In control disappears — no terminal interaction was needed at any
   point (spec SC-002).

## Scenario 4 — Add an award end-to-end

1. Look up the test user (Scenario 1).
2. Click Add Award; confirm the picker opens with the catalog minus whatever this user already
   holds.
3. Type part of a known award's name into the picker's search box.
4. **Expected**: the list narrows to matching awards, the same way the TUI's own Add picker filter
   already behaves.
5. Select the award, optionally enter a suffix (e.g. `x1`), and click Confirm.
6. **Expected**: the award appears in the user's displayed award list immediately, and checking the
   live Google Sheet (or re-running `awards-tui <username>`) confirms it was written exactly as a
   manual TUI Add would produce — same cell format, same location.

## Scenario 5 — Stale-write conflict

1. Look up the test user and open the Add picker.
2. In another window/session, manually write something into the exact cell this add would target
   (or have another clerk add an award to the same user concurrently).
3. Select an award and click Confirm.
4. **Expected**: the write is refused with a clear "stale, refresh and retry" message (the existing
   `add_award_to_user` re-check, unchanged); the picker stays open with the selection intact rather
   than silently discarding it or silently overwriting the sheet (spec Acceptance Scenario 2.4).

## Scenario 6 — Nothing left to add

1. Look up a test user who already holds every award in the catalog (or a small scratch catalog
   for this test).
2. Click Add Award.
3. **Expected**: a "no remaining awards to add for this user" message, not an empty picker.

## Regression check — existing TUI/CLI untouched

1. Run `cargo test --workspace --locked` and `cargo clippy --workspace --locked -- -D warnings`
   (and `--all-targets`) — all pre-existing tests across `awards-core`, `awards-sheets`, and
   `awards-tui` must still pass unchanged; this feature adds a new crate and new tests, and
   modifies no existing file's behavior.
2. Run `awards-tui <same test username>` from the CLI and confirm it shows the exact same awards
   the GUI showed in Scenario 1, including the one added in Scenario 4.
