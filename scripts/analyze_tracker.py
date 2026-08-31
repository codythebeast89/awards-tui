#!/usr/bin/env python3
"""Analyze QMC / logistics tracker spreadsheets via awards-tui OAuth credentials.

User request: use Sheets API from awards TUI to access both tracker spreadsheets
and deliver styling upgrade recommendations.

Callers: run manually (`python3 scripts/analyze_tracker.py`).
API: Google Sheets API v4 (read-only); reuses awards-tui token.json OAuth.
Schema: writes audits/tracker-analysis-<timestamp>.json with per-tab values + colors.

Usage:
  python3 scripts/analyze_tracker.py [SPREADSHEET_ID ...]
"""

from __future__ import annotations

import json
import sys
import urllib.error
import urllib.parse
import urllib.request
from collections import defaultdict
from datetime import datetime, timedelta, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TOKEN_PATH = ROOT / "token.json"
AUDITS = ROOT / "audits"

DEFAULT_SHEETS = [
    "1RayD8PRCVwut5gRG3_awt3HcWBKMH3lIker09dAMBYI",
    "1WEXwdOP_JvI6tFvxxCdaYsYPx_DavqJybhWIGGq9hrg",
]


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


def api(tok: dict, url: str, method: str = "GET", body: dict | None = None) -> dict:
    tok = refresh_token(tok)
    data = json.dumps(body).encode() if body is not None else None
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


def col_letter(ci: int) -> str:
    s = ""
    ci += 1
    while ci:
        ci, rem = divmod(ci - 1, 26)
        s = chr(65 + rem) + s
    return s


def rgb(color: dict | None) -> str | None:
    if not color:
        return None
    return (
        f"#{int(round(color.get('red', 0) * 255)):02x}"
        f"{int(round(color.get('green', 0) * 255)):02x}"
        f"{int(round(color.get('blue', 0) * 255)):02x}"
    )


def list_tabs(tok: dict, sheet_id: str) -> list[dict]:
    meta = api(tok, f"https://sheets.googleapis.com/v4/spreadsheets/{sheet_id}?fields=sheets(properties)")
    tabs = []
    for sh in meta.get("sheets", []):
        p = sh["properties"]
        gp = p.get("gridProperties", {})
        tabs.append(
            {
                "title": p["title"],
                "sheetId": p["sheetId"],
                "rows": gp.get("rowCount"),
                "cols": gp.get("columnCount"),
            }
        )
    return tabs


def get_values(tok: dict, sheet_id: str, sheet: str, a1: str = "A1:Z100") -> list[list[str]]:
    enc = urllib.parse.quote(f"'{sheet}'!{a1}", safe="")
    return (
        api(
            tok,
            f"https://sheets.googleapis.com/v4/spreadsheets/{sheet_id}/values/{enc}?valueRenderOption=FORMULA",
        ).get("values", [])
        or []
    )


def analyze_colors(tok: dict, sheet_id: str, sheet: str, rows: int = 40, cols: int = 20) -> dict:
    end = col_letter(cols - 1)
    rng = urllib.parse.quote(f"'{sheet}'!A1:{end}{rows}")
    resp = api(
        tok,
        f"https://sheets.googleapis.com/v4/spreadsheets/{sheet_id}?includeGridData=true&ranges={rng}",
    )
    colors: dict[str, int] = defaultdict(int)
    images: list[dict] = []
    for sh in resp.get("sheets", []):
        if sh["properties"]["title"] != sheet:
            continue
        for data in sh.get("data", []):
            for ri, row in enumerate(data.get("rowData", []) or []):
                for ci, cell in enumerate(row.get("values", []) or []):
                    if not cell:
                        continue
                    ef = cell.get("effectiveFormat", {}) or {}
                    bg = rgb(ef.get("backgroundColor"))
                    if bg and bg not in ("#ffffff",):
                        colors[bg] += 1
                    ue = cell.get("userEnteredValue", {})
                    if "formulaValue" in ue and "IMAGE(" in ue["formulaValue"].upper():
                        images.append(
                            {"cell": f"{col_letter(ci)}{ri + 1}", "formula": ue["formulaValue"][:300]}
                        )
    return {"colors": dict(sorted(colors.items(), key=lambda x: -x[1])), "images": images}


def analyze_spreadsheet(tok: dict, sheet_id: str) -> dict:
    meta = api(
        tok,
        f"https://sheets.googleapis.com/v4/spreadsheets/{sheet_id}?fields=properties(title)",
    )
    title = meta.get("properties", {}).get("title", sheet_id)
    tabs = list_tabs(tok, sheet_id)
    out = {"id": sheet_id, "title": title, "tabs": tabs, "sheets": {}}
    for tab in tabs:
        name = tab["title"]
        cols = min(tab["cols"] or 26, 26)
        rows = min(tab["rows"] or 50, 50)
        out["sheets"][name] = {
            "values": get_values(tok, sheet_id, name, f"A1:{col_letter(cols - 1)}{rows}"),
            "format": analyze_colors(tok, sheet_id, name, rows=35, cols=min(tab["cols"] or 20, 20)),
        }
    return out


def main() -> int:
    if not TOKEN_PATH.is_file():
        print(f"Missing {TOKEN_PATH}. Run: awards-tui --login", file=sys.stderr)
        return 1

    sheet_ids = sys.argv[1:] or DEFAULT_SHEETS
    tok = load_token()
    AUDITS.mkdir(exist_ok=True)
    stamp = datetime.now().strftime("%Y-%m-%d_%H%M%S")
    dest = AUDITS / f"tracker-analysis-{stamp}.json"

    report = {"generated": datetime.now(timezone.utc).isoformat(), "spreadsheets": []}
    for sid in sheet_ids:
        print(f"Analyzing {sid}…")
        try:
            report["spreadsheets"].append(analyze_spreadsheet(tok, sid))
        except urllib.error.HTTPError as err:
            body = err.read().decode("utf-8", "replace")
            print(f"  HTTP {err.code}: {body[:200]}", file=sys.stderr)
            report["spreadsheets"].append({"id": sid, "error": f"HTTP {err.code}"})

    dest.write_text(json.dumps(report, indent=2))
    print(f"Wrote {dest}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
