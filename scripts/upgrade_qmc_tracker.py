#!/usr/bin/env python3
"""Apply QMC tracker upgrades via Google Sheets API using awards-tui OAuth.

User verbatim: "Start with 1 then grab my awards with the tui then work on the rest"

Callers: manual CLI only (`python3 scripts/upgrade_qmc_tracker.py`).
API: Google Sheets API v4 batchUpdate on spreadsheet 1RayD8PRCVwut5gRG3_awt3HcWBKMH3lIker09dAMBYI.
Schema: reads audits/image-map.json; writes Profile, Badges, Ribbons tabs + new decoration sheets.
"""

from __future__ import annotations

import json
import re
import sys
import urllib.parse
import urllib.request
from datetime import datetime, timedelta, timezone
from pathlib import Path
import time

ROOT = Path(__file__).resolve().parents[1]
TOKEN_PATH = ROOT / "token.json"
IMAGE_MAP_PATH = ROOT / "audits" / "image-map.json"
USER_SHEET = "1RayD8PRCVwut5gRG3_awt3HcWBKMH3lIker09dAMBYI"

USERNAME = "codythebeast89"
ROBLOX_ID = "40485973"

LIVE_AWARDS = {
    "badges": [
        ("Aviator Badge", "Basic"),
        ("Combat Service Identification Badge", "1CAV, NATO, Afghanistan, Kosovo, Sea Duty, MATCOM CSIB"),
        ("Driver and Mechanic Badges", "Driver T, W & Operator"),
        ("Expert Soldier Badge", ""),
        ("Master Combat Action Badge", "3rd Award"),
        ("Master Gunner Identification Badge", ""),
        ("Overseas Bar", "9 Overseas Bars"),
        ("Sapper Tab", ""),
        ("Service Stripe", "4 Service Stripes"),
    ],
    "ribbons": [
        ("Afghanistan Campaign", "3rd Award"),
        ("Antarctica Service", "2nd Award"),
        ("Armed Forces Service Medal", ""),
        ("Army Commendation", '"C" (2nd Award)'),
        ("Army Good Conduct", "2nd Award"),
        ("Army of Occupation Medal", ""),
        ("Army Sea Duty", "3rd Award"),
        ("Iraq Campaign", ""),
        ("Joint Service Achievement", "3rd Award"),
        ("Kosovo Campaign", "3rd Award"),
        ("NATO ISAF", "2nd Award"),
        ("NATO Non-Article 5", "2nd Award"),
        ("Outstanding Volunteer", ""),
        ("Southwest Asia Service", ""),
    ],
    "foreign": [
        ("Queens Dedication Medal", ""),
    ],
}

SHEET_IDS = {
    "Interface": 415081012,
    "Badges": 1107052219,
    "Ribbons": 535614801,
    "OSB": 892134185,
    "JSA / Deployments": 46169016,
    "Army Sea Duty": 255774365,
    "Soutwest Asia Service": 1521712101,
    "Kosovo": 794131259,
}


def hex_rgb(h: str) -> dict:
    h = h.lstrip("#")
    return {
        "red": int(h[0:2], 16) / 255,
        "green": int(h[2:4], 16) / 255,
        "blue": int(h[4:6], 16) / 255,
    }


def load_token() -> dict:
    with TOKEN_PATH.open() as f:
        return json.load(f)


def refresh_token(tok: dict) -> dict:
    expiry = tok.get("expiry")
    if expiry:
        exp = datetime.fromisoformat(expiry.replace("Z", "+00:00"))
        if datetime.now(timezone.utc) < exp - timedelta(seconds=120):
            return tok
    data = urllib.parse.urlencode(
        {
            "grant_type": "refresh_token",
            "refresh_token": tok["refresh_token"],
            "client_id": tok["client_id"],
            "client_secret": tok["client_secret"],
        }
    ).encode()
    req = urllib.request.Request(tok["token_uri"], data=data, method="POST")
    with urllib.request.urlopen(req, timeout=30) as resp:
        new = json.load(resp)
    now = datetime.now(timezone.utc)
    tok["token"] = new["access_token"]
    tok["expiry"] = (now + timedelta(seconds=new.get("expires_in", 3600))).strftime(
        "%Y-%m-%dT%H:%M:%SZ"
    )
    with TOKEN_PATH.open("w") as f:
        json.dump(tok, f, indent=2)
    return tok


