#!/usr/bin/env python3
"""Interactive TUI for looking up and editing awards in the Decorations Database."""

from __future__ import annotations

import curses
import locale
import os
import sys
import threading
import time
from datetime import datetime, timezone
from pathlib import Path


def _reexec_venv_if_needed() -> None:
    try:
        import googleapiclient  # noqa: F401
        return
    except ImportError:
        pass
    venv_python = Path(__file__).resolve().parent / ".venv" / "bin" / "python"
    if not venv_python.is_file():
        return
    if Path(sys.executable).resolve() == venv_python.resolve():
        return
    os.execv(str(venv_python), [str(venv_python), *sys.argv])


if __name__ == "__main__":
    _reexec_venv_if_needed()

from awards import (
    CATEGORY_LABELS,
    Award,
    AwardDef,
    AwardsData,
    build_awards_data,
    col_to_index,
    find_duplicates_for_user,
    flatten_awards_sorted,
    get_awards_for_username,
    normalize_username,
    row_offset,
)
from sheets_auth import auth_status
from sheets_edit import add_award_to_user, remove_award, update_award_cell


class AwardsTUI:
    def __init__(self, stdscr: curses.window) -> None:
        self.stdscr = stdscr
        self.query = ""
        self.status = "Loading awards from Google Sheets…"
        self.error: str | None = None
        self.data: AwardsData | None = None
        self.synced_at: str | None = None
        self.scroll = 0
        self.results_username: str | None = None
        self.results: list[Award] = []
        self.duplicates: list[Award] = []
        self.selected = 0
        self.focus = "search"  # search | list
        self.mode = "main"  # main | add | edit | confirm_delete
        self.modal_input = ""
        self.modal_filter = ""
        self.cursor = 0
        self.add_candidates: list[AwardDef] = []
        self.add_selected = 0
        self.add_step = "pick"  # pick | suffix
        self.pending_award_def: AwardDef | None = None
        self._load_lock = threading.Lock()
        self._loading = False
        self._busy = False

    def all_entries(self) -> list[Award]:
        return self.results + self.duplicates

    def _clamp_cursor(self, text: str) -> None:
        self.cursor = max(0, min(self.cursor, len(text)))

    def _apply_text_key(self, key: int, text: str) -> tuple[str, bool]:
        """Insert/delete/move in `text` at `self.cursor`. Returns (text, handled)."""
        self._clamp_cursor(text)
        if key == curses.KEY_LEFT:
            self.cursor = max(0, self.cursor - 1)
            return text, True
        if key == curses.KEY_RIGHT:
            self.cursor = min(len(text), self.cursor + 1)
            return text, True
        if key in (curses.KEY_HOME, 1):  # Home / Ctrl+A
            self.cursor = 0
            return text, True
        if key in (curses.KEY_END, 5):  # End / Ctrl+E
            self.cursor = len(text)
            return text, True
        if key == curses.KEY_BACKSPACE or key in (127, 8):
            if self.cursor > 0:
                text = text[: self.cursor - 1] + text[self.cursor :]
                self.cursor -= 1
            return text, True
        if key == curses.KEY_DC:
            if self.cursor < len(text):
                text = text[: self.cursor] + text[self.cursor + 1 :]
            return text, True
        if 32 <= key <= 126:
            text = text[: self.cursor] + chr(key) + text[self.cursor :]
            self.cursor += 1
            return text, True
        return text, False

    def _place_cursor(self, y: int, x: int, text: str, field_x: int, max_x: int) -> None:
        self._clamp_cursor(text)
        curs_x = field_x + self.cursor
        if y >= 0 and field_x <= curs_x < max_x:
            try:
                self.stdscr.move(y, curs_x)
            except curses.error:
                pass

    def start(self) -> None:
        for loc in ("en_US.UTF-8", "C.UTF-8", ""):
            try:
                locale.setlocale(locale.LC_ALL, loc)
                break
            except locale.Error:
                continue
        curses.curs_set(1)
        curses.use_default_colors()
        if curses.has_colors():
            curses.init_pair(1, curses.COLOR_CYAN, -1)
            curses.init_pair(2, curses.COLOR_GREEN, -1)
            curses.init_pair(3, curses.COLOR_YELLOW, -1)
            curses.init_pair(4, curses.COLOR_RED, -1)
            curses.init_pair(5, curses.COLOR_WHITE, curses.COLOR_BLUE)
            curses.init_pair(6, curses.COLOR_BLACK, curses.COLOR_CYAN)
            curses.init_pair(7, curses.COLOR_MAGENTA, -1)
        self.stdscr.nodelay(True)
        self.stdscr.keypad(True)
        self.refresh_data(async_=True)

        while True:
            self.draw()
            try:
                key = self.stdscr.getch()
            except curses.error:
                key = -1

            if key == -1:
                time.sleep(0.03)
                continue

            if self.mode == "add":
                if self._handle_add(key):
                    break
                continue
            if self.mode == "edit":
                if self._handle_edit(key):
                    break
                continue
            if self.mode == "confirm_delete":
                if self._handle_confirm_delete(key):
                    break
                continue
            if self._handle_main(key):
                break

    def _handle_main(self, key: int) -> bool:
        """Return True to quit."""
        if key == 9:  # Tab
            if self.results or self.duplicates:
                self.focus = "list" if self.focus == "search" else "search"
            return False

        if self.focus == "list" and self.results_username:
            entries = self.all_entries()
            if key == curses.KEY_UP and entries:
                self.selected = max(0, self.selected - 1)
                self._ensure_selected_visible()
                return False
            if key == curses.KEY_DOWN and entries:
                self.selected = min(len(entries) - 1, self.selected + 1)
                self._ensure_selected_visible()
                return False
            if key in (ord("a"), ord("A")):
                self._open_add()
                return False
            if key in (ord("e"), ord("E")):
                if self.all_entries():
                    self._open_edit()
                return False
            if key in (ord("d"), ord("D")):
                if self._busy:
                    self.status = "Wait for the current sheet operation to finish"
                    return False
                if self.all_entries():
                    self.modal_input = ""
                    self.cursor = 0
                    self.mode = "confirm_delete"
                return False
            if key == ord("/"):
                self.focus = "search"
                return False

        if key in (ord("q"),) and self.focus == "search" and not self.query:
            return True
        if key == 27:
            if self.query:
                self.query = ""
                self.cursor = 0
                self.results = []
                self.duplicates = []
                self.results_username = None
                self.scroll = 0
                self.selected = 0
                self.focus = "search"
            else:
                return True
        elif key in (curses.KEY_ENTER, 10, 13):
            if self.focus == "search":
                self.lookup()
            return False
        elif key == curses.KEY_BACKSPACE or key in (127, 8, curses.KEY_DC, curses.KEY_LEFT, curses.KEY_RIGHT, curses.KEY_HOME, curses.KEY_END) or key in (1, 5):
            if self.focus == "search":
                self.query, _ = self._apply_text_key(key, self.query)
            return False
        elif key == curses.KEY_UP and self.focus == "search":
            self.scroll = max(0, self.scroll - 1)
            return False
        elif key == curses.KEY_DOWN and self.focus == "search":
            self.scroll += 1
            return False
        elif key == curses.KEY_PPAGE:
            self.scroll = max(0, self.scroll - 10)
            return False
        elif key == curses.KEY_NPAGE:
            self.scroll += 10
            return False
        elif key == curses.KEY_F5 or key == 18:
            if self._busy:
                self.status = "Wait for the current sheet operation to finish"
                return False
            self.refresh_data(async_=True)
            return False
        elif 32 <= key <= 126:
            if self.focus == "list":
                # Letters that aren't commands fall through to search.
                ch = chr(key)
                if ch.lower() in ("a", "e", "d"):
                    return False
                self.focus = "search"
                self.cursor = len(self.query)
                self.query, _ = self._apply_text_key(key, self.query)
            else:
                self.query, _ = self._apply_text_key(key, self.query)
        return False

    def _open_add(self) -> None:
        if self._busy:
            self.status = "Wait for the current sheet operation to finish"
            return
        if not self.results_username:
            self.status = "Look up a user before adding awards"
            return
        if not self.data:
            self.status = "Still loading…"
            return
        owned = {(a.sheet, a.col) for a in self.all_entries()}
        self.add_candidates = [d for d in self.data.catalog if (d.sheet, d.col) not in owned]
        self.add_selected = 0
        self.modal_filter = ""
        self.add_step = "pick"
        self.pending_award_def = None
        self.modal_input = ""
        self.cursor = 0
        self.mode = "add"
        self.error = None
        self.status = f"Add award for @{self.results_username}"

    def _filtered_add_candidates(self) -> list[AwardDef]:
        q = self.modal_filter.strip().casefold()
        if not q:
            return self.add_candidates
        return [
            d
            for d in self.add_candidates
            if q in d.base_name.casefold() or q in d.category or q in CATEGORY_LABELS.get(d.category, "").casefold()
        ]

    def _handle_add(self, key: int) -> bool:
        if self.add_step == "suffix":
            if key == 27:
                self.add_step = "pick"
                self.modal_input = ""
                self.cursor = 0
                return False
            if key in (curses.KEY_ENTER, 10, 13):
                self._commit_add(self.modal_input)
                return False
            self.modal_input, handled = self._apply_text_key(key, self.modal_input)
            return False

        # pick step
        filtered = self._filtered_add_candidates()
        if key == 27:
            self.mode = "main"
            self.status = f"{self.results_username} · {len(self.results)} award(s)" if self.results_username else self.status
            return False
        if key == curses.KEY_UP:
            self.add_selected = max(0, self.add_selected - 1)
            return False
        if key == curses.KEY_DOWN:
            self.add_selected = min(max(0, len(filtered) - 1), self.add_selected + 1)
            return False
        if key in (curses.KEY_ENTER, 10, 13):
            if not filtered:
                self.status = "No matching awards"
                return False
            self.pending_award_def = filtered[self.add_selected]
            self.add_step = "suffix"
            self.modal_input = ""
            self.cursor = 0
            self.status = "Optional suffix (e.g. x2 or Master) — Enter to save blank"
            return False
        self.modal_filter, handled = self._apply_text_key(key, self.modal_filter)
        if handled and key not in (curses.KEY_LEFT, curses.KEY_RIGHT, curses.KEY_HOME, curses.KEY_END, 1, 5):
            self.add_selected = 0
        return False

    def _commit_add(self, suffix: str) -> None:
        if not self.pending_award_def or not self.results_username or not self.data:
            self.mode = "main"
            return
        if self._busy:
            return
        award_def = self.pending_award_def
        username = self.results_username
        rows = self.data.sheet_rows.get(award_def.sheet)

        def worker() -> None:
            self._busy = True
            self.status = f"Writing {award_def.base_name}…"
            self.error = None
            try:
                result = add_award_to_user(
                    username=username,
                    award_def=award_def,
                    suffix=suffix,
                    rows=rows,
                    interactive_auth=False,
                )
                if result.ok and result.award:
                    if self.data:
                        bucket = self.data.index.setdefault(username.lower(), [])
                        bucket.append(result.award)
                        self._patch_sheet_cell(
                            result.award.sheet,
                            result.award.col,
                            result.award.row,
                            result.award.cell,
                        )
                    self._sync_user_view(select_award=result.award)
                    self.status = result.message
                    self.mode = "main"
                    self.focus = "list"
                else:
                    self.error = result.message
                    self.status = "Add failed"
                    self.mode = "main"
            finally:
                self._busy = False

        threading.Thread(target=worker, daemon=True).start()

    def _open_edit(self) -> None:
        if self._busy:
            self.status = "Wait for the current sheet operation to finish"
            return
        entries = self.all_entries()
        if not entries:
            return
        award = entries[self.selected]
        self.modal_input = award.cell or award.name
        self.cursor = len(self.modal_input)
        self.mode = "edit"
        self.status = f"Edit row {award.row} (must start with username)"

    def _handle_edit(self, key: int) -> bool:
        if key == 27:
            self.mode = "main"
            self.status = f"{self.results_username} · {len(self.results)} award(s)"
            return False
        if key in (curses.KEY_ENTER, 10, 13):
            self._commit_edit()
            return False
        self.modal_input, _ = self._apply_text_key(key, self.modal_input)
        return False

    def _commit_edit(self) -> None:
        entries = self.all_entries()
        if not entries or self._busy:
            return
        award = entries[self.selected]
        new_cell = self.modal_input

        def worker() -> None:
            self._busy = True
            self.status = "Updating sheet…"
            self.error = None
            try:
                result = update_award_cell(award, new_cell, interactive_auth=False)
                if result.ok and result.award:
                    if self.data and self.results_username:
                        key = self.results_username.lower()
                        self.data.index[key] = [
                            a
                            for a in self.data.index.get(key, [])
                            if not (a.sheet == award.sheet and a.col == award.col and a.row == award.row)
                        ]
                        self.data.index[key].append(result.award)
                        self._patch_sheet_cell(
                            result.award.sheet,
                            result.award.col,
                            result.award.row,
                            result.award.cell,
                        )
                    self._sync_user_view(select_award=result.award)
                    self.status = result.message
                else:
                    self.error = result.message
                    self.status = "Edit failed"
                self.mode = "main"
            finally:
                self._busy = False

        threading.Thread(target=worker, daemon=True).start()

    def _handle_confirm_delete(self, key: int) -> bool:
        if key == 27:
            self.modal_input = ""
            self.mode = "main"
            self.status = f"{self.results_username} · {len(self.results)} award(s)"
            return False
        if key in (curses.KEY_ENTER, 10, 13):
            if self.modal_input.strip().lower() == "delete":
                self.modal_input = ""
                self.cursor = 0
                self._commit_delete()
            else:
                self.status = 'Type "delete" and press Enter to confirm'
            return False
        self.modal_input, _ = self._apply_text_key(key, self.modal_input)
        return False

    def _commit_delete(self) -> None:
        entries = self.all_entries()
        if not entries or self._busy:
            return
        award = entries[self.selected]

        def worker() -> None:
            self._busy = True
            self.status = f"Removing {award.name}…"
            self.error = None
            try:
                result = remove_award(award, interactive_auth=False)
                if result.ok:
                    if self.data and self.results_username:
                        key = self.results_username.lower()
                        self.data.index[key] = [
                            a
                            for a in self.data.index.get(key, [])
                            if not (a.sheet == award.sheet and a.col == award.col and a.row == award.row)
                        ]
                        self._patch_sheet_cell(award.sheet, award.col, award.row, "")
                    self._sync_user_view(preserve_selection=True)
                    self.status = result.message
                else:
                    self.error = result.message
                    self.status = "Delete failed"
                self.mode = "main"
            finally:
                self._busy = False

        threading.Thread(target=worker, daemon=True).start()

    def refresh_data(self, async_: bool = True) -> None:
        if self._loading or self._busy:
            return

        def worker() -> None:
            with self._load_lock:
                self._loading = True
                self.status = "Syncing Badges / Ribbons / Foreign Awards…"
                self.error = None
                try:
                    data = build_awards_data()
                    self.data = data
                    self.synced_at = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M:%S UTC")
                    auth = auth_status()
                    auth_note = {
                        "service_account": "write: service account",
                        "oauth_token": "write: logged in",
                        "oauth_needs_login": "write: run --login",
                        "missing": "write: no credentials",
                    }.get(auth, auth)
                    self.status = f"Ready · {len(data.index)} users · {auth_note}"
                    if self.results_username:
                        self.lookup(silent=True, preserve_view=True)
                except Exception as exc:  # noqa: BLE001
                    self.error = str(exc)
                    self.status = "Sync failed"
                finally:
                    self._loading = False

        if async_:
            threading.Thread(target=worker, daemon=True).start()
        else:
            worker()

    def lookup(self, silent: bool = False, preserve_view: bool = False) -> None:
        username = normalize_username(self.query) or self.query.strip().lstrip("@")
        if not username:
            if not silent:
                self.status = "Enter a username"
            return
        if self.data is None:
            self.status = "Still loading awards…"
            return
        prev_selected = self.selected
        prev_scroll = self.scroll
        awards = flatten_awards_sorted(get_awards_for_username(self.data.index, username))
        dup_hits = find_duplicates_for_user(self.data, username)
        self.results_username = username
        self.results = awards
        self.duplicates = [h.to_award() for h in dup_hits]
        if preserve_view:
            self.selected = min(prev_selected, max(0, len(self.all_entries()) - 1))
            self.scroll = prev_scroll
        else:
            self.scroll = 0
            self.selected = 0
        self.focus = "list" if (awards or self.duplicates) else "search"
        dup_note = f" · {len(self.duplicates)} duplicate(s)" if self.duplicates else ""
        if awards or self.duplicates:
            self.status = f"{username} · {len(awards)} award(s){dup_note} · Tab list/search · a/e/d"
        else:
            self.status = f"No awards for {username} · press a to add"
            self.focus = "list"

    def _patch_sheet_cell(self, sheet: str, col: str, row: int, value: str) -> None:
        """Keep cached sheet rows aligned with local edits between refreshes."""
        if not self.data:
            return
        rows = self.data.sheet_rows.setdefault(sheet, [])
        csv_index = row - 1 - row_offset(sheet)
        col_idx = col_to_index(col)
        if csv_index < 0:
            return
        while len(rows) <= csv_index:
            rows.append([])
        while len(rows[csv_index]) <= col_idx:
            rows[csv_index].append("")
        rows[csv_index][col_idx] = value

    def _sync_user_view(
        self,
        *,
        preserve_selection: bool = True,
        select_award: Award | None = None,
    ) -> None:
        """Refresh results and duplicate sections for the current user."""
        if not self.results_username or not self.data:
            return
        prev_selected = self.selected
        prev_scroll = self.scroll
        username = self.results_username
        self.results = flatten_awards_sorted(
            get_awards_for_username(self.data.index, username),
        )
        self.duplicates = [h.to_award() for h in find_duplicates_for_user(self.data, username)]
        if select_award:
            entries = self.all_entries()
            self.selected = next(
                (
                    i
                    for i, a in enumerate(entries)
                    if a.sheet == select_award.sheet
                    and a.col == select_award.col
                    and a.row == select_award.row
                ),
                min(prev_selected, max(0, len(entries) - 1)),
            )
        elif preserve_selection:
            self.selected = min(prev_selected, max(0, len(self.all_entries()) - 1))
            self.scroll = prev_scroll
        self._ensure_selected_visible()

    def _ensure_selected_visible(self) -> None:
        # Selection index maps to award entries in flattened list; scroll tracks line view.
        # Keep simple: scroll so selected award line is on screen approximately.
        # Lines are header + categories; approximate by selected offset.
        target = 2 + self.selected
        h, _ = self.stdscr.getmaxyx()
        view_h = max(1, h - 9)
        if target < self.scroll:
            self.scroll = target
        elif target >= self.scroll + view_h:
            self.scroll = target - view_h + 1

    def draw(self) -> None:
        self.stdscr.erase()
        h, w = self.stdscr.getmaxyx()
        if h < 10 or w < 48:
            self._addstr(0, 0, "Terminal too small", curses.A_BOLD)
            self.stdscr.refresh()
            return

        if self.mode == "add":
            self._draw_add(h, w)
            self.stdscr.refresh()
            return
        if self.mode == "edit":
            self._draw_edit(h, w)
            self.stdscr.refresh()
            return
        if self.mode == "confirm_delete":
            self._draw_confirm_delete(h, w)
            self.stdscr.refresh()
            return

        title = " Decorations Database · Awards Editor "
        self._addstr(0, max(0, (w - len(title)) // 2), title, curses.color_pair(5) | curses.A_BOLD)

        sync = f" Synced: {self.synced_at or 'never'} "
        self._addstr(1, 1, sync[: w - 2], curses.color_pair(1))

        prompt = " Username: "
        box = f"{prompt}{self.query}"
        search_attr = curses.A_BOLD | (curses.color_pair(6) if self.focus == "search" else 0)
        self._addstr(3, 1, "┌" + "─" * (w - 4) + "┐")
        self._addstr(4, 1, "│" + box[: w - 4].ljust(w - 4) + "│", search_attr)
        self._addstr(5, 1, "└" + "─" * (w - 4) + "┘")

        help_line = "Enter lookup · Tab focus · a add · e edit · d delete · F5 refresh · q quit"
        self._addstr(h - 2, 1, help_line[: w - 2], curses.color_pair(3))

        status = self.error or self.status
        attr = curses.color_pair(4) if self.error else curses.color_pair(2)
        self._addstr(h - 1, 0, status[:w].ljust(w), attr | curses.A_BOLD)

        lines = self._result_lines()
        view_top = 7
        view_h = max(0, h - 9)
        if self.scroll > max(0, len(lines) - view_h):
            self.scroll = max(0, len(lines) - view_h)
        visible = lines[self.scroll : self.scroll + view_h]
        for i, (text, style) in enumerate(visible):
            self._addstr(view_top + i, 2, text[: w - 3], style)

        if self.focus == "search":
            self._place_cursor(4, 2 + len(prompt), self.query, 2 + len(prompt), w - 1)

        self.stdscr.refresh()

    def _draw_add(self, h: int, w: int) -> None:
        title = f" Add award · @{self.results_username} "
        self._addstr(0, max(0, (w - len(title)) // 2), title, curses.color_pair(5) | curses.A_BOLD)
        if self.add_step == "suffix":
            name = self.pending_award_def.base_name if self.pending_award_def else ""
            self._addstr(2, 2, f"Award: {name}"[: w - 3], curses.A_BOLD)
            self._addstr(4, 2, "Suffix (optional):", curses.color_pair(1))
            self._addstr(5, 2, self.modal_input[: w - 4], curses.A_BOLD)
            self._addstr(7, 2, "Examples: x2   Master   \"Bronze Oak Leaf\"", curses.A_DIM)
            self._addstr(h - 2, 1, "←/→ move · Backspace/Del · Enter save · Esc back", curses.color_pair(3))
        else:
            self._addstr(2, 2, f"Filter: {self.modal_filter}"[: w - 3], curses.A_BOLD)
            filtered = self._filtered_add_candidates()
            if self.add_selected >= len(filtered):
                self.add_selected = max(0, len(filtered) - 1)
            view_top = 4
            view_h = max(1, h - 7)
            start = max(0, self.add_selected - view_h + 1) if self.add_selected >= view_h else 0
            for i, d in enumerate(filtered[start : start + view_h]):
                label = f"[{CATEGORY_LABELS.get(d.category, d.category)}] {d.base_name}"
                attr = curses.color_pair(6) | curses.A_BOLD if start + i == self.add_selected else curses.A_NORMAL
                self._addstr(view_top + i, 2, label[: w - 3], attr)
            self._addstr(h - 2, 1, "Type to filter · ↑/↓ · Enter · Esc cancel", curses.color_pair(3))
        status = self.error or self.status
        attr = curses.color_pair(4) if self.error else curses.color_pair(2)
        self._addstr(h - 1, 0, status[:w].ljust(w), attr | curses.A_BOLD)
        if self.add_step == "suffix":
            self._place_cursor(5, 2, self.modal_input, 2, w - 1)
        else:
            self._place_cursor(2, 2 + len("Filter: "), self.modal_filter, 2 + len("Filter: "), w - 1)

    def _draw_edit(self, h: int, w: int) -> None:
        entries = self.all_entries()
        award = entries[self.selected] if entries else None
        title = " Edit award cell "
        self._addstr(0, max(0, (w - len(title)) // 2), title, curses.color_pair(5) | curses.A_BOLD)
        if award:
            self._addstr(2, 2, f"{award.name}"[: w - 3], curses.A_BOLD)
            self._addstr(3, 2, f"{award.sheet} · row {award.row}"[: w - 3], curses.color_pair(1))
        self._addstr(5, 2, "Cell value:", curses.A_DIM)
        self._addstr(6, 2, self.modal_input[: w - 4], curses.A_BOLD)
        self._addstr(8, 2, 'Format: Username   or   Username x2   or   Username - detail', curses.A_DIM)
        self._addstr(h - 2, 1, "←/→ move · Home/End · Backspace/Del · Enter save · Esc cancel", curses.color_pair(3))
        status = self.error or self.status
        attr = curses.color_pair(4) if self.error else curses.color_pair(2)
        self._addstr(h - 1, 0, status[:w].ljust(w), attr | curses.A_BOLD)
        self._place_cursor(6, 2, self.modal_input, 2, w - 1)

    def _draw_confirm_delete(self, h: int, w: int) -> None:
        entries = self.all_entries()
        award = entries[self.selected] if entries else None
        title = " Confirm delete "
        self._addstr(0, max(0, (w - len(title)) // 2), title, curses.color_pair(5) | curses.A_BOLD)
        name = award.name if award else "?"
        loc = f"row {award.row}" if award else ""
        self._addstr(4, 2, f"Remove {name} ({loc}) from @{self.results_username}?"[: w - 3], curses.color_pair(4) | curses.A_BOLD)
        self._addstr(6, 2, 'Type "delete" to confirm:', curses.A_DIM)
        self._addstr(7, 2, self.modal_input[: w - 4], curses.A_BOLD)
        self._addstr(h - 2, 1, "Enter confirm · Esc cancel", curses.color_pair(3))
        self._addstr(h - 1, 0, (self.error or self.status or "")[:w].ljust(w), curses.color_pair(3))
        self._place_cursor(7, 2, self.modal_input, 2, w - 1)

    def _result_lines(self) -> list[tuple[str, int]]:
        if self.results_username is None:
            return [
                ("Type a username and press Enter.", curses.A_DIM),
                ("Then Tab to the list: a=add, e=edit, d=delete.", curses.A_DIM),
                ("Writes need OAuth: python3 main.py --login", curses.color_pair(7)),
            ]
        if not self.results and not self.duplicates:
            return [
                (f"No awards listed for @{self.results_username}.", curses.color_pair(3)),
                ("Press a to add an award (list focus).", curses.A_DIM),
            ]

        lines: list[tuple[str, int]] = [
            (f"Awards for @{self.results_username}", curses.A_BOLD | curses.color_pair(1)),
            ("", curses.A_NORMAL),
        ]
        award_idx = 0

        def append_award_block(items: list[Award], section_title: str | None, warn: bool) -> None:
            nonlocal award_idx
            if not items:
                return
            if section_title:
                lines.append((section_title, curses.A_BOLD | curses.color_pair(4 if warn else 2)))
            current_cat = None
            for award in items:
                label = CATEGORY_LABELS.get(award.category, award.category)
                if section_title is None:
                    if award.category != current_cat:
                        current_cat = award.category
                        count = sum(1 for a in items if a.category == award.category)
                        lines.append((f"▸ {label} ({count})", curses.A_BOLD | curses.color_pair(2)))
                selected = self.focus == "list" and award_idx == self.selected
                prefix = "➤ " if selected else "  • "
                if warn:
                    attr = curses.color_pair(4) | (curses.A_BOLD if selected else 0)
                else:
                    attr = curses.color_pair(6) | curses.A_BOLD if selected else curses.A_NORMAL
                loc = f"  · row {award.row}" if award.row else ""
                cell_note = f'  raw: "{award.cell}"' if warn and award.cell else ""
                lines.append((f"{prefix}{award.name}{loc}{cell_note}", attr))
                award_idx += 1
            if section_title:
                lines.append(("", curses.A_NORMAL))

        append_award_block(self.results, None, False)
        if self.duplicates:
            append_award_block(
                self.duplicates,
                f"▸ Duplicates / typos ({len(self.duplicates)})",
                True,
            )
        return lines

    def _addstr(self, y: int, x: int, text: str, attr: int = 0) -> None:
        h, w = self.stdscr.getmaxyx()
        if y < 0 or y >= h or x >= w:
            return
        try:
            self.stdscr.addnstr(y, x, text, max(0, w - x - 1), attr)
        except curses.error:
            pass


def main() -> None:
    curses.wrapper(lambda stdscr: AwardsTUI(stdscr).start())


if __name__ == "__main__":
    main()
