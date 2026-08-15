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

Use the left **Actions** pane or Detail buttons for the same operations. Award tabs: All / Badges / Ribbons / Foreign / Duplicates.

Duplicates and typos appear in **red** on the **All** tab (after normal awards) and again under the **Duplicates** tab.

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
- Sheet row numbers in the TUI include CSV→live offsets so they match Google Sheets: Badges **+6**, Ribbons **+8**, Foreign Awards **+7** (public CSV export lags the live sheet).
- **Do not commit** `credentials.json`, `token.json`, or `service_account.json`.

## Development

```bash
python3 test_awards.py
```

## License

MIT — see [LICENSE](LICENSE).
