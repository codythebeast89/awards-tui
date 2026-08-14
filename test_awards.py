#!/usr/bin/env python3
"""Offline unit checks for award parsing and cell builders (no network)."""

from __future__ import annotations

from awards import (
    Award,
    add_award,
    format_award_name,
    index_to_col,
    normalize_username,
    group_awards,
    flatten_awards_sorted,
)
from sheets_edit import build_cell_value, find_first_empty_row


def test_normalize() -> None:
    assert normalize_username("@FooBar_1 - x2") == "foobar_1"
    assert normalize_username("Alice") == "alice"
    assert normalize_username("") is None
    assert normalize_username("\u200b\u200bamongus400and20") == "amongus400and20"
    assert normalize_username("Alice  ") == "alice"


def test_usernames_similar() -> None:
    from awards import usernames_similar

    assert usernames_similar("codythebeast89", "codythebast89")
    assert not usernames_similar("codythebeast89", "totallydifferent")
    # Mid-string typo on a long name (prefix rule used to miss these)
    assert usernames_similar("rangers_apprentice122", "rangers_aprentice122")
    assert usernames_similar("thundebolt_rblx", "thunderbolt_rblx")


def test_cell_format_issues() -> None:
    from awards import cell_format_issues

    assert "missing_space_before_dash" in cell_format_issues("codythebeast89- Master")
    assert "extra_spaces" in cell_format_issues("user  - x2")
    assert cell_format_issues("Alice  ") == []
    assert cell_format_issues("Alice") == []


def test_find_duplicates_for_user() -> None:
    from awards import AwardsData, find_duplicates_for_user

    rows = [
        ["", "", "hdr"],
        ["", "", "x"],
        ["", "", "Army Parachutist Badge"],
        ["", "", "codythebeast89"],
        ["", "", "codythebeast89"],
        ["", "", "codythebast89- Master"],
        ["", "", "codythebeast89 - Master"],
    ]
    data = AwardsData(
        index={},
        catalog=[],
        sheet_rows={"Badges Database": rows},
    )
    hits = find_duplicates_for_user(data, "codythebeast89")
    reasons = {h.reason for h in hits}
    assert "duplicate_conflict" in reasons
    assert "similar_username" in reasons
    assert "malformed_cell" in reasons


def test_duplicate_identical_vs_conflict() -> None:
    from awards import AwardsData, collect_sheet_audit, format_audit_report

    rows = [
        ["", "", "hdr"],
        ["", "", "x"],
        ["", "", "Army Parachutist Badge"],
        ["", "", "alice"],
        ["", "", "alice"],
        ["", "", "bob - Basic"],
        ["", "", "bob - Master"],
    ]
    data = AwardsData(index={}, catalog=[], sheet_rows={"Badges Database": rows})
    report = collect_sheet_audit(data)
    kinds = {(g["user"], g["kind"]) for g in report["duplicate_groups"]}
    assert ("alice", "identical") in kinds
    assert ("bob", "conflict") in kinds
    text = format_audit_report(report, "2026-08-14 00:00:00 UTC")
    assert "Decorations Database — duplicate audit" in text
    assert "@alice" in text
    assert "@bob" in text
    assert "Row " in text
    assert "End of report." in text


def test_format_ribbon() -> None:
    name = format_award_name("ribbons", "Army Good Conduct Medal", 'user - "Bronze Oak Leaf" x2')
    assert "Army Good Conduct Medal" in name
    assert "2nd Award" in name


