#!/usr/bin/env python3
"""Create/sync Proof - Iraq tab from campaign tracker doc with link chips.

User request: add Iraq tab with deployments; x1 used old 4-deployment cycle per
QMC Database Campaign Tracker (1/24/2025-5/23/2025). Read-only reference — no DB writes.

Callers: `python3 scripts/sync_proof_iraq.py`
API: Sheets batchUpdate (duplicateSheet + chipRuns) on USER_SHEET.
"""

from __future__ import annotations

import sys
from datetime import datetime, timedelta
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

from upgrade_qmc_tracker import batch_update, load_token, refresh_token, sheet_exists  # noqa: E402

SHEET_TITLE = "Proof - Iraq"
TEMPLATE_SHEET = "Proof - Kosovo"

# Source: https://docs.google.com/document/d/1YwjXQ-YW3i7qWx9EL3c6k13aKJD-8CAC4IpBwdAR5gE
# QMC Campaign Tracker (read-only): previous Iraq period 1/24/2025-5/23/2025 = 4 deployments/ribbon
IRAQ_LOGGED: list[tuple[str, str, str, str]] = [
    (
        "OPERATION Our Glorious King Polar Oppst (With Glaze On Top)",
        "Iraq Campaign x1",
        "2/1/2025",
        "https://docs.google.com/document/d/1e1TN6b23ZLBGo_Be5JfjfXPTlaL3_mFsvzHcwxCoGYI/edit",
    ),
    (
        'Operation "Something about roki being petite"',
        "Iraq Campaign x1",
        "2/2/2025",
        "https://docs.google.com/document/d/180TL7EGUWMVZHKbIVdmJzrGUo9ZNAEb4AQV5aPPerSk/edit",
    ),
    (
        "OPERATION SOLO OR MULTICREW",
        "Iraq Campaign x1",
        "2/23/2025",
        "https://docs.google.com/document/d/1MmF6jJCmX8Zv5Zr3cF1wFw11-lPDkS0TDY_kQcEE_V4/edit",
    ),
    (
        "OPERATION ''Hide in a bush and wait until the coast is clear then run''",
        "Iraq Campaign x1",
        "3/16/2025",
        "https://docs.google.com/document/d/1MAOQfNDfOrMEsDfSP3LN407Yq-7ACuS0LDFkuli_zw0/edit",
    ),
    (
        "OPERATION When Do I Get Out Of Incarceration?",
        "Iraq Campaign x2",
        "3/23/2025",
        "https://docs.google.com/document/d/1MTIZ06DePtVpNUqG3yNiUgTzxcxbVUYg7TOFCyEU1Z4/edit",
    ),
    (
        "OPERATION EASTER EGG",
        "Iraq Campaign x2",
        "4/22/2025",
        "https://docs.google.com/document/d/1XHQiDD1LEiPF6EEjexEPRzy0lNRblJ6AiJEQHdSyoKM/edit",
    ),
]

# x2 still on previous 4-deployment cycle (2 of 4 logged)
IRAQ_PENDING_X2 = 2


def deployment_week(date_str: str) -> str:
    dt = datetime.strptime(date_str, "%m/%d/%Y")
    days_since_sunday = (dt.weekday() + 1) % 7
    sunday = dt - timedelta(days=days_since_sunday)
    saturday = sunday + timedelta(days=6)

    def short(d: datetime) -> str:
        return f"{d.month}/{d.day}/{str(d.year)[2:]}"

    return f"{short(sunday)} - {short(saturday)}"


def plain(value: str) -> dict:
    return {"userEnteredValue": {"stringValue": value}}


def link_chip(uri: str) -> dict:
    return {
        "userEnteredValue": {"stringValue": "@"},
        "chipRuns": [
            {
                "chip": {
                    "richLinkProperties": {
                        "uri": uri,
                        "mimeType": "application/vnd.google-apps.kix",
                    }
                }
            }
        ],
    }


def list_sheets(tok: dict) -> dict[str, int]:
    import json
    import urllib.request

    from upgrade_qmc_tracker import USER_SHEET

    url = f"https://sheets.googleapis.com/v4/spreadsheets/{USER_SHEET}?fields=sheets.properties"
    req = urllib.request.Request(url, headers={"Authorization": f"Bearer {tok['token']}"})
    with urllib.request.urlopen(req, timeout=60) as resp:
        sheets = json.load(resp)["sheets"]
    return {s["properties"]["title"]: s["properties"]["sheetId"] for s in sheets}


def ensure_sheet(tok: dict) -> int:
    if sheet_exists(tok, SHEET_TITLE):
        return list_sheets(tok)[SHEET_TITLE]

    sheets = list_sheets(tok)
    template_id = sheets[TEMPLATE_SHEET]
    resp = batch_update(
        tok,
        [
            {
                "duplicateSheet": {
                    "sourceSheetId": template_id,
                    "insertSheetIndex": len(sheets),
                    "newSheetName": SHEET_TITLE,
                }
            }
        ],
        delay=0,
    )
    return resp["replies"][0]["duplicateSheet"]["properties"]["sheetId"]


def build_rows() -> list[list[dict]]:
    header = [
        plain("Name"),
        plain("Number"),
        plain("Date of Deployment"),
        plain("Week of Deployment"),
        plain("Status"),
        plain("Link"),
    ]
    rows = [header]
    for name, number, date, uri in IRAQ_LOGGED:
        rows.append(
            [
                plain(name),
                plain(number),
                plain(date),
                plain(deployment_week(date)),
                plain("Logged"),
                link_chip(uri),
            ]
        )
    for _ in range(IRAQ_PENDING_X2):
        rows.append(
            [
                plain(""),
                plain("Iraq Campaign x2"),
                plain(""),
                plain(""),
                plain("Pending"),
                plain(""),
            ]
        )
    return rows


def main() -> int:
    tok = refresh_token(load_token())
    scopes = tok.get("scopes") or []
    if "drive.readonly" not in " ".join(scopes):
        print("token.json needs drive.readonly scope for link chips.", file=sys.stderr)
        return 1

    sheet_id = ensure_sheet(tok)
    rows = build_rows()
    batch_update(
        tok,
        [
            {
                "updateCells": {
                    "range": {
                        "sheetId": sheet_id,
                        "startRowIndex": 0,
                        "endRowIndex": len(rows),
                        "startColumnIndex": 0,
                        "endColumnIndex": 6,
                    },
                    "rows": [{"values": row} for row in rows],
                    "fields": "userEnteredValue,chipRuns",
                }
            }
        ],
        delay=0,
    )
    logged = len(IRAQ_LOGGED)
    print(
        f"Updated {SHEET_TITLE}: {logged} logged "
        f"(x1=4 old-cycle, x2=2) + {IRAQ_PENDING_X2} pending x2 (link chips)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
