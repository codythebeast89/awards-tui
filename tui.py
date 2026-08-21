#!/usr/bin/env python3
"""Interactive Textual TUI for looking up and editing awards in the Decorations Database."""

from __future__ import annotations

import os
import sys
from datetime import datetime, timezone
from pathlib import Path


def _reexec_venv_if_needed() -> None:
    try:
        import googleapiclient  # noqa: F401
        import textual  # noqa: F401
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

from textual import on, work
from textual.app import App, ComposeResult
from textual.binding import Binding
from textual.containers import Horizontal, Vertical, VerticalScroll
from textual.screen import ModalScreen
from textual.widgets import (
    Button,
    Footer,
    Header,
    Input,
    Label,
    OptionList,
    Static,
    TabbedContent,
    TabPane,
)
from textual.widgets.option_list import Option
from rich.text import Text

from awards import (
    CATEGORY_LABELS,
    ROOT,
    Award,
    AwardDef,
    AwardsData,
    awards_excluding_duplicate_rows,
    build_awards_data,
    col_to_index,
    collect_sheet_audit,
    find_duplicates_for_user,
    flatten_awards_sorted,
    format_audit_report,
    get_awards_for_username,
    normalize_username,
    owned_award_columns,
    reindex_column_after_delete,
    row_offset,
    shift_column_up_in_rows,
    upsert_award_in_index,
)
from sheets_auth import auth_status, build_sheets_service
from sheets_edit import add_award_to_user, award_with_live_row, remove_award, update_award_cell


# ---------------------------------------------------------------------------
# Modals
# ---------------------------------------------------------------------------


class AddAwardScreen(ModalScreen[tuple[AwardDef, str] | None]):
    """Pick an award (optional filter) and optional cell suffix."""

    CSS = """
    AddAwardScreen {
        align: center middle;
    }
    #add-dialog {
        width: 72;
        height: 28;
        border: thick $primary;
        background: $surface;
        padding: 1 2;
    }
    #add-dialog Label {
        margin-bottom: 1;
    }
    #add-candidates {
        height: 1fr;
        border: solid $primary-darken-2;
        margin: 1 0;
    }
    #add-actions {
        height: auto;
        align: right middle;
    }
    #add-actions Button {
        margin-left: 1;
    }
    """

    def __init__(self, candidates: list[AwardDef]) -> None:
        super().__init__()
        self._all = candidates
        self._filtered = list(candidates)
        self._step = "pick"
        self._chosen: AwardDef | None = None

    def compose(self) -> ComposeResult:
        with Vertical(id="add-dialog"):
            yield Label("Add award", id="add-title")
            yield Input(placeholder="Filter awards…", id="add-filter")
            yield OptionList(id="add-candidates")
            yield Input(placeholder="Suffix optional (x2, Master, …)", id="add-suffix")
            with Horizontal(id="add-actions"):
                yield Button("Cancel", id="add-cancel")
                yield Button("Add", variant="primary", id="add-confirm")

    def on_mount(self) -> None:
        self.query_one("#add-suffix", Input).display = False
        self._reload_candidates()
        self.query_one("#add-filter", Input).focus()

    def _reload_candidates(self) -> None:
        filt = self.query_one("#add-filter", Input).value.strip().casefold()
        self._filtered = [
            d for d in self._all if not filt or filt in d.base_name.casefold()
        ]
        opts = self.query_one("#add-candidates", OptionList)
        opts.clear_options()
        for i, d in enumerate(self._filtered):
            cat = CATEGORY_LABELS.get(d.category, d.category)
            opts.add_option(Option(f"[{cat}] {d.base_name}", id=f"cand-{i}"))

    @on(Input.Changed, "#add-filter")
    def on_filter_changed(self) -> None:
        if self._step == "pick":
            self._reload_candidates()

    @on(OptionList.OptionSelected, "#add-candidates")
    def on_candidate_selected(self, event: OptionList.OptionSelected) -> None:
        if event.option_id and event.option_id.startswith("cand-"):
            idx = int(event.option_id.split("-", 1)[1])
            if 0 <= idx < len(self._filtered):
                self._chosen = self._filtered[idx]
                self._step = "suffix"
                self.query_one("#add-title", Label).update(
                    f"Suffix for {self._chosen.base_name} (optional)"
                )
                self.query_one("#add-filter", Input).display = False
                self.query_one("#add-candidates", OptionList).display = False
                suffix = self.query_one("#add-suffix", Input)
                suffix.display = True
                suffix.focus()

    @on(Button.Pressed, "#add-cancel")
    def on_cancel(self) -> None:
        self.dismiss(None)

    @on(Button.Pressed, "#add-confirm")
    def on_confirm(self) -> None:
        if self._step == "pick":
            opts = self.query_one("#add-candidates", OptionList)
            if opts.highlighted is None or not self._filtered:
                self.app.notify("Select an award first", severity="warning")
                return
            highlighted = opts.highlighted
            if isinstance(highlighted, int) and 0 <= highlighted < len(self._filtered):
                self._chosen = self._filtered[highlighted]
            elif self._filtered:
                self._chosen = self._filtered[0]
            else:
                return
            self._step = "suffix"
            self.query_one("#add-title", Label).update(
                f"Suffix for {self._chosen.base_name} (optional)"
            )
            self.query_one("#add-filter", Input).display = False
            self.query_one("#add-candidates", OptionList).display = False
            suffix = self.query_one("#add-suffix", Input)
            suffix.display = True
            suffix.focus()
            return
        if not self._chosen:
            self.dismiss(None)
            return
        suffix = self.query_one("#add-suffix", Input).value.strip()
        self.dismiss((self._chosen, suffix))

    def on_key(self, event) -> None:  # noqa: ANN001
        if event.key == "escape":
            self.dismiss(None)
            event.stop()


