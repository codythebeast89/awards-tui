#!/usr/bin/env python3
"""Sync Proof - SWA Service tab from campaign tracker docs with link chips.

x1 + x2 use old 4-deployment cycle (QMC period 8/24/2024-12/24/2024).
x3 uses current 3-deployment cycle (QMC period 02/24/2026-05/24/2026).
Read-only DB reference — no DB writes.

Callers: `python3 scripts/sync_proof_swa.py`
API: Sheets batchUpdate (chipRuns) on USER_SHEET Proof - SWA Service tab.
"""

from __future__ import annotations

import json
import sys
import urllib.request
from datetime import datetime, timedelta
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

from upgrade_qmc_tracker import USER_SHEET, batch_update, load_token, refresh_token, sheet_exists  # noqa: E402

SHEET_TITLE = "Proof - SWA Service"

# x1: https://docs.google.com/document/d/1ovoTixs84PERBz5nIOTJ0TA_JD09Uc0L2I8IscADk0c
# x2/x3: https://docs.google.com/document/d/1mvt31J-QXsMyKTg3DJrDMg1yWaGirstnOp8mWecQd9U
SWA_LOGGED: list[tuple[str, str, str, str]] = [
    (
        "Operation Is MPC Silly or Locked in?",
        "Southwest Asia Service x1",
        "8/26/2024",
        "https://docs.google.com/document/d/1_BcF1f6mJde-B22P-TLfh3FbtQ8QvPawa3nqJsEaEp0/edit",
    ),
    (
        "Operation Infinite Operations, Operation, but no Operations",
        "Southwest Asia Service x1",
        "9/27/2024",
        "https://docs.google.com/document/d/1L7B2CSnv2j_qm6XEtT6Yz3ysMp0kiIUARe_6AxKoytU/edit",
    ),
    (
        "OPERATION: Ill Still Has Priority Candidate in QMC",
        "Southwest Asia Service x1",
        "10/12/2024",
        "https://docs.google.com/document/d/1FnlGAaA9xerOspCjqEeUp0oGj3gbyA82lzHECTg03aQ/edit",
    ),
    (
        "Operation This took Forever to Schedule",
        "Southwest Asia Service x1",
        "10/19/2024",
        "https://docs.google.com/document/d/1aXkb7b8iEIOJyPvpKNnM7ZQqs_Q015FrMT_gZmpDJHs/edit",
    ),
    (
        "OPERATION Uhhh, Idk, I'm bad at this",
        "Southwest Asia Service x2",
        "10/26/2024",
        "https://docs.google.com/document/d/1KF9hOwDyKLwPGXnPjYl8Qvp4Fh4_Eht38W7vwd8VKvQ/edit",
    ),
    (
        "OPERATION Mari just had to have USAF",
        "Southwest Asia Service x2",
        "11/8/2024",
        "https://docs.google.com/document/d/1g9VmctwXFSDud5JZ4dzRRl53p1c5NIKC56rn7sssISo/edit",
    ),
    (
        "OPERATION 3 weeks to schedule",
        "Southwest Asia Service x3",
        "4/5/2026",
        "https://docs.google.com/document/d/1A7khvUB9KDeKKVb6lZSPXLFgl1ic424Dg0rbXE0yz1A/edit",
    ),
]

SWA_PENDING_X2 = 2  # old 4-deployment cycle
SWA_PENDING_X3 = 2  # current 3-deployment cycle


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


def get_sheet_id(tok: dict) -> int:
    url = f"https://sheets.googleapis.com/v4/spreadsheets/{USER_SHEET}?fields=sheets.properties"
    req = urllib.request.Request(url, headers={"Authorization": f"Bearer {tok['token']}"})
    with urllib.request.urlopen(req, timeout=60) as resp:
        sheets = json.load(resp)["sheets"]
    for sheet in sheets:
        if sheet["properties"]["title"] == SHEET_TITLE:
            return sheet["properties"]["sheetId"]
    raise RuntimeError(f"Sheet not found: {SHEET_TITLE}")


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
    for name, number, date, uri in SWA_LOGGED:
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
    for _ in range(SWA_PENDING_X2):
        rows.append(
            [
                plain(""),
                plain("Southwest Asia Service x2"),
                plain(""),
                plain(""),
                plain("Pending"),
                plain(""),
            ]
        )
    for _ in range(SWA_PENDING_X3):
        rows.append(
            [
                plain(""),
                plain("Southwest Asia Service x3"),
                plain(""),
                plain(""),
                plain("Pending"),
                plain(""),
            ]
        )
    return rows


def clear_below(tok: dict, sheet_id: int, start_row: int) -> None:
    url = f"https://sheets.googleapis.com/v4/spreadsheets/{USER_SHEET}?fields=sheets.properties"
    req = urllib.request.Request(url, headers={"Authorization": f"Bearer {tok['token']}"})
    with urllib.request.urlopen(req, timeout=60) as resp:
        sheets = json.load(resp)["sheets"]
    row_count = next(
        s["properties"]["gridProperties"]["rowCount"]
        for s in sheets
        if s["properties"]["sheetId"] == sheet_id
    )
    if start_row >= row_count:
        return
    body = {"ranges": [f"'{SHEET_TITLE}'!A{start_row + 1}:F{row_count}"]}
    url = f"https://sheets.googleapis.com/v4/spreadsheets/{USER_SHEET}/values:batchClear"
    req = urllib.request.Request(
        url,
        data=json.dumps(body).encode(),
        method="POST",
        headers={"Authorization": f"Bearer {tok['token']}", "Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=60) as resp:
        json.load(resp)


def ensure_row_capacity(tok: dict, sheet_id: int, min_rows: int) -> None:
    url = f"https://sheets.googleapis.com/v4/spreadsheets/{USER_SHEET}?fields=sheets.properties"
    req = urllib.request.Request(url, headers={"Authorization": f"Bearer {tok['token']}"})
    with urllib.request.urlopen(req, timeout=60) as resp:
        sheets = json.load(resp)["sheets"]
    row_count = next(
        s["properties"]["gridProperties"]["rowCount"]
        for s in sheets
        if s["properties"]["sheetId"] == sheet_id
    )
    if row_count >= min_rows:
        return
    batch_update(
        tok,
        [
            {
                "updateSheetProperties": {
                    "properties": {"sheetId": sheet_id, "gridProperties": {"rowCount": min_rows}},
                    "fields": "gridProperties.rowCount",
                }
            }
        ],
        delay=0,
    )


def main() -> int:
    tok = refresh_token(load_token())
    if not sheet_exists(tok, SHEET_TITLE):
        print(f"Missing tab: {SHEET_TITLE}", file=sys.stderr)
        return 1
    scopes = tok.get("scopes") or []
    if "drive.readonly" not in " ".join(scopes):
        print("token.json needs drive.readonly scope for link chips.", file=sys.stderr)
        return 1

    sheet_id = get_sheet_id(tok)
    rows = build_rows()
    ensure_row_capacity(tok, sheet_id, len(rows))
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
    clear_below(tok, sheet_id, len(rows))
    logged = len(SWA_LOGGED)
    print(
        f"Updated {SHEET_TITLE}: {logged} logged "
        f"(x1=4, x2=2, x3=1) + {SWA_PENDING_X2} pending x2 + {SWA_PENDING_X3} pending x3 (link chips)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
