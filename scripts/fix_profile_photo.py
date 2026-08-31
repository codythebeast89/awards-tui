#!/usr/bin/env python3
"""Rebuild Profile without embedded overlay; service photo via IMAGE in merged C7:F22.

Callers: `python3 scripts/fix_profile_photo.py`
"""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

from copy_reference_tabs import PROFILE_UPDATES, USER_SHEET, api, delete_tab, load_token, move_tab_first  # noqa: E402
from upgrade_qmc_tracker import USERNAME, batch_update, cell  # noqa: E402

PHOTO = "https://raw.githubusercontent.com/codythebeast89/awards-tui/master/assets/cody_service_photo.png"


def build_profile_clean(tok: dict):
    delete_tab(tok, "Profile")
    resp = api(tok, "POST", ":batchUpdate", {"requests": [{"addSheet": {"properties": {"title": "Profile", "gridProperties": {"rowCount": 26, "columnCount": 13}}}}]})
    sid = resp["replies"][0]["addSheet"]["properties"]["sheetId"]
    title = f"{USERNAME} | Service Record File"

    batch_update(tok, [{"mergeCells": {"range": {"sheetId": sid, "startRowIndex": 4, "endRowIndex": 6, "startColumnIndex": 2, "endColumnIndex": 11}, "mergeType": "MERGE_ALL"}}])
    batch_update(tok, [{"mergeCells": {"range": {"sheetId": sid, "startRowIndex": 6, "endRowIndex": 22, "startColumnIndex": 2, "endColumnIndex": 6}, "mergeType": "MERGE_ALL"}}])
    batch_update(
        tok,
        [
            {
                "updateCells": {
                    "range": {"sheetId": sid, "startRowIndex": 4, "endRowIndex": 6, "startColumnIndex": 2, "endColumnIndex": 3},
                    "rows": [{"values": [cell(title, "#980000", "#f4cccc", True, 15)]}],
                    "fields": "userEnteredValue,userEnteredFormat",
                }
            }
        ],
    )
    batch_update(
        tok,
        [
            {
                "updateCells": {
                    "range": {"sheetId": sid, "startRowIndex": 6, "endRowIndex": 7, "startColumnIndex": 2, "endColumnIndex": 3},
                    "rows": [{"values": [cell(f'=IMAGE("{PHOTO}",1)', "#434343", formula=True)]}],
                    "fields": "userEnteredValue,userEnteredFormat",
                }
            }
        ],
    )

    data = [{"range": f"Profile!{c}", "values": [[v]]} for c, v in PROFILE_UPDATES.items()]
    api(tok, "POST", "/values:batchUpdate?valueInputOption=USER_ENTERED", {"data": data, "valueInputOption": "USER_ENTERED"})

    labels = [
        "Username", "Roblox ID", "Discord ID", "Rank", "Command", "Division",
        "Brigade/Battalion/Group", "Company", "Join Date", "Unit Time of Service", "Position", "Position Date of Hire",
    ]
    reqs = [
        {
            "updateCells": {
                "range": {"sheetId": sid, "startRowIndex": 2, "endRowIndex": 22, "startColumnIndex": 1, "endColumnIndex": 12},
                "rows": [{"values": [cell("", "#434343")] * 11} for _ in range(20)],
                "fields": "userEnteredFormat",
            }
        }
    ]
    for i, label in enumerate(labels):
        row = 6 + i
        reqs += [
            {"mergeCells": {"range": {"sheetId": sid, "startRowIndex": row, "endRowIndex": row + 1, "startColumnIndex": 6, "endColumnIndex": 8}, "mergeType": "MERGE_ALL"}},
            {"mergeCells": {"range": {"sheetId": sid, "startRowIndex": row, "endRowIndex": row + 1, "startColumnIndex": 8, "endColumnIndex": 11}, "mergeType": "MERGE_ALL"}},
            {
                "updateCells": {
                    "range": {"sheetId": sid, "startRowIndex": row, "endRowIndex": row + 1, "startColumnIndex": 6, "endColumnIndex": 7},
                    "rows": [{"values": [cell(label, "#999999")]}],
                    "fields": "userEnteredValue,userEnteredFormat",
                }
            },
        ]
    batch_update(tok, reqs)
    move_tab_first(tok, "Profile")


def main() -> int:
    tok = load_token()
    print("Rebuilding Profile with your service photo…")
    build_profile_clean(tok)
    print(f"Done: https://docs.google.com/spreadsheets/d/{USER_SHEET}/edit")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