def sheets_api(tok: dict, path: str, body: dict | None = None) -> dict:
    tok = refresh_token(tok)
    url = f"https://sheets.googleapis.com/v4/spreadsheets/{USER_SHEET}{path}"
    data = json.dumps(body).encode() if body is not None else None
    method = "POST" if body and "batchUpdate" in path else ("PUT" if body else "GET")
    req = urllib.request.Request(
        url,
        data=data,
        method=method,
        headers={
            "Authorization": f"Bearer {tok['token']}",
            "Content-Type": "application/json",
        },
    )
    with urllib.request.urlopen(req, timeout=120) as resp:
        return json.load(resp)


def batch_update(tok: dict, requests: list, delay: float = 2.5) -> dict:
    time.sleep(delay)
    # Sheets allows up to ~100 requests per batch; chunk to stay safe.
    if len(requests) <= 40:
        return sheets_api(tok, ":batchUpdate", {"requests": requests})
    last = None
    for i in range(0, len(requests), 40):
        last = sheets_api(tok, ":batchUpdate", {"requests": requests[i : i + 40]})
        time.sleep(delay)
    return last or {}


def cell(
    value: str,
    bg: str | None = None,
    fg: str | None = None,
    bold: bool = False,
    size: int = 10,
    formula: bool = False,
):
    entry: dict = {}
    if formula:
        entry["userEnteredValue"] = {"formulaValue": value}
    else:
        entry["userEnteredValue"] = {"stringValue": value}
    fmt: dict = {"textFormat": {"fontSize": size, "bold": bold}}
    if bg:
        fmt["backgroundColor"] = hex_rgb(bg)
    if fg:
        fmt["textFormat"]["foregroundColor"] = hex_rgb(fg)
    entry["userEnteredFormat"] = fmt
    return entry


def update_range(tok: dict, sheet_id: int, r1: int, c1: int, rows: list[list[dict]]):
    batch_update(
        tok,
        [
            {
                "updateCells": {
                    "range": {
                        "sheetId": sheet_id,
                        "startRowIndex": r1,
                        "endRowIndex": r1 + len(rows),
                        "startColumnIndex": c1,
                        "endColumnIndex": c1 + max(len(r) for r in rows),
                    },
                    "rows": [{"values": r} for r in rows],
                    "fields": "userEnteredValue,userEnteredFormat",
                }
            }
        ],
    )


def merge(tok: dict, sheet_id: int, r1: int, r2: int, c1: int, c2: int):
    batch_update(
        tok,
        [
            {
                "mergeCells": {
                    "range": {
                        "sheetId": sheet_id,
                        "startRowIndex": r1,
                        "endRowIndex": r2,
                        "startColumnIndex": c1,
                        "endColumnIndex": c2,
                    },
                    "mergeType": "MERGE_ALL",
                }
            }
        ],
    )


def normalize(name: str) -> str:
    return re.sub(r"[^a-z0-9]+", " ", name.lower()).strip()


def match_award(row_name: str, live_name: str) -> bool:
    a = normalize(row_name)
    b = normalize(live_name)
    if a == b or a in b or b in a:
        return True
    aliases = {
        "diver and mechanic badges": "driver and mechanic badges",
        "combat action badge": "master combat action badge",
        "queen s dedication medal": "queens dedication medal",
        "afghanistan campagin": "afghanistan campaign",
        "antartica service": "antarctica service",
    }
    aa = aliases.get(a, a)
    bb = aliases.get(b, b)
    return aa == bb or aa in bb or bb in aa


def find_image(name: str, maps: dict) -> str | None:
    n = normalize(name)
    image_aliases = {
        "master combat action badge": "combat action badge",
        "diver and mechanic badges": "driver and mechanic badges",
    }
    n = image_aliases.get(n, n)
    for bucket in ("badges", "ribbons", "foreign"):
        for k, url in maps.get(bucket, {}).items():
            kn = normalize(k)
            if n in kn or kn in n:
                return url
    return None


