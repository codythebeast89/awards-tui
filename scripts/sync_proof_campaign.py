#!/usr/bin/env python3
"""Sync Proof - ASD (Army Sea Duty) tab from campaign tracker docs.

Callers: `python3 scripts/sync_proof_campaign.py`
API: Google Sheets values batchUpdate on USER_SHEET Proof - ASD tab.
Schema: rows Name | Number | Date | Week | Status | Link (HYPERLINK to AAR docs).
"""

from __future__ import annotations

import json
import sys
import urllib.request
from datetime import datetime, timedelta
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

from upgrade_qmc_tracker import USER_SHEET, load_token, refresh_token  # noqa: E402

ARMY_SEA_DUTY_DEPLOYMENTS: list[tuple[str, str, str, str, str]] = [
    (
        "Operation Potatos more like tomatos",
        "Army Sea Duty x1",
        "11/25/2025",
        "Operation Potatos more like tomatos | After Action",
        "https://docs.google.com/document/d/1M6wrSGg-ZGMSX8i_ExY_tHzth9pNJeBI3PUBwWOc7oE/edit",
    ),
    (
        "OPERATION HAYDOS IS SIGMA",
        "Army Sea Duty x1",
        "12/7/2025",
        "Operation Haydos is sigma | After Action",
        "https://docs.google.com/document/d/1Bd_WnR9Mxpe4WZaBF7Wwq0rPxxPSXmcdyhrjIAPrUTA/edit",
    ),
    (
        "OPERATION: QMC might hate Miko",
        "Army Sea Duty x1",
        "1/3/2026",
        "165th x 1CAV vs RAF | OPERATION: QMC might hate Miko",
        "https://docs.google.com/document/d/1BjSfya5GKScg9lKdq0rgNN1fOGNBP-44o5dk71RpNoI/edit",
    ),
    (
        "OPERATION EAGLE",
        "Army Sea Duty x2",
        "1/10/2026",
        "Operation Eagle | After Action",
        "https://docs.google.com/document/d/1Swe1Jg6JoTiH6UJiVszmPUPy7DnnDfd2Xuz95ZjQ8WU/edit",
    ),
    (
        "OPERATION New Year, New Beatdown",
        "Army Sea Duty x2",
        "1/17/2026",
        "Operation New Year, New Beatdown | After Action",
        "https://docs.google.com/document/d/1P9JmF6Gl8aGonRiQPvF4nfcL3BztEnv3tV9M1Id2w6c/edit",
    ),
    (
        "OPERATION: Name 24",
        "Army Sea Duty x2",
        "1/24/2026",
        "AFA x 1st Cavalry Division vs KAS | After Action Report",
        "https://docs.google.com/document/d/1SRbqKFMWkXtOiJJi3FU6w6D3giQ2-lri_mbAHKblmgs/edit",
    ),
    (
        "OPERATION Desert Re-Claim",
        "Army Sea Duty x3",
        "2/1/2026",
        "Operation Desert Re-Claim",
        "https://docs.google.com/document/d/1LaYRopuLQEgCZBWfhkKIKds7PC8P3jBA1t3jWU3drXk/edit",
    ),
    (
        "OPERATION Stuck in the Potholes",
        "Army Sea Duty x3",
        "2/13/2026",
        "Operation Stuck in the Potholes | After Action report",
        "https://docs.google.com/document/d/1eIcUha8Ht3NRb1Yl8LUvwnpZgq8DQeWzvb6HmsKpVLo/edit",
    ),
    (
        "Operation I Do Not Know MTC",
        "Army Sea Duty x3",
        "2/22/2026",
        "FORSCOM Deployment v FAF | Operation I Do Not Know MTC",
        "https://docs.google.com/document/d/1t55VNQjtffi8MebGdSDfGisTFI1iMTRuDHj9vXSiq60/edit",
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


def hyperlink_formula(url: str, label: str) -> str:
    safe_url = url.replace('"', '""')
    safe_label = label.replace('"', '""')
    return f'=HYPERLINK("{safe_url}","{safe_label}")'


def build_rows() -> list[list[str]]:
    header = [
        "Name",
        "Number",
        "Date of Deployment",
        "Week of Deployment",
        "Status",
        "Link",
    ]
    rows = [header]
    for name, number, date, link_label, url in ARMY_SEA_DUTY_DEPLOYMENTS:
        rows.append(
            [
                name,
                number,
                date,
                deployment_week(date),
                "Logged",
                hyperlink_formula(url, link_label),
            ]
        )
    return rows


def batch_update_values(tok: dict, rows: list[list[str]]) -> None:
    tok = refresh_token(tok)
    body = {
        "valueInputOption": "USER_ENTERED",
        "data": [{"range": "Proof - ASD!A1:F10", "values": rows}],
    }
    url = f"https://sheets.googleapis.com/v4/spreadsheets/{USER_SHEET}/values:batchUpdate"
    req = urllib.request.Request(
        url,
        data=json.dumps(body).encode(),
        method="POST",
        headers={
            "Authorization": f"Bearer {tok['token']}",
            "Content-Type": "application/json",
        },
    )
    with urllib.request.urlopen(req, timeout=60) as resp:
        json.load(resp)


def main() -> int:
    rows = build_rows()
    batch_update_values(load_token(), rows)
    print(f"Updated Proof - ASD with {len(rows) - 1} Army Sea Duty deployments")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
