"""Fetch and index awards from the Decorations Database Google Sheet."""

from __future__ import annotations

import csv
import difflib
import io
import json
import re
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

SHEET_ID = "1e_AqHIGrGdfNSgoHt6kLV89E6LADJmlZzhfRAUXo0wY"
USER_AGENT = "awards-tui/1.1 (decorations lookup + edit)"

SHEET_META = {
    "Ribbons Database": {"category": "ribbons", "name_row": 2, "data_start_row": 3, "row_offset": 0},
    # Public CSV export for Badges is 6 rows shorter than the live sheet row numbers.
    "Badges Database": {"category": "badges", "name_row": 3, "data_start_row": 4, "row_offset": 6},
    "Foreign Awards Database": {"category": "foreign", "name_row": 2, "data_start_row": 3, "row_offset": 0},
}

CATEGORY_LABELS = {
    "badges": "Badges",
    "ribbons": "Ribbons",
    "foreign": "Foreign Awards",
}

BADGE_ABBREV_SPECIAL = {
    "ESB": "Expert Soldier Badge",
}

ROOT = Path(__file__).resolve().parent
COLUMNS_PATH = ROOT / "award_columns.json"


@dataclass(frozen=True)
class Award:
    category: str
    name: str
    sheet: str = ""
    col: str = ""
    row: int = 0  # 1-based sheet row
    cell: str = ""
    base_name: str = ""


@dataclass(frozen=True)
class AwardDef:
    category: str
    sheet: str
    col: str
    base_name: str


@dataclass(frozen=True)
class DuplicateHit:
    category: str
    base_name: str
    sheet: str
    col: str
    row: int
    cell: str
    cell_username: str
    reason: str  # duplicate_in_column | similar_username | malformed_cell

    def to_award(self) -> Award:
        label = {
            "duplicate_in_column": "duplicate row",
            "similar_username": f"similar to @{self.cell_username}",
            "malformed_cell": "malformed cell",
        }.get(self.reason, self.reason)
        return Award(
            category=self.category,
            name=f"⚠ {self.base_name} ({label})",
            sheet=self.sheet,
            col=self.col,
            row=self.row,
            cell=self.cell,
            base_name=self.base_name,
        )


@dataclass
class AwardsData:
    index: dict[str, list[Award]]
    catalog: list[AwardDef]
    sheet_rows: dict[str, list[list[str]]]


def row_offset(sheet: str) -> int:
    return SHEET_META.get(sheet, {}).get("row_offset", 0)


def csv_index_to_sheet_row(sheet: str, csv_index: int) -> int:
    """Convert 0-based CSV row index to 1-based Google Sheets row number."""
    return csv_index + 1 + row_offset(sheet)


def sheet_data_start_row(sheet: str) -> int:
    meta = SHEET_META[sheet]
    return meta["data_start_row"] + row_offset(sheet)


def col_to_index(col: str) -> int:
    n = 0
    for ch in col.upper():
        n = n * 26 + (ord(ch) - 64)
    return n - 1


def index_to_col(idx: int) -> str:
    n = idx + 1
    letters = []
    while n:
        n, rem = divmod(n - 1, 26)
        letters.append(chr(65 + rem))
    return "".join(reversed(letters))


def normalize_username(cell: str | None) -> str | None:
    if not cell:
        return None
    match = re.match(r"^@?([A-Za-z0-9_]+)", cell.strip())
    return match.group(1).lower() if match else None


def usernames_similar(a: str, b: str) -> bool:
    """True when two usernames are likely the same person with a typo."""
    if not a or not b or a == b:
        return False
    if abs(len(a) - len(b)) > 3:
        return False
    # Require strong similarity and a shared prefix so random names don't match.
    prefix = 0
    for x, y in zip(a, b, strict=False):
        if x != y:
            break
        prefix += 1
    if prefix < max(4, int(min(len(a), len(b)) * 0.55)):
        return False
    return difflib.SequenceMatcher(None, a, b).ratio() >= 0.84