def fix_badge_names(tok: dict) -> int:
    """Rename tracker rows that drift from the live decorations database."""
    enc = urllib.parse.quote("'Badges'!A1:F100", safe="")
    vals = sheets_api(tok, f"/values/{enc}").get("values", [])
    if not vals:
        return 0
    header, *rows = vals
    fixes = {"Diver and Mechanic Badges": "Driver and Mechanic Badges"}
    changed = 0
    for row in rows:
        if row and row[0] in fixes:
            row[0] = fixes[row[0]]
            changed += 1
    if changed:
        enc2 = urllib.parse.quote("'Badges'!A1", safe="")
        sheets_api(tok, f"/values/{enc2}?valueInputOption=USER_ENTERED", {"values": [header] + rows})
    return changed


def build_profile(tok: dict):
    sid = SHEET_IDS["Interface"]
    batch_update(tok, [{"updateSheetProperties": {"properties": {"sheetId": sid, "title": "Profile"}, "fields": "title"}}], delay=0)

    title = f"{USERNAME} | Service Record File"
    profile_rows = [
        ("Username", USERNAME),
        ("Roblox ID", ROBLOX_ID),
        ("Discord ID", ""),
        ("Rank", ""),
        ("Command", "Forces Command"),
        ("Division", ""),
        ("Brigade/Battalion/Group", ""),
        ("Company", ""),
        ("Join Date", ""),
        ("Unit Time of Service", '=IF(I15="","",DATEDIF(I15,TODAY(),"D")&" days")'),
        ("Position", ""),
        ("Position Date of Hire", ""),
    ]

    reqs: list[dict] = []
    reqs.append(
        {
            "updateCells": {
                "range": {"sheetId": sid, "startRowIndex": 0, "endRowIndex": 2, "startColumnIndex": 1, "endColumnIndex": 10},
                "rows": [[cell("", "#ffffff")] * 9, [cell("", "#ffffff")] * 9],
                "fields": "userEnteredValue,userEnteredFormat",
            }
        }
    )
    reqs.append(
        {
            "updateCells": {
                "range": {"sheetId": sid, "startRowIndex": 2, "endRowIndex": 5, "startColumnIndex": 1, "endColumnIndex": 5},
                "rows": [[cell("", "#434343")] * 4] * 3,
                "fields": "userEnteredValue,userEnteredFormat",
            }
        }
    )
    reqs.append({"mergeCells": {"range": {"sheetId": sid, "startRowIndex": 4, "endRowIndex": 5, "startColumnIndex": 2, "endColumnIndex": 5}, "mergeType": "MERGE_ALL"}})
    reqs.append(
        {
            "updateCells": {
                "range": {"sheetId": sid, "startRowIndex": 4, "endRowIndex": 5, "startColumnIndex": 2, "endColumnIndex": 3},
                "rows": [[cell(title, "#980000", "#f4cccc", True, 15)]],
                "fields": "userEnteredValue,userEnteredFormat",
            }
        }
    )

    for i, (label, value) in enumerate(profile_rows):
        row_idx = 6 + i
        is_formula = value.startswith("=")
        reqs.append({"mergeCells": {"range": {"sheetId": sid, "startRowIndex": row_idx, "endRowIndex": row_idx + 1, "startColumnIndex": 6, "endColumnIndex": 8}, "mergeType": "MERGE_ALL"}})
        reqs.append({"mergeCells": {"range": {"sheetId": sid, "startRowIndex": row_idx, "endRowIndex": row_idx + 1, "startColumnIndex": 8, "endColumnIndex": 11}, "mergeType": "MERGE_ALL"}})
        reqs.append(
            {
                "updateCells": {
                    "range": {"sheetId": sid, "startRowIndex": row_idx, "endRowIndex": row_idx + 1, "startColumnIndex": 6, "endColumnIndex": 9},
                    "rows": [[cell(label, "#999999"), cell("", "#999999"), cell(value, "#cccccc", formula=is_formula)]],
                    "fields": "userEnteredValue,userEnteredFormat",
                }
            }
        )

    reqs.append({"mergeCells": {"range": {"sheetId": sid, "startRowIndex": 19, "endRowIndex": 21, "startColumnIndex": 6, "endColumnIndex": 11}, "mergeType": "MERGE_ALL"}})
    reqs.append(
        {
            "updateCells": {
                "range": {"sheetId": sid, "startRowIndex": 19, "endRowIndex": 21, "startColumnIndex": 6, "endColumnIndex": 7},
                "rows": [
                    [cell('="Badges: "&COUNTIF(Badges!E:E,"Obtained")&" obtained"', "#666666", "#b7b7b7", True, formula=True)],
                    [cell('="Ribbons: "&COUNTIF(Ribbons!D:D,"Obtained")&" obtained"', "#666666", "#b7b7b7", True, formula=True)],
                ],
                "fields": "userEnteredValue,userEnteredFormat",
            }
        }
    )
    reqs.append({"mergeCells": {"range": {"sheetId": sid, "startRowIndex": 22, "endRowIndex": 23, "startColumnIndex": 1, "endColumnIndex": 10}, "mergeType": "MERGE_ALL"}})
    reqs.append(
        {
            "updateCells": {
                "range": {"sheetId": sid, "startRowIndex": 22, "endRowIndex": 23, "startColumnIndex": 1, "endColumnIndex": 2},
                "rows": [[cell("Proof of Username change [OLD] noob4style >>> [NEW] xLvcified", "#356854", "#ffffff")]],
                "fields": "userEnteredValue,userEnteredFormat",
            }
        }
    )
    batch_update(tok, reqs)