class EditAwardScreen(ModalScreen[str | None]):
    """Edit the raw sheet cell value."""

    CSS = """
    EditAwardScreen {
        align: center middle;
    }
    #edit-dialog {
        width: 70;
        height: auto;
        border: thick $primary;
        background: $surface;
        padding: 1 2;
    }
    #edit-dialog Input {
        margin: 1 0;
    }
    #edit-actions {
        height: auto;
        align: right middle;
    }
    #edit-actions Button {
        margin-left: 1;
    }
    """

    def __init__(self, award: Award) -> None:
        super().__init__()
        self.award = award

    def compose(self) -> ComposeResult:
        with Vertical(id="edit-dialog"):
            yield Label(f"Edit · {self.award.name}")
            yield Label(
                f"{self.award.sheet} · {self.award.col}{self.award.row} · cell @{normalize_username(self.award.cell) or '?'}",
                classes="muted",
            )
            yield Input(
                value=self.award.cell or self.award.name,
                id="edit-cell",
            )
            yield Label('Format: Username   or   Username x2   or   Username - detail')
            with Horizontal(id="edit-actions"):
                yield Button("Cancel", id="edit-cancel")
                yield Button("Save", variant="primary", id="edit-save")

    def on_mount(self) -> None:
        self.query_one("#edit-cell", Input).focus()

    @on(Button.Pressed, "#edit-cancel")
    def on_cancel(self) -> None:
        self.dismiss(None)

    @on(Button.Pressed, "#edit-save")
    def on_save(self) -> None:
        self.dismiss(self.query_one("#edit-cell", Input).value)

    @on(Input.Submitted, "#edit-cell")
    def on_submit(self) -> None:
        self.dismiss(self.query_one("#edit-cell", Input).value)

    def on_key(self, event) -> None:  # noqa: ANN001
        if event.key == "escape":
            self.dismiss(None)
            event.stop()


