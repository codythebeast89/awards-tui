# Awards TUI

Terminal UI to look up and edit awards in the [Decorations Database](https://docs.google.com/spreadsheets/d/1e_AqHIGrGdfNSgoHt6kLV89E6LADJmlZzhfRAUXo0wY) Google Sheet.

Tabs used: **Badges Database**, **Ribbons Database**, **Foreign Awards Database**.

**Primary entrypoint (v2):** Rust binary (`awards-tui`) — Ratatui purple layout with Actions / Awards tabs / Detail.

## Install

This is how you get the program onto your computer. It's a little like installing an app on your
phone, except we use a black window with text instead of tapping icons. That black window is
called a **"terminal"** (on a Mac, press Cmd+Space, type `Terminal`, press Enter; on Windows, search
the Start Menu for `PowerShell`). You'll copy a line of text, paste it in, and press Enter. Do that
for each line, one at a time, waiting for each one to finish before pasting the next.

You only need to pick **one** of the three options below — do not do all three.

### Option 1 (easiest if you have a Mac): Homebrew

1. If you don't already have "Homebrew" on your Mac, go to [brew.sh](https://brew.sh) and copy-paste
   the one install command shown there into your terminal. Press Enter and wait for it to finish.
2. Then paste these two lines into your terminal, one at a time:

```bash
brew tap codythebeast89/awards-tui https://github.com/codythebeast89/awards-tui
brew install codythebeast89/awards-tui/awards-tui
```

That's it — the program is now installed.

### Option 2 (Linux or Mac, no Homebrew): cargo-binstall

This downloads a ready-made copy of the program instead of building it yourself.

1. Install "cargo" first by following the instructions at [rustup.rs](https://rustup.rs/) (copy-paste
   the one command shown there).
2. Then paste this into your terminal:

```bash
cargo binstall --git https://github.com/codythebeast89/awards-tui awards-tui
```

If it asks "Do you want to continue?", type `y` and press Enter.

### Option 3 (for developers, or if the above don't work): build it yourself

This takes longer because your computer builds the program from scratch instead of downloading a
finished copy.

1. Install Rust by following the instructions at [rustup.rs](https://rustup.rs/).
2. Paste these lines into your terminal, one at a time:

```bash
git clone https://github.com/codythebeast89/awards-tui.git
cd awards-tui
cargo install --path crates/awards-tui
```

Prefer to just try it once without installing it permanently? Run `cargo run -p awards-tui --release`
instead of the last line above.

## Setup (only needed if you are a Logistics Clerk)

Looking up awards works immediately after install — no setup needed for that. The steps below are
only for clerks who need to **write** changes back to the Google Sheet. It's a one-time setup.

Skip this entirely if you only ever want to look someone up.

Here's the short version of what you're about to do: you're going to tell Google "it's okay for this
program to edit my sheet, using my own account" — the same way a website asks "Sign in with Google."

1. Open [Google Cloud Console](https://console.cloud.google.com/) in your web browser and sign in
   with the same Google account you use for the Decorations Database sheet.
2. At the top of the page, click to create a new project (or pick an existing one — any name is
   fine). This is just Google's way of grouping things; you won't see it again after this.
3. In the search bar at the top, type `Google Sheets API`, click the result, then click the blue
   **Enable** button.
4. On the left-hand menu, click **Credentials**. Click **+ Create Credentials** near the top, then
   choose **OAuth client ID**.
   - If it asks you to configure a "consent screen" first, pick **External**, fill in only the
     required fields (an app name and your email are enough), and save.
   - For "Application type," choose **Desktop app**. Give it any name. Click **Create**.
5. A box will pop up with a **Download JSON** button. Click it — this saves a small file to your
   computer's Downloads folder.
6. Rename that downloaded file to exactly `credentials.json` and move it into this project's folder
   (the same folder this `README.md` is in). There's an example file, `credentials.example.json`,
   in that same folder if you want to see what it should look like.
7. Back in your terminal, type:

```bash
awards-tui --login
```

   Your web browser will open and ask you to sign in and approve access — this is normal and expected,
   it's the same "it's okay for this program to edit my sheet" step mentioned above. Approve it.

That's it — you're done. The program saves a `token.json` file so you won't have to repeat this
step again on this computer. You can check it worked by typing `awards-tui --auth-status`.

**Something went wrong?** Delete `token.json` and run `awards-tui --login` again.

## Run

```bash
awards-tui
```

Purple dark layout: username bar, fixed **Actions** pane, tabbed **Awards** list, **Detail** pane.

### TUI keys

| Key | Action |
|-----|--------|
| Enter (in username) | Look up username |
| a | Add award for current user |
| p / Paste | Paste a forwarded Discord request (badge/ribbon) and pre-fill Add from it |
| e | Edit selected award cell |
| d | Delete selected award (type `delete` to confirm) |
| n | Rename current user across all sheet cells (type `rename` to confirm) |
| c / Assist | Clerk assist: check award request (MCAB/MCIB/MCMB); Enter to grant if eligible |
| ? | Show the full keybinding reference screen |
| F5 / Ctrl+R | Refresh sheet data |
| Tab | Cycle focus |
| Ctrl+Q | Quit |
| Esc | Cancel modal / close audit browser |

Actions → **Paste** opens a buffer for a Logistics Clerk to paste the raw text of a badge/ribbon
request manually forwarded from Discord (e.g. `ROBLOX Username: ...` / `Badge Requested: ...`).
Press Enter to submit: a recognized username and a confidently-matched award land you straight in
the existing Add flow's award-detail step, pre-filled (including a trailing repeat/count indicator
like `x1`); an unmatched or ambiguous award instead opens the normal searchable award picker
pre-filled with that text; a missing or unrecognized username keeps the paste buffer open with an
inline message so you can correct and resubmit. This never fetches, opens, or otherwise processes
whatever the pasted text names as proof — you still check that yourself, exactly as before — and
every write still goes through the same OAuth-gated path the manual Add flow already uses.

Actions → **Audit** opens a selectable findings list (also writes the plain-text report to
`audits/audit-*.txt`, unchanged, for record-keeping). Findings are grouped by the username
involved. In the findings list: ↑/↓ or j/k, PgUp/PgDn to move, **Enter** to jump straight into
the matching fix (Delete for a duplicate row, Edit for a malformed cell, or a 1/2 username choice
then Rename for similar usernames — an unparseable cell has no direct fix and stays listed for
manual review), **Tab** to switch to the plain-text report view, Esc to close. In the report view:
↑/↓ or j/k, PgUp/PgDn, Home/End, Tab back to the findings list, Esc to close. Fixing a finding
writes through the same OAuth-gated live-sheet path as the regular Edit/Delete/Rename actions and
re-checks the audit locally afterward, so a resolved finding drops off the list without a manual
re-run; Esc while fixing a finding returns to the findings list instead of closing everything.

Award tabs: All / Badges / Ribbons / Foreign / Duplicates/Typos. Duplicates and typos appear in **red** on **All**.

### Theme / config

Optional TOML (see `awards-tui.example.toml`). First match wins:

1. `AWARDS_TUI_CONFIG`
2. `./awards-tui.toml`
3. `~/.config/awards-tui/config.toml`

```toml
[theme]
bg = "#0c0c0f"
purple = "#a78bfa"
dup = "#f87171"
```

### CLI

```bash
awards-tui SomeUsername
awards-tui --audit
awards-tui --audit-out ~/Desktop/decorations-audit.txt
awards-tui SomeUsername --add "Army Service" --suffix "x2"
awards-tui SomeUsername --edit "Army Service" --cell "SomeUsername x2"
awards-tui SomeUsername --delete "Army Service"
awards-tui OldUsername --rename NewUsername
awards-tui MaoZhuuDong --check MCAB
awards-tui MaoZhuuDong --grant MCAB
awards-tui SomeUser --check MCIB
awards-tui SomeUser --grant MCMB
awards-tui --login
awards-tui --auth-status
```

## Notes

- Lookups use the public CSV export (no credentials required).
- Add/edit/delete/rename use the Sheets API and require OAuth or a service account.
- **Clerk Assist** (`--check` / `--grant`, or Actions → Assist / `c`): eligibility for master combat badges (**MCAB**←ESB+CAB, **MCIB**←EIB+CIB, **MCMB**←EFMB+CMB). Prints Discord reminders; `--grant` / Enter writes the sheet. You still forward/deny in Discord yourself.
- **Rename** rewrites every cell for a user (`OldName --rename NewName` or Actions → Rename / `n`), keeping suffixes like `x2` and `- detail`. The new name must be a bare Roblox username. Overlapping columns on the live sheet are refused.
- Delete clears the award cell and **shifts that column up** so no blank hole is left.
- Award cells are written like `Username`, `Username x2`, or `Username - detail`.
- Sheet row numbers include CSV→live offsets: Badges **+6**, Ribbons **+8**, Foreign Awards **+7**. When logged in, lookup reconciles mid-sheet lag against the live sheet.
- Shared config: `award_columns.json`, `credentials.json`, `token.json` (repo cwd, `AWARDS_ROOT`, or `~/.config/awards-tui/`).
- **Do not commit** `credentials.json`, `token.json`, or `service_account.json`.

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md) for test/clippy expectations and GPG-signed commit setup.

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

## Legacy Python

The previous Textual / Python implementation lives under [`legacy/`](legacy/). Prefer the Rust binary; see [legacy/README.md](legacy/README.md) if you still need it.

## License

MIT — see [LICENSE](LICENSE).