def sync_checklist(tok: dict, sheet: str, obtained_col: int, live: list[tuple[str, str]]) -> int:
    enc = urllib.parse.quote(f"'{sheet}'!A1:F100", safe="")
    vals = sheets_api(tok, f"/values/{enc}").get("values", [])
    if not vals:
        return 0
    header, *rows = vals
    changed = 0
    device_col = 5 if sheet == "Badges" else 4
    for row in rows:
        if not row:
            continue
        name = row[0]
        while len(row) < device_col + 1:
            row.append("")
        matched = next(((n, d) for n, d in live if match_award(name, n)), None)
        new_status = "Obtained" if matched else "Not Obtained"
        new_device = matched[1] if matched and matched[1] else "N/A"
        if row[obtained_col - 1] != new_status or row[device_col] != new_device:
            changed += 1
        row[obtained_col - 1] = new_status
        row[device_col] = new_device if matched else (row[device_col] or "N/A")
    enc2 = urllib.parse.quote(f"'{sheet}'!A1", safe="")
    sheets_api(tok, f"/values/{enc2}?valueInputOption=USER_ENTERED", {"values": [header] + rows})
    return changed


def add_conditional_formatting(tok: dict):
    requests = []

    def bool_rule(sheet_id: int, col: int, text: str, bg: str, bold: bool = False):
        fmt: dict = {"backgroundColor": hex_rgb(bg)}
        if bold:
            fmt["textFormat"] = {"bold": True}
        return {
            "addConditionalFormatRule": {
                "rule": {
                    "ranges": [{"sheetId": sheet_id, "startRowIndex": 1, "endRowIndex": 100, "startColumnIndex": col, "endColumnIndex": col + 1}],
                    "booleanRule": {
                        "condition": {"type": "TEXT_EQ", "values": [{"userEnteredValue": text}]},
                        "format": fmt,
                    },
                },
                "index": 0,
            }
        }

    requests += [
        bool_rule(SHEET_IDS["Badges"], 4, "Obtained", "#d9ead3", True),
        bool_rule(SHEET_IDS["Badges"], 4, "Not Obtained", "#f4cccc"),
        bool_rule(SHEET_IDS["Ribbons"], 3, "Obtained", "#d9ead3", True),
        bool_rule(SHEET_IDS["Ribbons"], 3, "Not Obtained", "#f4cccc"),
    ]

    group_colors = {
        "Group 1": "#cfe2f3",
        "Group 2": "#d9ead3",
        "Group 3": "#fff2cc",
        "Group 4": "#fce5cd",
        "Group 5": "#d9d2e9",
        "Identification Badge": "#ead1dc",
        "Tab": "#c9daf8",
        "Overseas Bar": "#b6d7a8",
        "Service Stripe": "#b6d7a8",
        "Foreign Awards": "#f4cccc",
        "Achievement": "#cfe2f3",
        "Service": "#d9ead3",
        "Campaign": "#fce5cd",
    }
    for label, bg in group_colors.items():
        for sheet_id, formula in (
            (SHEET_IDS["Badges"], f'=$B2="{label}"'),
            (SHEET_IDS["Ribbons"], f'=$B2="{label}"'),
        ):
            requests.append(
                {
                    "addConditionalFormatRule": {
                        "rule": {
                            "ranges": [{"sheetId": sheet_id, "startRowIndex": 1, "endRowIndex": 100, "startColumnIndex": 0, "endColumnIndex": 6}],
                            "booleanRule": {
                                "condition": {"type": "CUSTOM_FORMULA", "values": [{"userEnteredValue": formula}]},
                                "format": {"backgroundColor": hex_rgb(bg)},
                            },
                        },
                        "index": 0,
                    }
                }
            )
    batch_update(tok, requests)