def cell_format_issues(cell: str) -> list[str]:
    """Detect common sheet entry formatting problems."""
    text = cell.strip()
    issues: list[str] = []
    if re.search(r"[A-Za-z0-9_]-", text):
        issues.append("missing_space_before_dash")
    if "  " in text:
        issues.append("extra_spaces")
    if text != cell:
        issues.append("trailing_space")
    return issues


def find_duplicates_for_user(data: AwardsData, username: str) -> list[DuplicateHit]:
    """Find duplicate rows, typos, and malformed cells related to a username."""
    key = normalize_username(username) or username.strip().lstrip("@").lower()
    if not key:
        return []

    exact_by_col: dict[tuple[str, str], list[tuple[int, str, str]]] = {}
    hits: list[DuplicateHit] = []
    seen_hit: set[tuple[str, str, int, str]] = set()

    def add_hit(
        *,
        category: str,
        base_name: str,
        sheet: str,
        col: str,
        row: int,
        cell: str,
        cell_username: str,
        reason: str,
    ) -> None:
        sig = (sheet, col, row, reason)
        if sig in seen_hit:
            return
        seen_hit.add(sig)
        hits.append(
            DuplicateHit(
                category=category,
                base_name=base_name,
                sheet=sheet,
                col=col,
                row=row,
                cell=cell,
                cell_username=cell_username,
                reason=reason,
            )
        )

    for entry in load_columns():
        sheet = entry["sheet"]
        col = entry["col"]
        meta = SHEET_META.get(sheet)
        rows = data.sheet_rows.get(sheet)
        if not meta or not rows:
            continue

        col_idx = col_to_index(col)
        base_name = ""
        name_row = rows[meta["name_row"] - 1] if len(rows) >= meta["name_row"] else []
        if col_idx < len(name_row):
            base_name = (name_row[col_idx] or "").strip()
        if not base_name:
            continue

        col_key = (sheet, col)
        for r in range(meta["data_start_row"] - 1, len(rows)):
            row = rows[r]
            cell = row[col_idx] if col_idx < len(row) else ""
            if not cell or not str(cell).strip():
                continue
            cell = str(cell).strip()
            cell_user = normalize_username(cell)
            if not cell_user:
                continue

            sheet_row = csv_index_to_sheet_row(sheet, r)
            issues = cell_format_issues(cell)

            if cell_user == key:
                exact_by_col.setdefault(col_key, []).append((sheet_row, cell, cell_user))
                if issues:
                    add_hit(
                        category=meta["category"],
                        base_name=base_name,
                        sheet=sheet,
                        col=col,
                        row=sheet_row,
                        cell=cell,
                        cell_username=cell_user,
                        reason="malformed_cell",
                    )
            elif usernames_similar(cell_user, key):
                add_hit(
                    category=meta["category"],
                    base_name=base_name,
                    sheet=sheet,
                    col=col,
                    row=sheet_row,
                    cell=cell,
                    cell_username=cell_user,
                    reason="similar_username",
                )
                if issues:
                    add_hit(
                        category=meta["category"],
                        base_name=base_name,
                        sheet=sheet,
                        col=col,
                        row=sheet_row,
                        cell=cell,
                        cell_username=cell_user,
                        reason="malformed_cell",
                    )

    for (sheet, col), entries in exact_by_col.items():
        if len(entries) < 2:
            continue
        meta = SHEET_META[sheet]
        col_idx = col_to_index(col)
        rows = data.sheet_rows[sheet]
        base_name = rows[meta["name_row"] - 1][col_idx].strip()
        for sheet_row, cell, cell_user in entries:
            add_hit(
                category=meta["category"],
                base_name=base_name,
                sheet=sheet,
                col=col,
                row=sheet_row,
                cell=cell,
                cell_username=cell_user,
                reason="duplicate_in_column",
            )

    hits.sort(key=lambda h: (h.reason, h.base_name.casefold(), h.row))
    return hits


def ordinal_award(n: int) -> str:
    if n == 2:
        return "2nd Award"
    if n == 3:
        return "3rd Award"
    return f"{n}th Award"


