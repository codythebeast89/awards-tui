#!/usr/bin/env python3
"""Copy styled tabs from reference spreadsheet into user tracker via Sheets API copyTo.

Preserves images, merges, colors — unlike programmatic rebuild.

Usage:
  python3 scripts/copy_reference_tabs.py
  python3 scripts/copy_reference_tabs.py --profile-only
"""

from __future__ import annotations

import argparse
import json
import sys
import time
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
from upgrade_qmc_tracker import ROBLOX_ID, USERNAME, load_token, refresh_token  # noqa: E402

REF_SHEET = "1Qb3cwzozoGmSi6speJLPeac1vRhApH2utpfeGJW3wBk"
USER_SHEET = "1RayD8PRCVwut5gRG3_awt3HcWBKMH3lIker09dAMBYI"

REF_TABS = {
    "Profile": 1587421901,
    "Decorations - Badges": 913183075,
    "Decorations - Ribbons": 195347084,
}

PROFILE_UPDATES = {
    "C5": f"{USERNAME} | Service Record File",
    "I7": USERNAME,
    "I8": ROBLOX_ID,
    "I9": "",
    "I10": "",
    "I11": "Forces Command",
    "I12": "",
    "I13": "",
    "I14": "",
    "I15": "",
    "I16": "",  # clear bogus TOS until join date set
    "I17": "",
    "I18": "",
}


def api(tok: dict, method: str, path: str, body: dict | None = None, sheet_id: str = USER_SHEET) -> dict:
    tok = refresh_token(tok)
    url = f"https://sheets.googleapis.com/v4/spreadsheets/{sheet_id}{path}"
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(
        url,
        data=data,
        method=method,
        headers={"Authorization": f"Bearer {tok['token']}", "Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=120) as resp:
        return json.load(resp)


def list_sheets(tok: dict, spreadsheet_id: str) -> dict[str, int]:
    meta = api(tok, "GET", "?fields=sheets(properties(sheetId,title))", sheet_id=spreadsheet_id)
    return {s["properties"]["title"]: s["properties"]["sheetId"] for s in meta.get("sheets", [])}


def delete_tab(tok: dict, title: str) -> bool:
    sheets = list_sheets(tok, USER_SHEET)
    if title not in sheets:
        return False
    api(tok, "POST", ":batchUpdate", {"requests": [{"deleteSheet": {"sheetId": sheets[title]}}]})
    time.sleep(2)
    return True


def copy_tab(tok: dict, ref_sheet_id: int, new_title: str | None = None) -> int:
    resp = api(
        tok,
        "POST",
        f"/sheets/{ref_sheet_id}:copyTo",
        {"destinationSpreadsheetId": USER_SHEET},
        sheet_id=REF_SHEET,
    )
    new_id = resp["sheetId"]
    if new_title:
        api(
            tok,
            "POST",
            ":batchUpdate",
            {"requests": [{"updateSheetProperties": {"properties": {"sheetId": new_id, "title": new_title}, "fields": "title"}}]},
        )
    time.sleep(2)
    return new_id


def move_tab_first(tok: dict, title: str):
    sheets = list_sheets(tok, USER_SHEET)
    if title not in sheets:
        return
    api(
        tok,
        "POST",
        ":batchUpdate",
        {
            "requests": [
                {
                    "updateSheetProperties": {
                        "properties": {"sheetId": sheets[title], "index": 0},
                        "fields": "index",
                    }
                }
            ]
        },
    )


def update_profile(tok: dict, service_photo_url: str | None = None):
    data = [{"range": f"Profile!{cell}", "values": [[value]]} for cell, value in PROFILE_UPDATES.items()]
    if service_photo_url:
        data.append({"range": "Profile!B6", "values": [[f'=IMAGE("{service_photo_url}",1)']]})
    api(tok, "POST", "/values:batchUpdate?valueInputOption=USER_ENTERED", {"data": data, "valueInputOption": "USER_ENTERED"})


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--profile-only", action="store_true")
    parser.add_argument("--skip-decorations", action="store_true")
    args = parser.parse_args()

    if not (ROOT / "token.json").is_file():
        print("Run awards-tui --login first", file=sys.stderr)
        return 1

    tok = load_token()
    tabs_to_copy = ["Profile"]
    if not args.profile_only and not args.skip_decorations:
        tabs_to_copy += ["Decorations - Badges", "Decorations - Ribbons"]

    print("Copying reference tabs into your tracker…")
    for title in tabs_to_copy:
        print(f"  • {title}")
        delete_tab(tok, title)
        copy_tab(tok, REF_TABS[title], title)

    print("Updating Profile with your info…")
    update_profile(tok)
    print("Moving Profile to first tab…")
    move_tab_first(tok, "Profile")

    print(f"\nDone: https://docs.google.com/spreadsheets/d/{USER_SHEET}/edit")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