def add_sheet(tok: dict, title: str) -> int:
    resp = batch_update(tok, [{"addSheet": {"properties": {"title": title, "gridProperties": {"rowCount": 50, "columnCount": 20}}}}])
    return resp["replies"][0]["addSheet"]["properties"]["sheetId"]


def sheet_exists(tok: dict, title: str) -> bool:
    meta = sheets_api(tok, "?fields=sheets(properties(title))")
    return any(s["properties"]["title"] == title for s in meta.get("sheets", []))


# Reference layout from ocpstandard Service Record File.xlsx (Downloads)
BADGE_LAYOUT = {
    "skill": {
        "header_col": 2,
        "label_col": 2,
        "img_col": 2,
        "name_col": 3,
        "groups": [
            ("Group 1", [("Master Combat Action Badge", "3rd Award")]),
            ("Group 2", [("Expert Soldier Badge", "-")]),
            ("Group 3", [("Aviator Badge", "Basic")]),
            ("Group 5", [("Driver and Mechanic Badges", "Driver T, W & Operator")]),
        ],
    },
    "identification": {
        "header_col": 5,
        "img_col": 5,
        "name_col": 6,
        "items": [
            ("Master Gunner Identification Badge", "Master"),
            ("Combat Service Identification Badge", "1CAV, NATO, Afghanistan, Kosovo, Sea Duty, MATCOM CSIB"),
        ],
    },
    "tabs": {
        "header_col": 8,
        "img_col": 8,
        "name_col": 9,
        "items": [("Sapper Tab", "-")],
    },
    "service": {
        "header_col": 11,
        "img_col": 11,
        "name_col": 12,
        "items": [
            ("Overseas Bar", "x9"),
            ("Service Stripe", "x4"),
        ],
    },
    "foreign": {
        "header_col": 14,
        "img_col": 14,
        "name_col": 15,
        "items": [("Queens Dedication Medal", "-")],
    },
}

SECTION_HEADERS = [
    (2, "Skill Badges"),
    (5, "Identification Badges"),
    (8, "Skill Tabs"),
    (11, "Service Awards"),
    (14, "Foreign Awards"),
]


def delete_sheet_by_title(tok: dict, title: str) -> bool:
    meta = sheets_api(tok, "?fields=sheets(properties(sheetId,title))")
    for s in meta.get("sheets", []):
        if s["properties"]["title"] == title:
            batch_update(tok, [{"deleteSheet": {"sheetId": s["properties"]["sheetId"]}}])
            return True
    return False