def test_format_badge() -> None:
    # MC is relative to the column badge, not hardcoded to CAB
    cib = format_award_name("badges", "Combat Infantryman Badge", "cancholic - MC x2")
    assert cib == "Master Combat Infantryman Badge (2nd Award)", cib

    cab = format_award_name("badges", "Combat Action Badge", "user - MC")
    assert cab == "Master Combat Action Badge", cab

    cmb = format_award_name("badges", "Combat Medical Badge", "user x2 - MC")
    assert cmb == "Master Combat Medical Badge (2nd Award)", cmb

    esb = format_award_name("badges", "Combat Action Badge", "user - ESB")
    assert esb == "Expert Soldier Badge", esb

    # Non-abbrev details still work
    senior = format_award_name("badges", "Army Parachutist Badge", "user - Senior")
    assert senior == "Army Parachutist Badge (Senior)", senior

    one_cjs = format_award_name("badges", "Army Parachutist Badge", "user - Master (1x CJS)")
    assert one_cjs == "Army Parachutist Badge (Master, Combat Jump Star)", one_cjs

    x1_cjs = format_award_name("badges", "Military Freefall Badge", "user - Basic (x1 CJS)")
    assert x1_cjs == "Military Freefall Badge (Basic, Combat Jump Star)", x1_cjs

    three_cjs = format_award_name("badges", "Army Parachutist Badge", "user - Senior (3x CJS)")
    assert three_cjs == "Army Parachutist Badge (Senior, 3 Combat Jump Stars)", three_cjs

    messy_cjs = format_award_name("badges", "Army Parachutist Badge", "weeelfdude - Master  (x5 CJS)")
    assert messy_cjs == "Army Parachutist Badge (Master, 5 Combat Jump Stars)", messy_cjs


def test_group() -> None:
    awards = [
        Award("badges", "Expert Infantryman Badge"),
        Award("ribbons", "Army Service Ribbon"),
        Award("foreign", "German Armed Forces Badge"),
    ]
    grouped = group_awards(awards)
    assert grouped["Badges"] == ["Expert Infantryman Badge"]
    assert grouped["Ribbons"] == ["Army Service Ribbon"]
    assert grouped["Foreign Awards"] == ["German Armed Forces Badge"]


def test_dedupe() -> None:
    index: dict[str, list[Award]] = {}
    a = Award("ribbons", "ASR", sheet="Ribbons Database", col="C", row=10)
    add_award(index, "bob", a)
    add_award(index, "bob", a)
    assert len(index["bob"]) == 1


def test_dedupe_same_name_different_column() -> None:
    index: dict[str, list[Award]] = {}
    a1 = Award("ribbons", "ASR", sheet="Ribbons Database", col="C", row=10)
    a2 = Award("ribbons", "ASR", sheet="Ribbons Database", col="D", row=11)
    add_award(index, "bob", a1)
    add_award(index, "bob", a2)
    assert len(index["bob"]) == 2


def test_index_to_col() -> None:
    assert index_to_col(0) == "A"
    assert index_to_col(2) == "C"
    assert index_to_col(27) == "AB"


def test_build_cell_value() -> None:
    assert build_cell_value("Alice") == "Alice"
    assert build_cell_value("@Bob", "x2") == "Bob x2"
    assert build_cell_value("Carol", "Master") == "Carol - Master"


def test_find_first_empty() -> None:
    rows = [
        ["", "", "Army Distinguished Service Cross"],
        ["", "", "user1"],
        ["", "", ""],
        ["", "", "user2"],
    ]
    assert find_first_empty_row(rows, "Ribbons Database", "C") == 3


def test_badges_row_offset() -> None:
    from awards import csv_index_to_sheet_row

    assert csv_index_to_sheet_row("Badges Database", 49) == 56
    assert csv_index_to_sheet_row("Ribbons Database", 49) == 50


def test_flatten_order() -> None:
    awards = [
        Award("foreign", "Zulu"),
        Award("badges", "Beta"),
        Award("ribbons", "Alpha"),
        Award("badges", "Alpha"),
    ]
    flat = flatten_awards_sorted(awards)
    assert [a.category for a in flat] == ["badges", "badges", "ribbons", "foreign"]


if __name__ == "__main__":
    test_normalize()
    test_format_ribbon()
    test_format_badge()
    test_group()
    test_dedupe()
    test_dedupe_same_name_different_column()
    test_index_to_col()
    test_build_cell_value()
    test_find_first_empty()
    test_badges_row_offset()
    test_usernames_similar()
    test_cell_format_issues()
    test_find_duplicates_for_user()
    test_duplicate_identical_vs_conflict()
    test_flatten_order()
    print("All offline tests passed.")
