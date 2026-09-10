# Quickstart: Validating Discord Paste Quick-Add

Phase 1 output for `specs/003-discord-paste-quick-add/plan.md`. Run these after implementation to
confirm the feature works end-to-end, using the two real Discord messages gathered while writing
`spec.md`. This is a validation guide, not an implementation spec — see `data-model.md` and
`contracts/tui-paste-interaction.md` for the exact state machine and interaction rules.

## Prerequisites

- Built binary with OAuth already set up (`awards-tui --auth-status` shows logged in), same
  prerequisite as any other write-capable flow in this app.
- A terminal emulator with bracketed-paste support (the common case — iTerm2, Windows Terminal,
  GNOME Terminal/VTE, kitty, Alacritty, the VS Code integrated terminal, and most others all
  support it by default). See `research.md` §1 for the documented fallback in terminals that don't.
- The Decorations Database sheet reachable as usual (`awards-tui` run normally, not `--cli`).

## Scenario 1 — confident match, badge request (spec User Story 1, Acceptance Scenario 1)

1. Run `awards-tui`.
2. Press `p` (or select **Paste** from the Actions pane).
3. Paste this exact text (as it would arrive from Discord's "Copy Text" / manual forward):

   ```text
   ROBLOX Username: torba_f
   ROBLOX ID: 2452545815
   Current Division & Rank: 1ID, Colonel
   Badge Requested: Army Parachutist Badge
   Proof: [image attachment]
   ```

4. Press `Enter`.

**Expected**: the app looks up `torba_f` (no network round trip — instant), matches "Army
Parachutist Badge" against the Badges catalog, and lands directly in the Add flow's Suffix step
with that award already chosen and an empty suffix. Pressing `Enter` again writes the award for
`torba_f`, going through the same OAuth-gated write path as a manual Add.

## Scenario 2 — confident match with a repeat-count suffix, ribbon request (spec User Story 1,
Acceptance Scenario 3)

1. From a fresh state, press `p`.
2. Paste:

   ```text
   ROBLOX Username: Nevazaku_u
   ROBLOX ID: 1881585077
   Current Division & Rank: JFKSWCS, Brigadier General
   Ribbon Requested: Afganistan Campaign x1
   Proof: https://docs.google.com/spreadsheets/d/1Y8jEcLpRb6lDDhe7Axb5Z27dotsb3p-QKMtpvFATEjY/edit?gid=1708955154#gid=1708955154
   ```

3. Press `Enter`.

**Expected**: the app looks up `Nevazaku_u`, splits `"Afganistan Campaign x1"` into base text
`"Afganistan Campaign"` and suffix `"x1"`, matches the base text against the Ribbons catalog, and
lands in the Suffix step with that award chosen and the suffix field already pre-filled `x1`. The
Proof link is visible in the pasted text the clerk saw, but is never fetched or opened by the app
(confirm no outbound request to `docs.google.com` appears in any request logging/timeout path —
there is none, by design). Pressing `Enter` writes `Nevazaku_u x1` (or equivalent cell format) for
that award.

## Scenario 3 — unmatched award text (spec User Story 2, Acceptance Scenario 1)

1. Press `p`, paste a message with a valid `ROBLOX Username:` line but an award name that doesn't
   exist in the catalog (e.g. `Badge Requested: Not A Real Badge`), then `Enter`.

**Expected**: the app still looks the username up, then opens the normal Pick-step picker with the
filter box pre-filled `"Not A Real Badge"` (matching nothing, so the clerk sees the full catalog or
an empty filtered list to correct by hand) rather than silently guessing an award.

## Scenario 4 — missing/invalid username (spec User Story 2, Acceptance Scenario 2)

1. Press `p`, paste text with no `ROBLOX Username:`-style line at all (or a value that isn't a bare
   username, e.g. `Alice x2`), then `Enter`.

**Expected**: the Paste modal stays open with an inline error status; the buffer is not cleared, so
the clerk can correct the text and resubmit, or press `Esc` to cancel and fall back to the existing
manual Lookup + Add flow.

## Scenario 5 — totally unparseable paste (Edge Case)

1. Press `p`, paste unrelated text with none of the recognized labeled lines, then `Enter`.

**Expected**: same fallback as Scenario 4 — a generic "couldn't find a username" status, buffer
preserved, `Esc` still available to back out.

## Scenario 6 — Discord copy-paste artifacts and a bare "Username" label (Edge Case, FR-002)

1. Press `p`, paste:

   ```text
   **Username**: @torba_f
   Badge Requested: _Army Parachutist Badge_
   Proof: [image attachment]
   ```

2. Press `Enter`.

**Expected**: the bold markers, the leading `@`, and the italic markers around the award name are
all stripped before matching — the app resolves the same as Scenario 1 (`torba_f`, "Army
Parachutist Badge" pre-selected), confirming the bare `"Username"` label variant and Markdown/
mention-artifact stripping both work, not just the exact wording of the two original samples.

## Regression check — existing Add flow untouched

1. Look up a user manually (`Enter` in the username field), press `a`, and add an award by hand
   through the normal Pick → Suffix → Enter flow.

**Expected**: byte-for-byte the same behavior as before this feature — confirms the refactor of
`AddModal::reload`'s filter predicate into the shared `match_catalog_entries` (research.md §5)
didn't change any existing behavior.
