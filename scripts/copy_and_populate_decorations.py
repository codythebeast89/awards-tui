#!/usr/bin/env python3
"""Copy Decorations tabs from reference, then replace with user's awards.

Callers: `python3 scripts/copy_and_populate_decorations.py`
API: Sheets copyTo + values batchUpdate on USER_SHEET 1RayD8PRCVwut5gRG3_awt3HcWBKMH3lIker09dAMBYI
"""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

from copy_reference_tabs import REF_TABS, USER_SHEET, api, copy_tab, delete_tab, load_token  # noqa: E402
from upgrade_qmc_tracker import LIVE_AWARDS  # noqa: E402

BADGE_CELLS = [
    ("C8", "Group 1"),
    ("D9", "Master Combat Action Badge"),
    ("D10", "3rd Award"),
    ("C13", "Group 2"),
    ("D14", "Expert Soldier Badge"),
    ("D15", "-"),
    ("C16", "Group 3"),
    ("D17", "Aviator Badge"),
    ("D18", "Basic"),
    ("C28", "Group 5"),
    ("D29", "Driver and Mechanic Badges"),
    ("D30", "Driver T, W & Operator"),
    ("G8", "Master Gunner Identification Badge"),
    ("G9", "Master"),
    ("G12", "Combat Service Identification Badge"),
    ("G13", "1CAV, NATO, Afghanistan, Kosovo, Sea Duty, MATCOM CSIB"),
    ("J9", "Sapper Tab"),
    ("J10", "-"),
    ("M8", "Overseas Bar"),
    ("M9", "x9"),
    ("M10", "Service Stripe"),
    ("M11", "x4"),
    ("P8", "Queens Dedication Medal"),
    ("P9", "-"),
]

BADGE_CLEAR = [
    "D11", "D12", "D19", "D20", "D21", "D22", "D23", "D24", "D25", "D26", "D27",
    "G10", "G11", "G14", "G15", "J11", "J12", "M12", "M13", "P10", "P11",
    "C19", "C20", "C21", "C22", "C23", "C24", "C25", "C26", "C27",
]


def ribbon_pairs() -> list[tuple[str, str, str, str]]:
    ribbons = LIVE_AWARDS["ribbons"]
    left, right = ribbons[:7], ribbons[7:]
    pairs = []
    for i in range(max(len(left), len(right))):
        ln, ld = left[i] if i < len(left) else ("", "")
        rn, rd = right[i] if i < len(right) else ("", "")
        pairs.append((ln, ld or "-", rn, rd or "-"))
    return pairs


def populate_badges(tok: dict):
    data = [{"range": f"Decorations - Badges!{cell}", "values": [[""]]} for cell in BADGE_CLEAR]
    data += [{"range": f"Decorations - Badges!{cell}", "values": [[val]]} for cell, val in BADGE_CELLS]
    api(tok, "POST", "/values:batchUpdate?valueInputOption=USER_ENTERED", {"data": data, "valueInputOption": "USER_ENTERED"})


def populate_ribbons(tok: dict):
    n = len(LIVE_AWARDS["ribbons"])
    data = [{"range": "Decorations - Ribbons!C7", "values": [[f"{n} Ribbons"]]}]
    for row in range(8, 24):
        data.append({"range": f"Decorations - Ribbons!E{row}", "values": [[""]]})
        data.append({"range": f"Decorations - Ribbons!J{row}", "values": [[""]]})
    for i, (ln, ld, rn, rd) in enumerate(ribbon_pairs()):
        r_right, r_left = 8 + i * 2, 10 + i * 2
        if rn:
            data += [{"range": f"Decorations - Ribbons!J{r_right}", "values": [[rn]]}, {"range": f"Decorations - Ribbons!J{r_right + 1}", "values": [[rd]]}]
        if ln:
            data += [{"range": f"Decorations - Ribbons!E{r_left}", "values": [[ln]]}, {"range": f"Decorations - Ribbons!E{r_left + 1}", "values": [[ld]]}]
    api(tok, "POST", "/values:batchUpdate?valueInputOption=USER_ENTERED", {"data": data, "valueInputOption": "USER_ENTERED"})


def main() -> int:
    tok = load_token()
    for title, ref_id in [
        ("Decorations - Badges", REF_TABS["Decorations - Badges"]),
        ("Decorations - Ribbons", REF_TABS["Decorations - Ribbons"]),
    ]:
        print(f"Copying {title} from reference…")
        delete_tab(tok, title)
        copy_tab(tok, ref_id, title)
    print("Writing your awards…")
    populate_badges(tok)
    populate_ribbons(tok)
    print(f"Done: https://docs.google.com/spreadsheets/d/{USER_SHEET}/edit")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
