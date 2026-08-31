#!/usr/bin/env python3
"""Sync Proof - Kosovo tab from campaign tracker doc with Drive link chips.

User request: fix Kosovo tab with deployments/dates/links; links must be chips.

Callers: `python3 scripts/sync_proof_kosovo.py`
API: Sheets batchUpdate (chipRuns) on USER_SHEET Proof - Kosovo tab.
Requires token.json with spreadsheets + drive.readonly scopes (awards-tui --login).
"""

from __future__ import annotations

import json
import sys
import urllib.request
from datetime import datetime, timedelta
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

from upgrade_qmc_tracker import USER_SHEET, batch_update, load_token, refresh_token  # noqa: E402

# Source: https://docs.google.com/document/d/16n9cz7e3hN6dQGMhL8lA8Jab72WI3DSOrVdRwJAN7Bo
KOSOVO_DEPLOYMENTS: list[tuple[str, str, str, str]] = [
    (
        "OPERATION I NEED KOSOVO CAMPAIGN RIBBON",
        "Kosovo Campaign x1",
        "8/31/2025",
        "https://docs.google.com/document/d/1CqsukAyY-PEun3_EkGp4W1hmdmpunvE5YIATe0YbWRs/edit",
    ),
    (
        "OPERATION OVERINTURD",
        "Kosovo Campaign x1",
        "9/7/2025",
        "https://docs.google.com/document/d/1Wv8J0lx3H7-5yTMVMCGcg6LplPPi02dRqfKI0yXCHwM/edit",
    ),
    (
        "OPERATION BUTT TICKLING BANDIT",
        "Kosovo Campaign x1",
        "9/14/2025",
        "https://docs.google.com/document/d/1afiM-_DNys8cO_uu5ylsX47mDdpJ19v5DqA2h24TkIE/edit",
    ),
    (
        "Operation duvall needs to lock in",
        "Kosovo Campaign x2",
        "9/28/2025",
        "https://docs.google.com/document/d/1ptpWBxp5luzSCUrFtdDGW5aq9F4j_XFrK2__FEFLyag/edit",
    ),
    (
        "OPERATION I Lego HICOM",
        "Kosovo Campaign x2",
        "10/9/2025",
        "https://docs.google.com/document/d/1OQF4Ef_mQSebZX084EbCn2fHrI4wjYJgWwoRqx_mD9c/edit",
    ),
    (
        "OPERATION Bahay Kubo",
        "Kosovo Campaign x2",
        "10/12/2025",
        "https://docs.google.com/document/d/1CwudTowIQeqehv086aESi8wefvYXjsmBnLrEZUHh74U/edit",
    ),
    (
        "Operation Steadfast",
        "Kosovo Campaign x3",
        "10/25/2025",
        "https://docs.google.com/document/d/1xJUrq0IawYlePnnG5qC6nb7C_9zFVPWrUFCEc-FiPQ4/edit",
    ),
    (
        "OPERATION No More Femboys",
        "Kosovo Campaign x3",
        "11/21/2025",
        "https://docs.google.com/document/d/1RHHCZHapZ6JlP-W_pzhFJeZuVGXw0o1mZ8kTTjKnm7U/edit",
    ),
    (
        "OPERATION Khaby Lame Mechanism",
        "Kosovo Campaign x3",
        "11/15/2025",
        "https://docs.google.com/document/d/1dKPiLBs83ubYbKrHIber3p7jo8wi6n6o5w6CLL_QFB4/edit",
    ),
]


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


def get_sheet_id(tok: dict, title: str) -> int:
    url = f"https://sheets.googleapis.com/v4/spreadsheets/{USER_SHEET}?fields=sheets.properties"
    req = urllib.request.Request(url, headers={"Authorization": f"Bearer {tok['token']}"})
    with urllib.request.urlopen(req, timeout=60) as resp:
        sheets = json.load(resp)["sheets"]
    for sheet in sheets:
        if sheet["properties"]["title"] == title:
            return sheet["properties"]["sheetId"]
    raise RuntimeError(f"Sheet not found: {title}")


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
    for name, number, date, uri in KOSOVO_DEPLOYMENTS:
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
    return rows


def main() -> int:
    tok = refresh_token(load_token())
    scopes = tok.get("scopes") or []
    if "drive.readonly" not in " ".join(scopes):
        print(
            "token.json needs drive.readonly scope for link chips.\n"
            "Re-login: .venv/bin/python -c \"from google_auth_oauthlib.flow import InstalledAppFlow; "
            "from pathlib import Path; "
            "SCOPES=['https://www.googleapis.com/auth/spreadsheets', "
            "'https://www.googleapis.com/auth/drive.readonly']; "
            "creds=InstalledAppFlow.from_client_secrets_file('credentials.json', SCOPES)"
            ".run_local_server(port=0); Path('token.json').write_text(creds.to_json())\"",
            file=sys.stderr,
        )
        return 1

    sheet_id = get_sheet_id(tok, "Proof - Kosovo")
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
    print(f"Updated Proof - Kosovo with {len(rows) - 1} deployments (link chips)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