def set_cells(tok: dict, sheet_id: int, placements: list[tuple[int, int, dict]]):
    """Batch-write sparse cells: list of (row, col, cell_dict) zero-indexed."""
    if not placements:
        return
    by_row: dict[int, dict[int, dict]] = {}
    for r, c, cd in placements:
        by_row.setdefault(r, {})[c] = cd
    rows = []
    row_indices = sorted(by_row)
    min_r, max_r = row_indices[0], row_indices[-1]
    min_c = min(c for cols in by_row.values() for c in cols)
    max_c = max(c for cols in by_row.values() for c in cols) + 1
    for r in range(min_r, max_r + 1):
        row_cells = []
        for c in range(min_c, max_c):
            row_cells.append(by_row.get(r, {}).get(c, cell("")))
        rows.append(row_cells)
    batch_update(
        tok,
        [
            {
                "updateCells": {
                    "range": {
                        "sheetId": sheet_id,
                        "startRowIndex": min_r,
                        "endRowIndex": max_r + 1,
                        "startColumnIndex": min_c,
                        "endColumnIndex": max_c,
                    },
                    "rows": [{"values": r} for r in rows],
                    "fields": "userEnteredValue,userEnteredFormat",
                }
            }
        ],
    )


def badge_entry(name: str, device: str, maps: dict) -> tuple[dict, dict, dict]:
    url = find_image(name, maps)
    img = cell(f'=IMAGE("{url}")', formula=True) if url else cell("", "#cccccc")
    return img, cell(name, "#999999"), cell(device or "-", "#cccccc")


def build_decorations_badges(tok: dict, maps: dict, force: bool = False):
    if force:
        delete_sheet_by_title(tok, "Decorations - Badges")
    elif sheet_exists(tok, "Decorations - Badges"):
        return
    sid = add_sheet(tok, "Decorations - Badges")

    placements: list[tuple[int, int, dict]] = []
    # backdrop row 3 (index 2)
    for c in range(1, 18):
        placements.append((2, c, cell("", "#434343")))

    placements.append((3, 2, cell("Badges", "#cc0000", size=20)))
    for col, title in SECTION_HEADERS:
        placements.append((6, col, cell(title, "#666666", "#b7b7b7", True)))

    # Skill badges with group labels
    row = 7
    for group, items in BADGE_LAYOUT["skill"]["groups"]:
        lc, ic, nc = BADGE_LAYOUT["skill"]["label_col"], BADGE_LAYOUT["skill"]["img_col"], BADGE_LAYOUT["skill"]["name_col"]
        placements.append((row, lc, cell(group, "#999999")))
        row += 1
        for name, device in items:
            img, nm, dev = badge_entry(name, device, maps)
            placements += [(row, ic, img), (row, nc, nm), (row + 1, nc, dev)]
            row += 2

    # Flat sections (identification, tabs, service, foreign)
    for key in ("identification", "tabs", "service", "foreign"):
        sec = BADGE_LAYOUT[key]
        row = 7
        for name, device in sec["items"]:
            img, nm, dev = badge_entry(name, device, maps)
            placements += [(row, sec["img_col"], img), (row, sec["name_col"], nm), (row + 1, sec["name_col"], dev)]
            row += 2

    set_cells(tok, sid, placements)

    merge_reqs = [
        {"mergeCells": {"range": {"sheetId": sid, "startRowIndex": 3, "endRowIndex": 4, "startColumnIndex": 2, "endColumnIndex": 5}, "mergeType": "MERGE_ALL"}},
    ]
    for col, _ in SECTION_HEADERS:
        merge_reqs.append(
            {"mergeCells": {"range": {"sheetId": sid, "startRowIndex": 6, "endRowIndex": 7, "startColumnIndex": col, "endColumnIndex": col + 3}, "mergeType": "MERGE_ALL"}}
        )
    batch_update(tok, merge_reqs)


