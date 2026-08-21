# Awards TUI

Terminal UI to look up and edit awards in the [Decorations Database](https://docs.google.com/spreadsheets/d/1e_AqHIGrGdfNSgoHt6kLV89E6LADJmlZzhfRAUXo0wY) Google Sheet.

Tabs used: **Badges Database**, **Ribbons Database**, **Foreign Awards Database**.

**Primary entrypoint (v2):** Rust binary (`awards-tui`) — Ratatui purple layout with Actions / Awards tabs / Detail.

## Install

### From source

Requires a recent [Rust toolchain](https://rustup.rs/).

```bash
git clone https://github.com/codythebeast89/awards-tui.git
cd awards-tui
cargo install --path crates/awards-tui
```

Or run without installing: `cargo run -p awards-tui --release`

### cargo-binstall

After a `v*` GitHub release (Linux/macOS assets):

```bash
cargo binstall --git https://github.com/codythebeast89/awards-tui awards-tui
```

Release tarballs are named `awards-tui-<rust-target>.tar.gz` (see `[package.metadata.binstall]`).

### Homebrew (tap)

This repo includes `Formula/awards-tui.rb` so the GitHub repo can be used as a tap:

```bash
brew tap codythebeast89/awards-tui https://github.com/codythebeast89/awards-tui
brew install --HEAD codythebeast89/awards-tui/awards-tui
```

(`--HEAD` builds from `master` until a versioned bottle/source tarball is published.)

## Setup (write access)

A Google **API key alone cannot edit** the sheet. Use OAuth as your Logistics Clerk Google account (or a service account shared on the sheet).

1. In [Google Cloud Console](https://console.cloud.google.com/): create/select a project → enable **Google Sheets API**.
2. **APIs & Services → Credentials → Create credentials → OAuth client ID → Desktop app**.
3. Download the JSON and save it as `credentials.json` in this folder (see `credentials.example.json`).
4. Log in (browser opens):

```bash
awards-tui --login
# or: cargo run -p awards-tui --release -- --login
```

That writes `token.json` (gitignored). Check status with `awards-tui --auth-status`.

**Service account alternative:** place `service_account.json` in the project folder and share the Decorations spreadsheet with that service account email as Editor.

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
| e | Edit selected award cell |
| d | Delete selected award (type `delete` to confirm) |
| F5 / Ctrl+R | Refresh sheet data |
| Tab | Cycle focus |
| Ctrl+Q | Quit |
| Esc | Cancel modal / close audit browser |

Actions → **Audit** opens an in-app report browser (also writes `audits/audit-*.txt`). In the browser: ↑/↓ or j/k, PgUp/PgDn, Home/End, Esc.

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
awards-tui --login
awards-tui --auth-status
```

## Notes

- Lookups use the public CSV export (no credentials required).
- Add/edit/delete use the Sheets API and require OAuth or a service account.
- Delete clears the award cell and **shifts that column up** so no blank hole is left.
- Award cells are written like `Username`, `Username x2`, or `Username - detail`.
- Sheet row numbers include CSV→live offsets: Badges **+6**, Ribbons **+8**, Foreign Awards **+7**. When logged in, lookup reconciles mid-sheet lag against the live sheet.
- Shared config at repo root: `award_columns.json`, `credentials.json`, `token.json`.
- **Do not commit** `credentials.json`, `token.json`, or `service_account.json`.

## Development

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

## Legacy Python

The previous Textual / Python implementation lives under [`legacy/`](legacy/). Prefer the Rust binary; see [legacy/README.md](legacy/README.md) if you still need it.

## License

MIT — see [LICENSE](LICENSE).