def format_ribbon_award(base_name: str, cell: str) -> str:
    name = base_name.strip()
    device = re.search(r'-\s*"([^"]+)"', cell)
    if device:
        name += f' ("{device.group(1)}")'
    count = re.search(r"\bx(\d+)\b", cell, re.I)
    if count:
        name += f" ({ordinal_award(int(count.group(1)))})"
    return name


CJS_RE = re.compile(r"\(?\s*(?:(\d+)\s*x|x\s*(\d+))\s*CJS\s*\)?", re.I)


def cjs_phrase(n: int) -> str:
    if n <= 1:
        return "Combat Jump Star"
    return f"{n} Combat Jump Stars"


def extract_cjs(cell: str) -> tuple[str, str | None]:
    """Pull Combat Jump Star notation out so its count is not treated as an award ordinal."""
    match = CJS_RE.search(cell)
    if not match:
        return cell, None
    n = int(match.group(1) or match.group(2))
    rest = CJS_RE.sub("", cell)
    rest = re.sub(r"\(\s*\)", "", rest)
    rest = re.sub(r"\s*-\s*$", "", rest)
    rest = re.sub(r"\s+", " ", rest).strip()
    return rest, cjs_phrase(n)


def attach_cjs(name: str, cjs: str | None) -> str:
    if not cjs:
        return name
    if name.endswith(")") and "(" in name:
        return f"{name[:-1]}, {cjs})"
    return f"{name} ({cjs})"


def expand_badge_abbrev(base_name: str, abbrev: str) -> str:
    """Expand short badge level codes relative to the column's award name.

    MC means Master of the *current* badge (CIB → Master Combat Infantryman Badge,
    CAB → Master Combat Action Badge), not a hardcoded CAB title.
    """
    base = base_name.strip()
    key = abbrev.strip().upper()
    if key == "MC":
        if base.lower().startswith("master "):
            return base
        return f"Master {base}"
    if key in BADGE_ABBREV_SPECIAL:
        return BADGE_ABBREV_SPECIAL[key]
    return base


def format_badge_award(base_name: str, cell: str) -> str:
    base = base_name.strip()
    cell, cjs = extract_cjs(cell)
    dash = cell.find(" - ")
    if dash == -1:
        return attach_cjs(format_ribbon_award(base, cell), cjs)
    detail = cell[dash + 3 :].strip()
    if not detail:
        return attach_cjs(format_ribbon_award(base, cell), cjs)

    # Count may sit before or after the dash: "user x2 - MC" or "user - MC x2"
    count = re.search(r"\bx(\d+)\b", cell, re.I)
    label = re.sub(r"\s*x\d+\b", "", detail, flags=re.I).strip()
    label = re.sub(r"\s+", " ", label).strip(" -")

    # Known short codes (MC, ESB), with or without an award count
    if label.upper() in ("MC", "ESB") and "," not in detail:
        name = expand_badge_abbrev(base, label)
        if count:
            name += f" ({ordinal_award(int(count.group(1)))})"
        return attach_cjs(name, cjs)

    if count and "," not in detail:
        name = base
        if label and label.lower() not in name.lower():
            if len(label) <= 4:
                name = expand_badge_abbrev(base, label)
            else:
                name = f"{name} ({label})"
        name += f" ({ordinal_award(int(count.group(1)))})"
        return attach_cjs(name, cjs)

    return attach_cjs(f"{base} ({detail})", cjs)


def format_award_name(category: str, base_name: str | None, cell: str) -> str | None:
    if not base_name or not base_name.strip():
        return None
    if category == "badges":
        return format_badge_award(base_name, cell)
    return format_ribbon_award(base_name, cell)


def parse_csv(text: str) -> list[list[str]]:
    return list(csv.reader(io.StringIO(text)))