def build_decorations_ribbons(tok: dict, maps: dict, force: bool = False):
    if force:
        delete_sheet_by_title(tok, "Decorations - Ribbons")
    elif sheet_exists(tok, "Decorations - Ribbons"):
        return
    sid = add_sheet(tok, "Decorations - Ribbons")

    placements: list[tuple[int, int, dict]] = []
    for c in range(1, 12):
        placements.append((2, c, cell("", "#434343")))
    placements.append((3, 2, cell("Ribbons", "#e69138", size=20)))
    placements.append(
        (6, 2, cell('=COUNTIF(Ribbons!D:D,"Obtained")&" Ribbons"', "#666666", "#b7b7b7", True, formula=True))
    )

    ribbons = LIVE_AWARDS["ribbons"]
    left = ribbons[:7]
    right = ribbons[7:]

    # Reference: right column starts row 8, left column row 10; name + device on paired rows
    for i, (name, device) in enumerate(right):
        row = 7 + i * 2
        url = find_image(name, maps)
        img = cell(f'=IMAGE("{url}")', formula=True) if url else cell("", "#cccccc")
        placements += [(row, 8, img), (row, 9, cell(name, "#999999")), (row + 1, 9, cell(device or "-", "#cccccc"))]

    for i, (name, device) in enumerate(left):
        row = 9 + i * 2
        url = find_image(name, maps)
        img = cell(f'=IMAGE("{url}")', formula=True) if url else cell("", "#cccccc")
        placements += [(row, 3, img), (row, 4, cell(name, "#999999")), (row + 1, 4, cell(device or "-", "#cccccc"))]

    set_cells(tok, sid, placements)
    batch_update(
        tok,
        [
            {"mergeCells": {"range": {"sheetId": sid, "startRowIndex": 3, "endRowIndex": 4, "startColumnIndex": 2, "endColumnIndex": 5}, "mergeType": "MERGE_ALL"}},
            {"mergeCells": {"range": {"sheetId": sid, "startRowIndex": 6, "endRowIndex": 7, "startColumnIndex": 2, "endColumnIndex": 4}, "mergeType": "MERGE_ALL"}},
        ],
    )


def rename_sheets(tok: dict):
    renames = {"OSB": "Proof - Overseas Bar", "JSA / Deployments": "Proof - JSA", "Army Sea Duty": "Proof - Campaign"}
    reqs = []
    for old, new in renames.items():
        if old in SHEET_IDS:
            reqs.append({"updateSheetProperties": {"properties": {"sheetId": SHEET_IDS[old], "title": new}, "fields": "title"}})
    if reqs:
        batch_update(tok, reqs)


def add_events_log(tok: dict):
    if sheet_exists(tok, "Events Log"):
        return
    sid = add_sheet(tok, "Events Log")
    merge(tok, sid, 3, 4, 2, 6)
    update_range(tok, sid, 3, 2, [[cell("Events Log", "#980000", "#f4cccc", True, 15)]])
    update_range(tok, sid, 6, 2, [[cell(h, "#666666", "#b7b7b7", True) for h in ["Date", "Event", "Host", "Attendance", "Notes", "Proof Link"]]])


def main() -> int:
    if not TOKEN_PATH.is_file():
        print("Run awards-tui --login first", file=sys.stderr)
        return 1
    maps = json.loads(IMAGE_MAP_PATH.read_text()) if IMAGE_MAP_PATH.is_file() else {"badges": {}, "ribbons": {}, "foreign": {}}
    tok = load_token()
    steps = sys.argv[1:] or ["profile", "sync", "format", "badges", "ribbons", "proof"]
    if "profile" in steps:
        print("1. Profile…")
        build_profile(tok)
    if "fix-names" in steps:
        print("Fixing badge name drift…")
        n = fix_badge_names(tok)
        print(f"   {n} badge row(s) renamed")
    if "sync" in steps:
        print("2. Sync awards…")
        fix_badge_names(tok)
        b = sync_checklist(tok, "Badges", 5, LIVE_AWARDS["badges"] + LIVE_AWARDS["foreign"])
        r = sync_checklist(tok, "Ribbons", 4, LIVE_AWARDS["ribbons"])
        print(f"   {b} badge rows, {r} ribbon rows updated")
    if "format" in steps:
        print("3. Conditional formatting…")
        add_conditional_formatting(tok)
    if "badges" in steps or "rebuild-badges" in steps:
        print("4. Decorations - Badges…")
        build_decorations_badges(tok, maps, force="rebuild-badges" in steps)
    if "ribbons" in steps or "rebuild-ribbons" in steps:
        print("5. Decorations - Ribbons…")
        build_decorations_ribbons(tok, maps, force="rebuild-ribbons" in steps)
    if "proof" in steps:
        print("6. Proof renames…")
        rename_sheets(tok)
    if "events" in steps:
        print("7. Events Log…")
        add_events_log(tok)
    print(f"Done: https://docs.google.com/spreadsheets/d/{USER_SHEET}/edit")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
