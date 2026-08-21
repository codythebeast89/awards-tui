# Awards TUI

Terminal UI (Textual) to look up and edit awards in the [Decorations Database](https://docs.google.com/spreadsheets/d/1e_AqHIGrGdfNSgoHt6kLV89E6LADJmlZzhfRAUXo0wY) Google Sheet.

Tabs used:

- **Badges Database**
- **Ribbons Database**
- **Foreign Awards Database**

## Setup (write access)

A Google **API key alone cannot edit** the sheet. Use OAuth as your Logistics Clerk Google account (or a service account shared on the sheet).

1. In [Google Cloud Console](https://console.cloud.google.com/): create/select a project → enable **Google Sheets API**.
2. **APIs & Services → Credentials → Create credentials → OAuth client ID → Desktop app**.
3. Download the JSON and save it as `credentials.json` in this folder (see `credentials.example.json` for the expected shape).
4. Install deps and log in (browser opens; sign in as the clerk account that can edit the sheet):

```bash
cd awards-tui
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
python3 main.py --login
```

That writes `token.json` (gitignored). Check status anytime with `python3 main.py --auth-status`.

**Service account alternative:** place `service_account.json` in the project folder and share the Decorations spreadsheet with that service account email as Editor.

## Run

```bash
python3 main.py
```

If you created `.venv`, `python3 main.py` will use it automatically (needed for add/edit/delete and Textual). You can also activate it yourself: `source .venv/bin/activate`.

The interactive UI is a purple dark Textual layout: username bar, fixed **Actions** pane, tabbed **Awards** list, and a **Detail** pane.

### TUI keys

| Key | Action |
|-----|--------|
| Enter (in username) | Look up username |
| a | Add award for current user |
| e | Edit selected award cell |
| d | Delete selected award (type `delete` to confirm) |
| F5 / Ctrl+R | Refresh sheet data |
| Ctrl+Q | Quit |
| Esc | Cancel modal |

Use the left **Actions** pane or Detail buttons for the same operations. Award tabs: All / Badges / Ribbons / Foreign / Duplicates/Typos.

Duplicates and typos appear in **red** on the **All** tab (after normal awards) and again under the **Duplicates/Typos** tab. Similar-username rows can be edited or deleted; the confirm dialog names the **cell** username (the typo) so you do not remove the looked-up user by mistake.

Database-wide read-only scan (does not write to the sheet). Saves a clean report under `audits/` — also available from Actions → Audit:

```bash
python3 main.py --audit
# optional: python3 main.py --audit-out ~/Desktop/decorations-audit.txt
```

### One-shot CLI

```bash
python3 main.py SomeUsername
python3 main.py SomeUsername --add "Army Service Ribbon"
python3 main.py SomeUsername --add "Combat Action Badge" --suffix "x2"
```

## Notes

- Lookups use the public CSV export (no credentials required).
- Add/edit/delete use the Sheets API and require OAuth or a service account.
- Delete clears the award cell and **shifts that column up** so no blank hole is left (other award columns are unchanged).
- Award cells are written like `Username`, `Username x2`, or `Username - detail`.
- Sheet row numbers in the TUI include CSV→live offsets so they match Google Sheets: Badges **+6**, Ribbons **+8**, Foreign Awards **+7** at the top of each tab. Mid-sheet hidden/inserted rows can add extra lag; when you are logged in, lookup confirms each visible cell against the live sheet (so CSIB around row 4400 shows 4415, not 4413).
- **Do not commit** `credentials.json`, `token.json`, or `service_account.json`.

## Development

```bash
python3 test_awards.py
```

### Rust rewrite (in progress)

Cargo workspace under `crates/` (`awards-core`, `awards-sheets`, `awards-tui`). Python Textual remains available via `python3 main.py` until cutover.

```bash
cargo run -p awards-tui --release          # Ratatui TUI (default)
cargo run -p awards-tui --release -- SomeUser
cargo run -p awards-tui --release -- --audit
cargo run -p awards-tui --release -- --auth-status
```

Rust milestones: M0–M3 done (core, CSV/CLI, OAuth writes). **M4** Ratatui TUI matches the purple Actions / Awards tabs / Detail layout (a/e/d, F5, Ctrl+Q). Cutover is M5.

## License

MIT — see [LICENSE](LICENSE).