def fetch_sheet(sheet_name: str) -> list[list[str]]:
    query = urllib.parse.urlencode({"tqx": "out:csv", "sheet": sheet_name})
    url = f"https://docs.google.com/spreadsheets/d/{SHEET_ID}/gviz/tq?{query}"
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    try:
        with urllib.request.urlopen(req, timeout=60) as resp:
            raw = resp.read()
    except urllib.error.HTTPError as exc:
        raise RuntimeError(f"Sheet fetch failed ({sheet_name}): HTTP {exc.code}") from exc
    except urllib.error.URLError as exc:
        raise RuntimeError(f"Sheet fetch failed ({sheet_name}): {exc.reason}") from exc
    return parse_csv(raw.decode("utf-8", errors="replace"))


def load_columns() -> list[dict]:
    return json.loads(COLUMNS_PATH.read_text(encoding="utf-8"))


def add_award(index: dict[str, list[Award]], username: str | None, award: Award | None) -> None:
    if not username or not award or not award.name:
        return
    existing = index.setdefault(username, [])
    if award.sheet and award.col and award.row:
        key = (award.sheet, award.col, award.row)
        if any(a.sheet == key[0] and a.col == key[1] and a.row == key[2] for a in existing):
            return
    elif any(a.category == award.category and a.name == award.name for a in existing):
        return
    existing.append(award)


def build_awards_data(columns: Iterable[dict] | None = None) -> AwardsData:
    columns = list(columns or load_columns())
    sheet_rows: dict[str, list[list[str]]] = {}
    for sheet_name in SHEET_META:
        sheet_rows[sheet_name] = fetch_sheet(sheet_name)

    index: dict[str, list[Award]] = {}
    catalog: list[AwardDef] = []
    seen_defs: set[tuple[str, str]] = set()

    for entry in columns:
        sheet = entry["sheet"]
        meta = SHEET_META.get(sheet)
        rows = sheet_rows.get(sheet)
        if not meta or not rows:
            continue

        col = entry["col"]
        col_idx = col_to_index(col)
        base_name = ""
        name_row = rows[meta["name_row"] - 1] if len(rows) >= meta["name_row"] else []
        if col_idx < len(name_row):
            base_name = (name_row[col_idx] or "").strip()
        if not base_name:
            continue

        key = (sheet, col)
        if key not in seen_defs:
            seen_defs.add(key)
            catalog.append(
                AwardDef(
                    category=meta["category"],
                    sheet=sheet,
                    col=col,
                    base_name=base_name,
                )
            )

        for r in range(meta["data_start_row"] - 1, len(rows)):
            row = rows[r]
            cell = row[col_idx] if col_idx < len(row) else ""
            if not cell or not str(cell).strip():
                continue
            cell = str(cell)
            username = normalize_username(cell)
            name = format_award_name(meta["category"], base_name, cell)
            if name:
                add_award(
                    index,
                    username,
                    Award(
                        category=meta["category"],
                        name=name,
                        sheet=sheet,
                        col=col,
                        row=csv_index_to_sheet_row(sheet, r),
                        cell=cell.strip(),
                        base_name=base_name,
                    ),
                )

    catalog.sort(key=lambda d: (d.category, d.base_name.casefold()))
    return AwardsData(index=index, catalog=catalog, sheet_rows=sheet_rows)


def build_awards_index(columns: Iterable[dict] | None = None) -> dict[str, list[Award]]:
    return build_awards_data(columns).index


def get_awards_for_username(index: dict[str, list[Award]], username: str) -> list[Award]:
    key = normalize_username(username) or username.strip().lstrip("@").lower()
    return list(index.get(key, []))


def group_awards(awards: list[Award]) -> dict[str, list[str]]:
    grouped = {label: [] for label in CATEGORY_LABELS.values()}
    for award in awards:
        label = CATEGORY_LABELS.get(award.category, award.category.title())
        grouped.setdefault(label, []).append(award.name)
    for names in grouped.values():
        names.sort(key=str.casefold)
    return grouped


def flatten_awards_sorted(awards: list[Award]) -> list[Award]:
    """Category order then name — used for selectable TUI lists."""
    order = {cat: i for i, cat in enumerate(CATEGORY_LABELS)}
    return sorted(
        awards,
        key=lambda a: (order.get(a.category, 99), a.name.casefold()),
    )
