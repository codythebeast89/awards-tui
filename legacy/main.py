#!/usr/bin/env python3
"""CLI entry: interactive TUI (default), lookup, login, or mutate awards."""

from __future__ import annotations

import argparse
import os
import sys
from datetime import datetime, timezone
from pathlib import Path


def _reexec_venv_if_needed() -> None:
    """Use project .venv when system Python is missing Google API packages."""
    try:
        import googleapiclient  # noqa: F401
        return
    except ImportError:
        pass
    venv_python = Path(__file__).resolve().parent.parent / ".venv" / "bin" / "python"
    if not venv_python.is_file():
        return
    if Path(sys.executable).resolve() == venv_python.resolve():
        return
    os.execv(str(venv_python), [str(venv_python), *sys.argv])


_reexec_venv_if_needed()

from awards import (
    CATEGORY_LABELS,
    ROOT,
    build_awards_data,
    collect_sheet_audit,
    format_audit_report,
    get_awards_for_username,
    group_awards,
)


def print_awards(username: str) -> int:
    print("Syncing awards from Google Sheets…", file=sys.stderr)
    data = build_awards_data()
    awards = get_awards_for_username(data.index, username)
    if not awards:
        print(f"No awards found for {username}")
        return 1
    print(f"Awards for @{username} ({len(awards)} total)\n")
    grouped = group_awards(awards)
    for label in CATEGORY_LABELS.values():
        names = grouped.get(label) or []
        print(f"{label} ({len(names)})")
        if not names:
            print("  (none)")
        else:
            for name in names:
                print(f"  • {name}")
        print()
    return 0


def cmd_login() -> int:
    from sheets_auth import login

    print("Starting Google OAuth login (browser will open)…")
    try:
        hint = login()
    except Exception as exc:  # noqa: BLE001
        print(f"Login failed: {exc}", file=sys.stderr)
        return 1
    print(f"Logged in ({hint}). token.json saved — you can use add/edit/delete in the TUI.")
    return 0


def cmd_auth_status() -> int:
    from sheets_auth import auth_status, credentials_path, service_account_path, TOKEN_PATH

    status = auth_status()
    print(f"status: {status}")
    print(f"oauth client: {credentials_path() or '(none)'}")
    print(f"service account: {service_account_path() or '(none)'}")
    print(f"token: {TOKEN_PATH if TOKEN_PATH.is_file() else '(none)'}")
    return 0


def cmd_audit(out_path: str | None = None) -> int:
    """Read-only duplicate / typo report. Writes a .txt file; does not write to the sheet."""
    print("Syncing awards from Google Sheets (read-only)…", file=sys.stderr)
    data = build_awards_data()
    report = collect_sheet_audit(data)
    generated = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M:%S UTC")
    body = format_audit_report(report, generated)

    if out_path:
        dest = Path(out_path).expanduser()
    else:
        stamp = datetime.now(timezone.utc).strftime("%Y-%m-%d_%H%M%S")
        dest = ROOT / "audits" / f"audit-{stamp}.txt"
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_text(body, encoding="utf-8")

    groups = report["duplicate_groups"]
    identical = sum(1 for g in groups if g["kind"] == "identical")
    conflict = sum(1 for g in groups if g["kind"] == "conflict")
    print(f"Wrote {dest}")
    print(
        f"{report['columns']} columns · {report['cells']} cells · "
        f"{identical} identical copies · {conflict} conflicting · "
        f"{len(report['similar_pairs'])} similar · {len(report['malformed'])} malformed"
    )
    return 0


def cmd_add(username: str, award_query: str, suffix: str) -> int:
    from sheets_edit import add_award_to_user

    print("Syncing catalog…", file=sys.stderr)
    data = build_awards_data()
    q = award_query.casefold()
    matches = [d for d in data.catalog if q in d.base_name.casefold()]
    if not matches:
        print(f"No award matched {award_query!r}", file=sys.stderr)
        return 1
    if len(matches) > 1:
        print(f"Ambiguous ({len(matches)} matches). Be more specific:", file=sys.stderr)
        for d in matches[:20]:
            print(f"  [{CATEGORY_LABELS.get(d.category)}] {d.base_name}", file=sys.stderr)
        return 1
    award_def = matches[0]
    result = add_award_to_user(
        username=username,
        award_def=award_def,
        suffix=suffix,
        rows=data.sheet_rows.get(award_def.sheet),
        interactive_auth=False,
    )
    print(result.message)
    return 0 if result.ok else 1


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Look up and edit decorations in the FORSCOM Decorations Database.",
    )
    parser.add_argument(
        "username",
        nargs="?",
        help="If provided alone, print awards once and exit (no TUI).",
    )
    parser.add_argument("--cli", action="store_true", help="Force non-interactive lookup.")
    parser.add_argument("--login", action="store_true", help="Authorize Google Sheets write access via OAuth.")
    parser.add_argument("--auth-status", action="store_true", help="Show credential / login status.")
    parser.add_argument(
        "--audit",
        action="store_true",
        help="Read-only scan for duplicates; writes a timestamped .txt report under audits/.",
    )
    parser.add_argument(
        "--audit-out",
        metavar="FILE",
        help="Write the --audit report to FILE instead of audits/audit-TIMESTAMP.txt.",
    )
    parser.add_argument("--add", metavar="AWARD", help="Add award by name (with username).")
    parser.add_argument("--suffix", default="", help="Optional cell suffix when using --add (e.g. x2).")
    args = parser.parse_args()

    if args.login:
        return cmd_login()
    if args.auth_status:
        return cmd_auth_status()
    if args.audit or args.audit_out:
        return cmd_audit(args.audit_out)
    if args.add:
        if not args.username:
            parser.error("username is required with --add")
        return cmd_add(args.username, args.add, args.suffix)

    if args.username or args.cli:
        if not args.username:
            parser.error("username is required with --cli")
        return print_awards(args.username)

    from tui import main as tui_main

    tui_main()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
