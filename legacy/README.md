# Legacy Python awards-tui

Archived Textual / Python implementation (pre-v2). Prefer the Rust binary from the repo root:

```bash
cargo run -p awards-tui --release
```

## Run legacy UI

From the **repo root** (so `award_columns.json` and credentials resolve correctly):

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install -r legacy/requirements.txt
python3 legacy/main.py
python3 legacy/main.py --login
python3 legacy/main.py SomeUsername
python3 legacy/test_awards.py
```

Credentials and `award_columns.json` remain at the repository root (not inside `legacy/`).
