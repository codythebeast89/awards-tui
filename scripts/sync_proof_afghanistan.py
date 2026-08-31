#!/usr/bin/env python3
"""Create/sync Proof - Afghanistan tab from campaign tracker doc with link chips.

User request: make a tab for Afghanistan Campaign and add proof from tracker doc.

Callers: `python3 scripts/sync_proof_afghanistan.py`
API: Sheets batchUpdate (duplicateSheet + chipRuns) on USER_SHEET.
"""

from __future__ import annotations

import json
import sys
import urllib.request
from datetime import datetime, timedelta
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

from upgrade_qmc_tracker import (  # noqa: E402
    USER_SHEET,
    add_sheet,
    batch_update,
    load_token,
    refresh_token,
    sheet_exists,
)

SHEET_TITLE = "Proof - Afghanistan"
TEMPLATE_SHEET = "Proof - Kosovo"

# Source: https://docs.google.com/document/d/1CFDQea733bsMvPydje-L1XDimNYfXH7lq2I0tkx3SJQ
AFGHANISTAN_DEPLOYMENTS: list[tuple[str, str, str, str]] = [
    (
        "OPERATION THEY'RE IN THE TREES",
        "Afghanistan Campaign x1",
        "5/24/2025",
        "https://docs.google.com/document/d/1f_NxA4uYtARCmxjkcNRyvhJYjf_EAGOeFsna5-tJYNU/edit",
    ),
    (
        "Operation Friendly Friction",
        "Afghanistan Campaign x1",
        "5/30/2025",
        "https://docs.google.com/document/d/1r6ic3CTBzRTWdUiCHIylOrwnX6wX9JQ5mihXZX2V5Ec/edit",
    ),
    (
        "OPERATION Candy Blossom",
        "Afghanistan Campaign x1",
        "6/7/2025",
        "https://docs.google.com/document/d/14MYsJQ-K9fvwm9PJFPHhor0Io6A7oAueggGyv0Hfxfg/edit",
    ),
    (
        "OPERATION General Creighton Abrams",
        "Afghanistan Campaign x2",
        "6/8/2025",
        "https://docs.google.com/document/d/1CMpbdsjFpsLUubz2KdwUcai0k3_8sBHfBymbcogEI0g/edit",
    ),
    (
        "Operation Grow a Garden Crashes Roblox",
        "Afghanistan Campaign x2",
        "6/21/2025",
        "https://docs.google.com/document/d/1aKEQQyYDBa0YRO6Z8_4VOBu7HpNOBHyg5I5Fa9XxPXg/edit",
    ),
    (
        "OPERATION OVERIN LOST HIS MEDS",
        "Afghanistan Campaign x2",
        "6/22/2025",
        "https://docs.google.com/document/d/1gSm_IrBzClLrpGqdSXyJIinbjGdAejXiTzJ2ZcBv9zg/edit",
    ),
]

PENDING_X3 = 3


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


def empty_link() -> dict:
    return {"userEnteredValue": {"stringValue": ""}}


def list_sheets(tok: dict) -> dict[str, int]:
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
    for name, number, date, uri in AFGHANISTAN_DEPLOYMENTS:
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
    for _ in range(PENDING_X3):
        rows.append(
            [
                plain(""),
                plain("Afghanistan Campaign x3"),
                plain(""),
                plain(""),
                plain("Pending"),
                empty_link(),
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
    logged = len(AFGHANISTAN_DEPLOYMENTS)
    print(f"Updated {SHEET_TITLE}: {logged} logged + {PENDING_X3} pending x3 (link chips)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
