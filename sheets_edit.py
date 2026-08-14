"""Write helpers for adding, editing, and removing awards in the sheet."""

from __future__ import annotations

from dataclasses import dataclass

from awards import (
    SHEET_ID,
    SHEET_META,
    Award,
    AwardDef,
    col_to_index,
    csv_index_to_sheet_row,
    format_award_name,
    normalize_username,
    sheet_data_start_row,
)
from sheets_auth import AuthError, build_sheets_service


@dataclass
class EditResult:
    ok: bool
    message: str
    award: Award | None = None


def _a1(sheet: str, col: str, row: int) -> str:
    # Sheet names with spaces need quotes.
    return f"'{sheet}'!{col}{row}"


def _values_api(service):
    return service.spreadsheets().values()


def build_cell_value(username: str, suffix: str = "") -> str:
    """Build sheet cell text: Username or Username x2 / Username - detail."""
    user = username.strip().lstrip("@")
    suffix = (suffix or "").strip()
    if not suffix:
        return user
    if suffix.lower().startswith("x") and suffix[1:].isdigit():
        return f"{user} {suffix}"
    if suffix.startswith("-"):
        return f"{user} {suffix}"
    return f"{user} - {suffix}"


def find_user_cell(
    rows: list[list[str]],
    sheet: str,
    col: str,
    username: str,
) -> int | None:
    """Return 1-based row of username in column, or None."""
    meta = SHEET_META[sheet]
    col_idx = col_to_index(col)
    key = normalize_username(username)
    for r in range(meta["data_start_row"] - 1, len(rows)):
        row = rows[r]
        cell = row[col_idx] if col_idx < len(row) else ""
        if normalize_username(str(cell) if cell else "") == key:
            return csv_index_to_sheet_row(sheet, r)
    return None


def find_first_empty_row(rows: list[list[str]], sheet: str, col: str) -> int:
    meta = SHEET_META[sheet]
    col_idx = col_to_index(col)
    start = meta["data_start_row"] - 1
    last_filled = start - 1
    for r in range(start, len(rows)):
        row = rows[r]
        cell = row[col_idx] if col_idx < len(row) else ""
        if cell and str(cell).strip():
            last_filled = r
            continue
        return csv_index_to_sheet_row(sheet, r)
    return csv_index_to_sheet_row(sheet, last_filled + 1)


def add_award_to_user(
    *,
    username: str,
    award_def: AwardDef,
    suffix: str = "",
    rows: list[list[str]] | None = None,
    interactive_auth: bool = True,
) -> EditResult:
    user = username.strip().lstrip("@")
    if not user:
        return EditResult(False, "Username required")

    key = normalize_username(user)
    if rows is not None:
        existing = find_user_cell(rows, award_def.sheet, award_def.col, user)
        if existing:
            return EditResult(False, f"@{user} already has {award_def.base_name} (row {existing})")

    cell_value = build_cell_value(user, suffix)
    try:
        service = build_sheets_service(interactive=interactive_auth)
    except AuthError as exc:
        return EditResult(False, str(exc))

    # Re-read column via API for accurate empty-row placement when possible.
    target_row = None
    try:
        col_range = f"'{award_def.sheet}'!{award_def.col}:{award_def.col}"
        result = _values_api(service).get(spreadsheetId=SHEET_ID, range=col_range).execute()
        col_vals = result.get("values") or []
        meta = SHEET_META[award_def.sheet]
        start = sheet_data_start_row(award_def.sheet)
        for i in range(start - 1, len(col_vals)):
            cell = col_vals[i][0] if col_vals[i] else ""
            if normalize_username(str(cell)) == key:
                return EditResult(False, f"@{user} already has {award_def.base_name} (row {i + 1})")
        for i in range(start - 1, len(col_vals)):
            cell = col_vals[i][0] if col_vals[i] else ""
            if not str(cell).strip():
                target_row = i + 1
                break
        if target_row is None:
            target_row = max(len(col_vals) + 1, start)
    except Exception:
        if rows is not None:
            target_row = find_first_empty_row(rows, award_def.sheet, award_def.col)
        else:
            target_row = sheet_data_start_row(award_def.sheet)

    a1 = _a1(award_def.sheet, award_def.col, target_row)
    try:
        _values_api(service).update(
            spreadsheetId=SHEET_ID,
            range=a1,
            valueInputOption="USER_ENTERED",
            body={"values": [[cell_value]]},
        ).execute()
    except Exception as exc:  # noqa: BLE001
        return EditResult(False, f"Write failed: {exc}")

    display = format_award_name(award_def.category, award_def.base_name, cell_value) or award_def.base_name
    award = Award(
        category=award_def.category,
        name=display,
        sheet=award_def.sheet,
        col=award_def.col,
        row=target_row,
        cell=cell_value,
        base_name=award_def.base_name,
    )
    return EditResult(True, f"Added {display} for @{user} at {award_def.col}{target_row}", award)


def update_award_cell(
    award: Award,
    new_cell: str,
    *,
    interactive_auth: bool = True,
) -> EditResult:
    if not award.sheet or not award.col or not award.row:
        return EditResult(False, "Award has no sheet location (refresh and try again)")
    new_cell = new_cell.strip()
    if not new_cell:
        return EditResult(False, "Cell value cannot be empty (use delete instead)")
    if not normalize_username(new_cell):
        return EditResult(False, "Cell must start with a username")

    try:
        service = build_sheets_service(interactive=interactive_auth)
        a1 = _a1(award.sheet, award.col, award.row)
        _values_api(service).update(
            spreadsheetId=SHEET_ID,
            range=a1,
            valueInputOption="USER_ENTERED",
            body={"values": [[new_cell]]},
        ).execute()
    except AuthError as exc:
        return EditResult(False, str(exc))
    except Exception as exc:  # noqa: BLE001
        return EditResult(False, f"Update failed: {exc}")

    display = format_award_name(award.category, award.base_name or award.name, new_cell) or award.name
    updated = Award(
        category=award.category,
        name=display,
        sheet=award.sheet,
        col=award.col,
        row=award.row,
        cell=new_cell,
        base_name=award.base_name,
    )
    return EditResult(True, f"Updated {award.col}{award.row} → {new_cell}", updated)


def remove_award(
    award: Award,
    *,
    interactive_auth: bool = True,
) -> EditResult:
    if not award.sheet or not award.col or not award.row:
        return EditResult(False, "Award has no sheet location (refresh and try again)")
    try:
        service = build_sheets_service(interactive=interactive_auth)
        a1 = _a1(award.sheet, award.col, award.row)
        _values_api(service).clear(
            spreadsheetId=SHEET_ID,
            range=a1,
            body={},
        ).execute()
    except AuthError as exc:
        return EditResult(False, str(exc))
    except Exception as exc:  # noqa: BLE001
        return EditResult(False, f"Delete failed: {exc}")

    return EditResult(True, f"Removed {award.name} from {award.col}{award.row}")


# Re-export helper used by tests / callers
__all__ = [
    "EditResult",
    "add_award_to_user",
    "build_cell_value",
    "find_first_empty_row",
    "find_user_cell",
    "remove_award",
    "update_award_cell",
]
