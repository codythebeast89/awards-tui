# Awards TUI

Terminal UI to look up and edit awards in the [Decorations Database](https://docs.google.com/spreadsheets/d/1e_AqHIGrGdfNSgoHt6kLV89E6LADJmlZzhfRAUXo0wY) Google Sheet.

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

If you created `.venv`, `python3 main.py` will use it automatically (needed for add/edit/delete). You can also activate it yourself: `source .venv/bin/activate`.

Type a username → **Enter**. **Tab** toggles search ↔ list focus.

### TUI keys

| Key | Action |
|-----|--------|
| Enter | Look up username |
| Tab | Toggle search ↔ list focus |
| a | Add award for current user |
| e | Edit selected award cell |
| d | Delete selected award (type `delete` to confirm) |
| ↑ / ↓ | Move selection / scroll |
| F5 / Ctrl+R | Refresh sheet data |
| Esc | Clear search / cancel modal / quit |
| q | Quit (search focus, empty input only) |

When you look up a user, duplicates and typos appear in a red **Duplicates / typos** section:
identical copies, conflicting details in the same column (for example CSIB units on separate rows), similar usernames, and malformed cells like `user- Master`. You can edit or delete those entries the same way as normal awards.

Database-wide read-only scan (does not write):

```bash
python3 main.py --audit
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
- Award cells are written like `Username`, `Username x2`, or `Username - detail`.
- Badges sheet row numbers in the TUI include a +6 offset so they match live Google Sheet rows (CSV export lags the sheet).
- **Do not commit** `credentials.json`, `token.json`, or `service_account.json`.

## Development

```bash
python3 test_awards.py
```

## License

MIT — see [LICENSE](LICENSE).
