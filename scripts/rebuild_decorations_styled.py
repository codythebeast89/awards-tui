#!/usr/bin/env python3
"""Rebuild Decorations tabs with reference styling + user's awards via one batchUpdate.

Callers: `python3 scripts/rebuild_decorations_styled.py`
Reads: audits/image-map.json, upgrade_qmc_tracker LIVE_AWARDS
Writes: Decorations - Badges, Decorations - Ribbons on USER_SHEET
"""

from __future__ import annotations

import json
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

from copy_reference_tabs import USER_SHEET, api, delete_tab  # noqa: E402
from upgrade_qmc_tracker import IMAGE_MAP_PATH, LIVE_AWARDS, cell, find_image, load_token, refresh_token  # noqa: E402


def add_sheet(tok: dict, title: str) -> int:
    resp = api(tok, "POST", ":batchUpdate", {"requests": [{"addSheet": {"properties": {"title": title, "gridProperties": {"rowCount": 50, "columnCount": 20}}}}]})
    return resp["replies"][0]["addSheet"]["properties"]["sheetId"]

BADGE_SECTIONS = [
    ("Skill Badges", 2, [
        ("Group 1", [("Master Combat Action Badge", "3rd Award")]),
        ("Group 2", [("Expert Soldier Badge", "-")]),
        ("Group 3", [("Aviator Badge", "Basic")]),
        ("Group 5", [("Driver and Mechanic Badges", "Driver T, W & Operator")]),
    ]),
    ("Identification Badges", 5, [
        (None, [("Master Gunner Identification Badge", "Master")]),
        (None, [("Combat Service Identification Badge", "1CAV, NATO, Afghanistan, Kosovo, Sea Duty, MATCOM CSIB")]),
    ]),
    ("Skill Tabs", 8, [(None, [("Sapper Tab", "-")])]),
    ("Service Awards", 11, [(None, [("Overseas Bar", "x9")]), (None, [("Service Stripe", "x4")])]),
    ("Foreign Awards", 14, [(None, [("Queens Dedication Medal", "-")])]),
]


def batch(tok: dict, requests: list, delay: float = 2.0):
    if not requests:
        return
    time.sleep(delay)
    for i in range(0, len(requests), 40):
        api(tok, "POST", ":batchUpdate", {"requests": requests[i : i + 40]})


def img_cell(name: str, maps: dict) -> dict:
    url = find_image(name, maps)
    if url:
        return cell(f'=IMAGE("{url}",1)', formula=True)
    return cell("", "#cccccc")


def rebuild_badges(tok: dict, sid: int, maps: dict):
    reqs: list[dict] = []
    reqs.append(
        {
            "updateDimensionProperties": {
                "range": {"sheetId": sid, "dimension": "COLUMNS", "startIndex": 1, "endIndex": 18},
                "properties": {"pixelSize": 95},
                "fields": "pixelSize",
            }
        }
    )
    for r in range(2, 35):
        reqs.append(
            {
                "updateDimensionProperties": {
                    "range": {"sheetId": sid, "dimension": "ROWS", "startIndex": r, "endIndex": r + 1},
                    "properties": {"pixelSize": 28 if r in (6, 7) else 52},
                    "fields": "pixelSize",
                }
            }
        )

    placements: list[tuple[int, int, dict]] = []
    for c in range(1, 18):
        placements.append((2, c, cell("", "#434343")))
    placements.append((3, 2, cell("Badges", "#cc0000", size=20)))

    for title, col, groups in BADGE_SECTIONS:
        placements.append((6, col, cell(title, "#666666", "#b7b7b7", True)))
        row = 7
        for group_label, items in groups:
            if group_label:
                placements.append((row, col, cell(group_label, "#999999")))
                row += 1
            for name, device in items:
                placements += [(row, col, img_cell(name, maps)), (row, col + 1, cell(name, "#999999")), (row + 1, col + 1, cell(device, "#cccccc"))]
                row += 2

    min_r, max_r = min(p[0] for p in placements), max(p[0] for p in placements) + 1
    min_c, max_c = min(p[1] for p in placements), max(p[1] for p in placements) + 1
    by_row: dict[int, dict[int, dict]] = {}
    for r, c, cd in placements:
        by_row.setdefault(r, {})[c] = cd
    rows = [[by_row.get(r, {}).get(c, cell("")) for c in range(min_c, max_c)] for r in range(min_r, max_r)]

    reqs.append(
        {
            "updateCells": {
                "range": {"sheetId": sid, "startRowIndex": min_r, "endRowIndex": max_r, "startColumnIndex": min_c, "endColumnIndex": max_c},
                "rows": [{"values": r} for r in rows],
                "fields": "userEnteredValue,userEnteredFormat",
            }
        }
    )
    for _, col, _ in BADGE_SECTIONS:
        reqs.append({"mergeCells": {"range": {"sheetId": sid, "startRowIndex": 6, "endRowIndex": 7, "startColumnIndex": col, "endColumnIndex": col + 3}, "mergeType": "MERGE_ALL"}})
    reqs.append({"mergeCells": {"range": {"sheetId": sid, "startRowIndex": 3, "endRowIndex": 4, "startColumnIndex": 2, "endColumnIndex": 5}, "mergeType": "MERGE_ALL"}})
    batch(tok, reqs)