class DeleteAwardScreen(ModalScreen[bool]):
    """Confirm delete by typing delete."""

    CSS = """
    DeleteAwardScreen {
        align: center middle;
    }
    #delete-dialog {
        width: 72;
        height: auto;
        border: thick $error;
        background: $surface;
        padding: 1 2;
    }
    #delete-dialog Input {
        margin: 1 0;
    }
    #delete-actions {
        height: auto;
        align: right middle;
    }
    #delete-actions Button {
        margin-left: 1;
    }
    """

    def __init__(self, award: Award, viewed_username: str) -> None:
        super().__init__()
        self.award = award
        self.viewed_username = viewed_username

    def compose(self) -> ComposeResult:
        cell_user = normalize_username(self.award.cell) or "?"
        viewed = normalize_username(self.viewed_username) or self.viewed_username
        loc = f"{self.award.col}{self.award.row}" if self.award.col else f"row {self.award.row}"
        with Vertical(id="delete-dialog"):
            if viewed and cell_user != viewed:
                yield Label(
                    f"Typo / similar name: this cell is @{cell_user}, "
                    f"not the lookup @{viewed}."
                )
                yield Label(
                    f"Remove {self.award.name} at {loc} for @{cell_user} from the sheet?"
                )
            else:
                yield Label(
                    f"Remove {self.award.name} ({loc}) from @{cell_user}?"
                )
            yield Label('Type "delete" to confirm')
            yield Input(placeholder="delete", id="delete-confirm")
            with Horizontal(id="delete-actions"):
                yield Button("Cancel", id="delete-cancel")
                yield Button("Delete", variant="error", id="delete-ok")

    def on_mount(self) -> None:
        self.query_one("#delete-confirm", Input).focus()

    @on(Button.Pressed, "#delete-cancel")
    def on_cancel(self) -> None:
        self.dismiss(False)

    @on(Button.Pressed, "#delete-ok")
    def on_ok(self) -> None:
        if self.query_one("#delete-confirm", Input).value.strip().lower() == "delete":
            self.dismiss(True)
        else:
            self.app.notify('Type "delete" to confirm', severity="warning")

    @on(Input.Submitted, "#delete-confirm")
    def on_submit(self) -> None:
        if self.query_one("#delete-confirm", Input).value.strip().lower() == "delete":
            self.dismiss(True)
        else:
            self.app.notify('Type "delete" to confirm', severity="warning")

    def on_key(self, event) -> None:  # noqa: ANN001
        if event.key == "escape":
            self.dismiss(False)
            event.stop()


# ---------------------------------------------------------------------------
# Main app
# ---------------------------------------------------------------------------


class AwardsApp(App[None]):
    """Posting-inspired purple awards editor."""

    TITLE = "awards-tui"
    SUB_TITLE = "Decorations Database"

    CSS = """
    Screen {
        background: #0c0c0f;
    }

    Header {
        background: #12121a;
        color: #e8e4f5;
        text-style: bold;
    }

    Footer {
        background: #12121a;
    }

    #top-bar {
        height: 3;
        background: #12121a;
        padding: 0 1;
        align: left middle;
    }

    #user-chip {
        width: 8;
        background: #7c3aed;
        color: #ffffff;
        text-style: bold;
        content-align: center middle;
        margin-right: 1;
    }

    #username {
        width: 1fr;
        background: #1e1b4b;
        border: tall #4c1d95;
        color: #f5f3ff;
    }

    #username:focus {
        border: tall #a78bfa;
    }

    #lookup-btn {
        margin-left: 1;
        background: #7c3aed;
        color: #ffffff;
        text-style: bold;
        border: none;
        min-width: 12;
    }

    #lookup-btn:hover {
        background: #8b5cf6;
    }

    #body {
        height: 1fr;
        padding: 0 1 0 1;
    }

    #actions-panel, #awards-panel, #detail-panel {
        border: solid #3b3358;
        background: #101018;
        height: 1fr;
        padding: 0 1;
    }

    #actions-panel {
        width: 18;
        margin-right: 1;
    }

    #awards-panel {
        width: 1fr;
        margin-right: 1;
    }

    #detail-panel {
        width: 36;
    }

    .panel-title {
        color: #a78bfa;
        text-style: bold;
        padding: 0 1;
        dock: top;
        height: 1;
        background: #101018;
    }

    #actions-list {
        height: 1fr;
        border: none;
        background: transparent;
    }

    #actions-list > .option-list--option-highlighted {
        background: #4c1d95;
        color: #f5f3ff;
    }

    TabbedContent {
        height: auto;
    }

    Tabs {
        background: #101018;
        dock: top;
    }

    Tab {
        color: #9ca3af;
    }

    Tab.-active {
        color: #a78bfa;
        text-style: bold;
    }

    Underline > .underline--bar {
        background: #7c3aed;
        color: #7c3aed;
    }

    #awards-list {
        height: 1fr;
        border: none;
        background: transparent;
    }

    #awards-list > .option-list--option-highlighted {
        background: #312e81;
        color: #f5f3ff;
        text-style: bold;
    }

    #detail-body {
        height: 1fr;
        padding: 1 0;
    }

    #detail-body Label {
        color: #c4b5fd;
        margin-bottom: 0;
    }

    #detail-value {
        color: #f5f3ff;
        margin-bottom: 1;
        text-style: bold;
    }

    #detail-actions {
        height: auto;
        dock: bottom;
        padding: 1 0;
    }

    #detail-actions Button {
        margin-right: 1;
        min-width: 10;
    }

    #status-line {
        height: 1;
        background: #12121a;
        color: #a78bfa;
        padding: 0 1;
    }

    .muted {
        color: #9ca3af;
    }
    """

    BINDINGS = [
        Binding("ctrl+q", "quit", "Quit", show=True),
        Binding("a", "add", "Add", show=True),
        Binding("e", "edit", "Edit", show=True),
        Binding("d", "delete", "Delete", show=True),
        Binding("f5,ctrl+r", "refresh", "Refresh", show=True),
    ]

    def __init__(self) -> None:
        super().__init__()
        self.data: AwardsData | None = None
        self.synced_at: str | None = None
        self.results_username: str | None = None
        self.results: list[Award] = []
        self.duplicates: list[Award] = []
        self._visible: list[Award] = []
        self._visible_warn: list[bool] = []
        self._busy = False
        self._loading = False
        self._dialog_open = False
        self._active_tab = "all"

    def compose(self) -> ComposeResult:
        yield Header(show_clock=False)
        with Horizontal(id="top-bar"):
            yield Label("USER", id="user-chip")
            yield Input(placeholder="Enter a username…", id="username")
            yield Button("Lookup", id="lookup-btn")
        with Horizontal(id="body"):
            with Vertical(id="actions-panel"):
                yield Label(" Actions ", classes="panel-title")
                yield OptionList(
                    Option("Lookup", id="act-lookup"),
                    Option("Add", id="act-add"),
                    Option("Edit", id="act-edit"),
                    Option("Delete", id="act-delete"),
                    Option("Refresh", id="act-refresh"),
                    Option("Audit", id="act-audit"),
                    id="actions-list",
                )
            with Vertical(id="awards-panel"):
                yield Label(" Awards ", classes="panel-title")
                with TabbedContent(id="award-tabs", initial="all"):
                    yield TabPane("All", id="all")
                    yield TabPane("Badges", id="badges")
                    yield TabPane("Ribbons", id="ribbons")
                    yield TabPane("Foreign", id="foreign")
                    yield TabPane("Duplicates/Typos", id="duplicates")
                yield OptionList(id="awards-list")
            with Vertical(id="detail-panel"):
                yield Label(" Detail ", classes="panel-title")
                with VerticalScroll(id="detail-body"):
                    yield Label("Name")
                    yield Static("—", id="d-name", classes="detail-value")
                    yield Label("Sheet")
                    yield Static("—", id="d-sheet", classes="detail-value")
                    yield Label("Column / Row")
                    yield Static("—", id="d-loc", classes="detail-value")
                    yield Label("Cell")
                    yield Static("—", id="d-cell", classes="detail-value")
                with Horizontal(id="detail-actions"):
                    yield Button("Edit", id="detail-edit", variant="primary")
                    yield Button("Delete", id="detail-delete", variant="error")
        yield Static("Loading awards from Google Sheets…", id="status-line")
        yield Footer()

    def on_mount(self) -> None:
        self.query_one("#username", Input).focus()
        self._loading = True
        self.refresh_data()

    # --- status / busy -------------------------------------------------

    def _set_status(self, text: str) -> None:
        self.query_one("#status-line", Static).update(text)

    def _begin_busy(self, status: str) -> bool:
        """Claim the write lock on the UI thread before starting a worker."""
        if self._busy or self._loading or self._dialog_open:
            self.notify("Wait for the current sheet operation to finish", severity="warning")
            return False
        self._busy = True
        self._set_status(status)
        return True

    def _end_busy(self) -> None:
        self._busy = False

    def _begin_dialog(self) -> bool:
        if self._busy or self._loading or self._dialog_open:
            self.notify("Wait for the current sheet operation to finish", severity="warning")
            return False
        self._dialog_open = True
        return True

    def _end_dialog(self) -> None:
        self._dialog_open = False

    # --- data helpers --------------------------------------------------

    def _patch_sheet_cell(self, sheet: str, col: str, row: int, value: str) -> None:
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

    def _visible_for_tab(self) -> list[tuple[Award, bool]]:
        """Return (award, is_duplicate_or_typo) rows for the active tab."""
        tab = self._active_tab
        if tab == "duplicates":
            return [(a, True) for a in self.duplicates]
        if tab == "all":
            # Primary awards, then duplicate/typo rows (shown in red).
            return [(a, False) for a in self.results] + [
                (a, True) for a in self.duplicates
            ]
        return [(a, False) for a in self.results if a.category == tab]

    def _refresh_awards_list(self, *, select: Award | None = None) -> None:
        rows = self._visible_for_tab()
        self._visible = [a for a, _ in rows]
        self._visible_warn = [warn for _, warn in rows]
        opts = self.query_one("#awards-list", OptionList)
        opts.clear_options()
        highlight = 0
        for i, (award, warn) in enumerate(rows):
            loc = f"  · row {award.row}" if award.row else ""
            label = f"{award.name}{loc}"
            if warn:
                prompt: str | Text = Text(label, style="bold #f87171")
            else:
                prompt = label
            opts.add_option(Option(prompt, id=f"aw-{i}"))
            if (
                select
                and award.sheet == select.sheet
                and award.col == select.col
                and award.row == select.row
            ):
                highlight = i
        if self._visible:
            opts.highlighted = min(highlight, len(self._visible) - 1)
            self._show_detail(self._visible[opts.highlighted or 0])
        else:
            self._show_detail(None)

    def _show_detail(self, award: Award | None) -> None:
        if not award:
            self.query_one("#d-name", Static).update("—")
            self.query_one("#d-sheet", Static).update("—")
            self.query_one("#d-loc", Static).update("—")
            self.query_one("#d-cell", Static).update("—")
            return
        self.query_one("#d-name", Static).update(award.name)
        self.query_one("#d-sheet", Static).update(award.sheet or "—")
        loc = f"{award.col}{award.row}" if award.col and award.row else "—"
        self.query_one("#d-loc", Static).update(loc)
        self.query_one("#d-cell", Static).update(award.cell or "—")

    def _selected_award(self) -> Award | None:
        opts = self.query_one("#awards-list", OptionList)
        idx = opts.highlighted
        if idx is None or not self._visible:
            return None
        if 0 <= idx < len(self._visible):
            return self._visible[idx]
        return None

    def _apply_user_view(
        self,
        username: str,
        *,
        select: Award | None = None,
        status: str | None = None,
    ) -> None:
        if not self.data:
            return
        awards = flatten_awards_sorted(get_awards_for_username(self.data.index, username))
        dup_hits = find_duplicates_for_user(self.data, username)
        awards = awards_excluding_duplicate_rows(awards, dup_hits)
        self.results_username = username
        self.results = awards
        self.duplicates = [h.to_award() for h in dup_hits]
        self._refresh_awards_list(select=select)
        if self.results or self.duplicates:
            self.query_one("#awards-list", OptionList).focus()
        dup_note = f" · {len(self.duplicates)} duplicate(s)" if self.duplicates else ""
        self._set_status(
            status
            or f"{username} · {len(awards)} award(s){dup_note} · a/e/d · F5 refresh"
        )
        self._resolve_visible_rows(
            username,
            list(self.results),
            list(self.duplicates),
            select,
            status
            or f"{username} · {len(awards)} award(s){dup_note} · a/e/d · F5 refresh",
        )

    @work(thread=True, exclusive=True, group="rows")
    def _resolve_visible_rows(
        self,
        username: str,
        results: list[Award],
        duplicates: list[Award],
        select: Award | None,
        status: str,
    ) -> None:
        """CSV row numbers can lag mid-sheet; snap displayed rows to live cells."""
        try:
            service = build_sheets_service(interactive=False)
            api = service.spreadsheets().values()
        except Exception:
            return
        fixed_results = [award_with_live_row(a, api) for a in results]
        fixed_dups = [award_with_live_row(a, api) for a in duplicates]
        if fixed_results == results and fixed_dups == duplicates:
            return

        def apply() -> None:
            if self.results_username != username:
                return
            self.results = fixed_results
            self.duplicates = fixed_dups
            if select:
                select_fixed = next(
                    (
                        a
                        for a in fixed_results + fixed_dups
                        if a.sheet == select.sheet
                        and a.col == select.col
                        and a.cell == select.cell
                    ),
                    select,
                )
                self._refresh_awards_list(select=select_fixed)
            else:
                self._refresh_awards_list()
            self._set_status(status)

        self.call_from_thread(apply)

    # --- lookup / refresh / audit --------------------------------------

    @on(Button.Pressed, "#lookup-btn")
    def on_lookup_button(self) -> None:
        self.action_lookup()

    @on(Input.Submitted, "#username")
    def on_username_submitted(self) -> None:
        self.action_lookup()

    def action_lookup(self) -> None:
        raw = self.query_one("#username", Input).value
        username = normalize_username(raw) or raw.strip().lstrip("@")
        if not username:
            self.notify("Enter a username", severity="warning")
            return
        if self.data is None:
            self.notify("Still loading awards…", severity="warning")
            return
        self._apply_user_view(username)

    @work(thread=True, exclusive=True, group="sync")
    def refresh_data(self) -> None:
        self.call_from_thread(self._set_status, "Syncing Badges / Ribbons / Foreign Awards…")
        try:
            data = build_awards_data()
            synced = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M:%S UTC")
            auth = auth_status()
            auth_note = {
                "service_account": "write: service account",
                "oauth_token": "write: logged in",
                "oauth_needs_login": "write: run --login",
                "missing": "write: no credentials",
            }.get(auth, auth)
            status = f"Ready · {len(data.index)} users · {auth_note}"

            def apply() -> None:
                self.data = data
                self.synced_at = synced
                self.sub_title = f"Synced {synced}"
                self._set_status(status)
                if self.results_username:
                    self._apply_user_view(self.results_username, status=status)

            self.call_from_thread(apply)
        except Exception as exc:  # noqa: BLE001
            self.call_from_thread(self._set_status, "Sync failed")
            self.call_from_thread(self.notify, str(exc), severity="error")
        finally:
            def done() -> None:
                self._loading = False

            self.call_from_thread(done)

    def action_refresh(self) -> None:
        if self._busy or self._loading or self._dialog_open:
            self.notify("Wait for the current sheet operation to finish", severity="warning")
            return
        self._loading = True
        self.refresh_data()

    def action_audit(self) -> None:
        if not self._begin_busy("Running duplicate audit…"):
            return
        self.run_audit()

    @work(thread=True, exclusive=True, group="audit")
    def run_audit(self) -> None:
        try:
            data = self.data
            if data is None:
                data = build_awards_data()

                def store() -> None:
                    self.data = data

                self.call_from_thread(store)
            report = collect_sheet_audit(data)
            generated = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M:%S UTC")
            body = format_audit_report(report, generated)
            stamp = datetime.now(timezone.utc).strftime("%Y-%m-%d_%H%M%S")
            dest = ROOT / "audits" / f"audit-{stamp}.txt"
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_text(body, encoding="utf-8")
            groups = report["duplicate_groups"]
            identical = sum(1 for g in groups if g["kind"] == "identical")
            conflict = sum(1 for g in groups if g["kind"] == "conflict")
            msg = (
                f"Wrote {dest} · {identical} identical · {conflict} conflict · "
                f"{len(report['similar_pairs'])} similar"
            )

            def apply() -> None:
                self._set_status(msg)
                self.notify(f"Audit saved to {dest.name}")

            self.call_from_thread(apply)
        except Exception as exc:  # noqa: BLE001

            def fail() -> None:
                self.notify(str(exc), severity="error")
                self._set_status("Audit failed")

            self.call_from_thread(fail)
        finally:
            self.call_from_thread(self._end_busy)

    # --- actions pane --------------------------------------------------

    @on(OptionList.OptionSelected, "#actions-list")
    def on_action_selected(self, event: OptionList.OptionSelected) -> None:
        aid = event.option_id
        if aid == "act-lookup":
            self.action_lookup()
        elif aid == "act-add":
            self.action_add()
        elif aid == "act-edit":
            self.action_edit()
        elif aid == "act-delete":
            self.action_delete()
        elif aid == "act-refresh":
            self.action_refresh()
        elif aid == "act-audit":
            self.action_audit()

    @on(TabbedContent.TabActivated, "#award-tabs")
    def on_tab_activated(self, event: TabbedContent.TabActivated) -> None:
        pane_id = event.pane.id if event.pane else "all"
        self._active_tab = pane_id or "all"
        self._refresh_awards_list()

    @on(OptionList.OptionHighlighted, "#awards-list")
    def on_award_highlighted(self, event: OptionList.OptionHighlighted) -> None:
        idx = event.option_index
        if 0 <= idx < len(self._visible):
            self._show_detail(self._visible[idx])

    @on(OptionList.OptionSelected, "#awards-list")
    def on_award_selected(self, event: OptionList.OptionSelected) -> None:
        if event.option_id and event.option_id.startswith("aw-"):
            idx = int(event.option_id.split("-", 1)[1])
            if 0 <= idx < len(self._visible):
                self._show_detail(self._visible[idx])

    # --- add / edit / delete -------------------------------------------

    @on(Button.Pressed, "#detail-edit")
    def on_detail_edit(self) -> None:
        self.action_edit()

    @on(Button.Pressed, "#detail-delete")
    def on_detail_delete(self) -> None:
        self.action_delete()

    @work
    async def action_add(self) -> None:
        if not self._begin_dialog():
            return
        result: tuple[AwardDef, str] | None = None
        try:
            if not self.results_username:
                self.notify("Look up a user before adding awards", severity="warning")
                return
            if not self.data:
                self.notify("Still loading…", severity="warning")
                return
            owned = owned_award_columns(self.results + self.duplicates, self.results_username)
            candidates = [d for d in self.data.catalog if (d.sheet, d.col) not in owned]
            if not candidates:
                self.notify("No remaining awards to add for this user", severity="information")
                return
            result = await self.push_screen_wait(AddAwardScreen(candidates))
        finally:
            self._end_dialog()
        if not result:
            return
        award_def, suffix = result
        if not self._begin_busy(f"Writing {award_def.base_name}…"):
            return
        self._commit_add(award_def, suffix)

    @work(thread=True, exclusive=True, group="write")
    def _commit_add(self, award_def: AwardDef, suffix: str) -> None:
        username = self.results_username or ""
        rows = self.data.sheet_rows.get(award_def.sheet) if self.data else None
        try:
            result = add_award_to_user(
                username=username,
                award_def=award_def,
                suffix=suffix,
                rows=rows,
                interactive_auth=False,
            )

            def apply() -> None:
                if result.ok and result.award and self.data:
                    upsert_award_in_index(self.data.index, result.award)
                    self._patch_sheet_cell(
                        result.award.sheet,
                        result.award.col,
                        result.award.row,
                        result.award.cell,
                    )
                    self._apply_user_view(
                        username, select=result.award, status=result.message
                    )
                    self.notify(result.message)
                else:
                    self.notify(result.message, severity="error")
                    self._set_status("Add failed")

            self.call_from_thread(apply)
        except Exception as exc:  # noqa: BLE001
            self.call_from_thread(self.notify, str(exc), severity="error")
            self.call_from_thread(self._set_status, "Add failed")
        finally:
            self.call_from_thread(self._end_busy)

    @work
    async def action_edit(self) -> None:
        if not self._begin_dialog():
            return
        award: Award | None = None
        new_cell: str | None = None
        try:
            award = self._selected_award()
            if not award:
                self.notify("Select an award to edit", severity="warning")
                return
            new_cell = await self.push_screen_wait(EditAwardScreen(award))
        finally:
            self._end_dialog()
        if award is None or new_cell is None:
            return
        if not self._begin_busy("Updating sheet…"):
            return
        self._commit_edit(award, new_cell)

    @work(thread=True, exclusive=True, group="write")
    def _commit_edit(self, award: Award, new_cell: str) -> None:
        try:
            result = update_award_cell(award, new_cell, interactive_auth=False)

            def apply() -> None:
                if result.ok and result.award and self.data:
                    upsert_award_in_index(self.data.index, result.award)
                    self._patch_sheet_cell(
                        result.award.sheet,
                        result.award.col,
                        result.award.row,
                        result.award.cell,
                    )
                    new_key = normalize_username(result.award.cell)
                    viewed = (self.results_username or "").lower()
                    if new_key and viewed and new_key != viewed:
                        self._apply_user_view(
                            viewed,
                            status=f"{result.message} · no longer under @{viewed}",
                        )
                    elif self.results_username:
                        self._apply_user_view(
                            self.results_username,
                            select=result.award,
                            status=result.message,
                        )
                    self.notify(result.message)
                else:
                    self.notify(result.message, severity="error")
                    self._set_status("Edit failed")

            self.call_from_thread(apply)
        except Exception as exc:  # noqa: BLE001
            self.call_from_thread(self.notify, str(exc), severity="error")
            self.call_from_thread(self._set_status, "Edit failed")
        finally:
            self.call_from_thread(self._end_busy)

    @work
    async def action_delete(self) -> None:
        if not self._begin_dialog():
            return
        award: Award | None = None
        confirmed = False
        try:
            award = self._selected_award()
            if not award:
                self.notify("Select an award to delete", severity="warning")
                return
            viewed = self.results_username or "?"
            confirmed = bool(await self.push_screen_wait(DeleteAwardScreen(award, viewed)))
        finally:
            self._end_dialog()
        if not award or not confirmed:
            return
        if not self._begin_busy(f"Removing {award.name}…"):
            return
        self._commit_delete(award)

    @work(thread=True, exclusive=True, group="write")
    def _commit_delete(self, award: Award) -> None:
        try:
            result = remove_award(award, interactive_auth=False)

            def apply() -> None:
                if result.ok and self.data:
                    reindex_column_after_delete(
                        self.data.index, award.sheet, award.col, award.row
                    )
                    rows = self.data.sheet_rows.get(award.sheet)
                    if rows is not None:
                        shift_column_up_in_rows(
                            rows, award.sheet, award.col, award.row
                        )
                    if self.results_username:
                        self._apply_user_view(
                            self.results_username, status=result.message
                        )
                    self.notify(result.message)
                else:
                    self.notify(result.message, severity="error")
                    self._set_status("Delete failed")

            self.call_from_thread(apply)
        except Exception as exc:  # noqa: BLE001
            self.call_from_thread(self.notify, str(exc), severity="error")
            self.call_from_thread(self._set_status, "Delete failed")
        finally:
            self.call_from_thread(self._end_busy)


def main() -> None:
    _reexec_venv_if_needed()
    AwardsApp().run()


if __name__ == "__main__":
    main()