def rebuild_ribbons(tok: dict, sid: int, maps: dict):
    reqs: list[dict] = []
    reqs.append(
        {
            "updateDimensionProperties": {
                "range": {"sheetId": sid, "dimension": "COLUMNS", "startIndex": 1, "endIndex": 12},
                "properties": {"pixelSize": 100},
                "fields": "pixelSize",
            }
        }
    )
    for r in range(2, 28):
        reqs.append(
            {
                "updateDimensionProperties": {
                    "range": {"sheetId": sid, "dimension": "ROWS", "startIndex": r, "endIndex": r + 1},
                    "properties": {"pixelSize": 28 if r in (6, 7) else 48},
                    "fields": "pixelSize",
                }
            }
        )

    placements: list[tuple[int, int, dict]] = []
    for c in range(1, 12):
        placements.append((2, c, cell("", "#434343")))
    placements.append((3, 2, cell("Ribbons", "#e69138", size=20)))
    placements.append((6, 2, cell('=COUNTIF(Ribbons!D:D,"Obtained")&" Ribbons"', "#666666", "#b7b7b7", True, formula=True)))

    left, right = LIVE_AWARDS["ribbons"][:7], LIVE_AWARDS["ribbons"][7:]
    for i, (name, device) in enumerate(right):
        row = 7 + i * 2
        placements += [(row, 8, img_cell(name, maps)), (row, 9, cell(name, "#999999")), (row + 1, 9, cell(device or "-", "#cccccc"))]
    for i, (name, device) in enumerate(left):
        row = 9 + i * 2
        placements += [(row, 3, img_cell(name, maps)), (row, 4, cell(name, "#999999")), (row + 1, 4, cell(device or "-", "#cccccc"))]

    min_r, max_r = min(p[0] for p in placements), max(p[0] for p in placements) + 1
    min_c, max_c = min(p[1] for p in placements), max(p[1] for p in placements) + 1
    by_row: dict[int, dict[int, dict]] = {}
    for r, c, cd in placements:
        by_row.setdefault(r, {})[c] = cd
    rows = [[by_row.get(r, {}).get(c, cell("")) for c in range(min_c, max_c)] for r in range(min_r, max_r)]

    reqs.append(
        {
            "updateCells": {
                "range": {"sheetId": sid, "startRowIndex": min_r, "endRowIndex": max_r, "startColumnIndex": min_c, "endColumnIndex": max_c},
                "rows": [{"values": r} for r in rows],
                "fields": "userEnteredValue,userEnteredFormat",
            }
        }
    )
    reqs += [
        {"mergeCells": {"range": {"sheetId": sid, "startRowIndex": 3, "endRowIndex": 4, "startColumnIndex": 2, "endColumnIndex": 5}, "mergeType": "MERGE_ALL"}},
        {"mergeCells": {"range": {"sheetId": sid, "startRowIndex": 6, "endRowIndex": 7, "startColumnIndex": 2, "endColumnIndex": 4}, "mergeType": "MERGE_ALL"}},
    ]
    batch(tok, reqs)


def main() -> int:
    tok = refresh_token(load_token())
    maps = json.loads(IMAGE_MAP_PATH.read_text()) if IMAGE_MAP_PATH.is_file() else {"badges": {}, "ribbons": {}, "foreign": {}}

    print("Rebuilding Decorations - Badges…")
    delete_tab(tok, "Decorations - Badges")
    badges_id = add_sheet(tok, "Decorations - Badges")
    rebuild_badges(tok, badges_id, maps)

    print("Rebuilding Decorations - Ribbons…")
    delete_tab(tok, "Decorations - Ribbons")
    ribbons_id = add_sheet(tok, "Decorations - Ribbons")
    rebuild_ribbons(tok, ribbons_id, maps)

    print(f"Done: https://docs.google.com/spreadsheets/d/{USER_SHEET}/edit")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
