use awards_core::*;
use std::collections::HashMap;

#[test]
fn test_normalize() {
    assert_eq!(
        normalize_username(Some("@FooBar_1 - x2")).as_deref(),
        Some("foobar_1")
    );
    assert_eq!(normalize_username(Some("Alice")).as_deref(), Some("alice"));
    assert_eq!(normalize_username(Some("")), None);
    assert_eq!(
        normalize_username(Some("\u{200b}\u{200b}amongus400and20")).as_deref(),
        Some("amongus400and20")
    );
    assert_eq!(
        normalize_username(Some("Alice  ")).as_deref(),
        Some("alice")
    );
}

#[test]
fn test_user_agent_includes_package_version() {
    assert!(
        USER_AGENT.contains(env!("CARGO_PKG_VERSION")),
        "USER_AGENT={USER_AGENT}"
    );
    assert!(USER_AGENT.starts_with("awards-tui/"));
}

#[test]
fn test_usernames_similar() {
    assert!(usernames_similar("codythebeast89", "codythebast89"));
    assert!(!usernames_similar("codythebeast89", "totallydifferent"));
    assert!(usernames_similar(
        "rangers_apprentice122",
        "rangers_aprentice122"
    ));
    assert!(usernames_similar("thundebolt_rblx", "thunderbolt_rblx"));
}

#[test]
fn test_cell_format_issues() {
    assert!(cell_format_issues("codythebeast89- Master")
        .iter()
        .any(|i| i == "missing_space_before_dash"));
    assert!(cell_format_issues("user  - x2")
        .iter()
        .any(|i| i == "extra_spaces"));
    assert!(cell_format_issues("Alice  ").is_empty());
    assert!(cell_format_issues("Alice").is_empty());
}

#[test]
fn test_find_duplicates_for_user() {
    let rows = vec![
        vec!["".into(), "".into(), "hdr".into()],
        vec!["".into(), "".into(), "x".into()],
        vec!["".into(), "".into(), "Army Parachutist Badge".into()],
        vec!["".into(), "".into(), "codythebeast89".into()],
        vec!["".into(), "".into(), "codythebeast89".into()],
        vec!["".into(), "".into(), "codythebast89- Master".into()],
        vec!["".into(), "".into(), "codythebeast89 - Master".into()],
    ];
    let mut sheet_rows = HashMap::new();
    sheet_rows.insert("Badges Database".into(), rows);
    let data = AwardsData {
        index: HashMap::new(),
        catalog: vec![],
        sheet_rows,
    };
    let hits = find_duplicates_for_user(&data, "codythebeast89");
    let reasons: std::collections::HashSet<_> = hits.iter().map(|h| h.reason.as_str()).collect();
    assert!(reasons.contains("duplicate_conflict"));
    assert!(reasons.contains("similar_username"));
    assert!(reasons.contains("malformed_cell"));
}

#[test]
fn test_duplicate_identical_vs_conflict() {
    let rows = vec![
        vec!["".into(), "".into(), "hdr".into()],
        vec!["".into(), "".into(), "x".into()],
        vec!["".into(), "".into(), "Army Parachutist Badge".into()],
        vec!["".into(), "".into(), "alice".into()],
        vec!["".into(), "".into(), "alice".into()],
        vec!["".into(), "".into(), "bob - Basic".into()],
        vec!["".into(), "".into(), "bob - Master".into()],
    ];
    let mut sheet_rows = HashMap::new();
    sheet_rows.insert("Badges Database".into(), rows);
    let data = AwardsData {
        index: HashMap::new(),
        catalog: vec![],
        sheet_rows,
    };
    let report = collect_sheet_audit(&data);
    let kinds: std::collections::HashSet<_> = report
        .duplicate_groups
        .iter()
        .map(|g| (g.user.as_str(), g.kind.as_str()))
        .collect();
    assert!(kinds.contains(&("alice", "identical")));
    assert!(kinds.contains(&("bob", "conflict")));
    let text = format_audit_report(&report, "2026-08-14 00:00:00 UTC");
    assert!(text.contains("Decorations Database — duplicate audit"));
    assert!(text.contains("@alice"));
    assert!(text.contains("@bob"));
    assert!(text.contains("Row "));
    assert!(text.contains("End of report."));
}

#[test]
fn test_format_ribbon() {
    let name = format_award_name(
        "ribbons",
        Some("Army Good Conduct Medal"),
        "user - \"Bronze Oak Leaf\" x2",
    )
    .unwrap();
    assert!(name.contains("Army Good Conduct Medal"));
    assert!(name.contains("2nd Award"));
}

#[test]
fn test_format_badge() {
    let cib = format_award_name(
        "badges",
        Some("Combat Infantryman Badge"),
        "cancholic - MC x2",
    )
    .unwrap();
    assert_eq!(cib, "Master Combat Infantryman Badge (2nd Award)");

    let cab = format_award_name("badges", Some("Combat Action Badge"), "user - MC").unwrap();
    assert_eq!(cab, "Master Combat Action Badge");

    let cmb = format_award_name("badges", Some("Combat Medical Badge"), "user x2 - MC").unwrap();
    assert_eq!(cmb, "Master Combat Medical Badge (2nd Award)");

    let esb = format_award_name("badges", Some("Combat Action Badge"), "user - ESB").unwrap();
    assert_eq!(esb, "Expert Soldier Badge");

    let senior =
        format_award_name("badges", Some("Army Parachutist Badge"), "user - Senior").unwrap();
    assert_eq!(senior, "Army Parachutist Badge (Senior)");

    let one_cjs = format_award_name(
        "badges",
        Some("Army Parachutist Badge"),
        "user - Master (1x CJS)",
    )
    .unwrap();
    assert_eq!(one_cjs, "Army Parachutist Badge (Master, Combat Jump Star)");

    let x1_cjs = format_award_name(
        "badges",
        Some("Military Freefall Badge"),
        "user - Basic (x1 CJS)",
    )
    .unwrap();
    assert_eq!(x1_cjs, "Military Freefall Badge (Basic, Combat Jump Star)");

    let three_cjs = format_award_name(
        "badges",
        Some("Army Parachutist Badge"),
        "user - Senior (3x CJS)",
    )
    .unwrap();
    assert_eq!(
        three_cjs,
        "Army Parachutist Badge (Senior, 3 Combat Jump Stars)"
    );

    let messy_cjs = format_award_name(
        "badges",
        Some("Army Parachutist Badge"),
        "weeelfdude - Master  (x5 CJS)",
    )
    .unwrap();
    assert_eq!(
        messy_cjs,
        "Army Parachutist Badge (Master, 5 Combat Jump Stars)"
    );
}

#[test]
fn test_group() {
    let awards = vec![
        Award::new("badges", "Expert Infantryman Badge"),
        Award::new("ribbons", "Army Service Ribbon"),
        Award::new("foreign", "German Armed Forces Badge"),
    ];
    let grouped = group_awards(&awards);
    assert_eq!(
        grouped.get("Badges").map(|v| v.as_slice()),
        Some(&["Expert Infantryman Badge".to_string()][..])
    );
    assert_eq!(
        grouped.get("Ribbons").map(|v| v.as_slice()),
        Some(&["Army Service Ribbon".to_string()][..])
    );
    assert_eq!(
        grouped.get("Foreign Awards").map(|v| v.as_slice()),
        Some(&["German Armed Forces Badge".to_string()][..])
    );
}

#[test]
fn test_dedupe() {
    let mut index = HashMap::new();
    let a = Award::new("ribbons", "ASR").with_location("Ribbons Database", "C", 10);
    add_award(&mut index, Some("bob"), Some(&a));
    add_award(&mut index, Some("bob"), Some(&a));
    assert_eq!(index["bob"].len(), 1);
}

#[test]
fn test_dedupe_same_name_different_column() {
    let mut index = HashMap::new();
    let a1 = Award::new("ribbons", "ASR").with_location("Ribbons Database", "C", 10);
    let a2 = Award::new("ribbons", "ASR").with_location("Ribbons Database", "D", 11);
    add_award(&mut index, Some("bob"), Some(&a1));
    add_award(&mut index, Some("bob"), Some(&a2));
    assert_eq!(index["bob"].len(), 2);
}

#[test]
fn test_index_to_col() {
    assert_eq!(index_to_col(0), "A");
    assert_eq!(index_to_col(2), "C");
    assert_eq!(index_to_col(27), "AB");
}

#[test]
fn test_build_cell_value() {
    assert_eq!(build_cell_value("Alice", ""), "Alice");
    assert_eq!(build_cell_value("@Bob", "x2"), "Bob x2");
    assert_eq!(build_cell_value("Carol", "Master"), "Carol - Master");
}

#[test]
fn test_replace_username_in_cell() {
    assert_eq!(
        replace_username_in_cell("Alice", "Bob").as_deref(),
        Some("Bob")
    );
    assert_eq!(
        replace_username_in_cell("@alice x2", "NewName").as_deref(),
        Some("NewName x2")
    );
    assert_eq!(
        replace_username_in_cell("alice - 75th CSIB", "Nova").as_deref(),
        Some("Nova - 75th CSIB")
    );
    assert_eq!(
        replace_username_in_cell("alice - Master  (x5 CJS)", "bob").as_deref(),
        Some("bob - Master  (x5 CJS)")
    );
    assert_eq!(replace_username_in_cell("", "Bob"), None);
    assert_eq!(
        replace_username_in_cell("Alice x2", "Bob - Master"),
        None,
        "refuse cell-like new usernames"
    );
    assert_eq!(replace_username_in_cell("Alice", "Bob x2"), None);
}

#[test]
fn test_parse_bare_username() {
    assert_eq!(parse_bare_username("Bob").as_deref(), Some("Bob"));
    assert_eq!(parse_bare_username("@Nova_1").as_deref(), Some("Nova_1"));
    assert_eq!(parse_bare_username("Bob - Master"), None);
    assert_eq!(parse_bare_username("Alice x2"), None);
    assert_eq!(parse_bare_username(""), None);
}

#[test]
fn test_find_first_empty() {
    let rows = vec![
        vec![
            "".into(),
            "".into(),
            "Army Distinguished Service Cross".into(),
        ],
        vec!["".into(), "".into(), "user1".into()],
        vec!["".into(), "".into(), "".into()],
        vec!["".into(), "".into(), "user2".into()],
    ];
    // csv row 3 + ribbons offset 8 => sheet row 11
    assert_eq!(find_first_empty_row(&rows, "Ribbons Database", "C"), 11);
}

#[test]
fn test_badges_row_offset() {
    assert_eq!(csv_index_to_sheet_row("Badges Database", 49), 56);
    assert_eq!(csv_index_to_sheet_row("Ribbons Database", 49), 58);
    assert_eq!(csv_index_to_sheet_row("Ribbons Database", 305), 314);
    assert_eq!(sheet_data_start_row("Ribbons Database"), 10);
    assert_eq!(csv_index_to_sheet_row("Foreign Awards Database", 49), 57);
    assert_eq!(sheet_data_start_row("Foreign Awards Database"), 10);
}

#[test]
fn test_flatten_order() {
    let awards = vec![
        Award::new("foreign", "Zulu"),
        Award::new("badges", "Beta"),
        Award::new("ribbons", "Alpha"),
        Award::new("badges", "Alpha"),
    ];
    let flat = flatten_awards_sorted(&awards);
    let cats: Vec<_> = flat.iter().map(|a| a.category.as_str()).collect();
    assert_eq!(cats, vec!["badges", "badges", "ribbons", "foreign"]);
}

#[test]
fn test_awards_excluding_duplicate_rows() {
    let rows = vec![
        vec!["".into(), "".into(), "hdr".into()],
        vec!["".into(), "".into(), "x".into()],
        vec!["".into(), "".into(), "Army Parachutist Badge".into()],
        vec!["".into(), "".into(), "alice".into()],
        vec!["".into(), "".into(), "alice".into()],
        vec!["".into(), "".into(), "bob".into()],
    ];
    let a1 = Award::new("badges", "Army Parachutist Badge")
        .with_location("Badges Database", "C", 10)
        .with_cell("alice", "Army Parachutist Badge");
    let a2 = Award::new("badges", "Army Parachutist Badge")
        .with_location("Badges Database", "C", 11)
        .with_cell("alice", "Army Parachutist Badge");
    let a3 = Award::new("badges", "Army Parachutist Badge")
        .with_location("Badges Database", "C", 12)
        .with_cell("bob", "Army Parachutist Badge");
    let mut index = HashMap::new();
    index.insert("alice".into(), vec![a1, a2]);
    index.insert("bob".into(), vec![a3]);
    let mut sheet_rows = HashMap::new();
    sheet_rows.insert("Badges Database".into(), rows);
    let data = AwardsData {
        index,
        catalog: vec![],
        sheet_rows,
    };
    let hits = find_duplicates_for_user(&data, "alice");
    let primary = awards_excluding_duplicate_rows(
        &flatten_awards_sorted(&get_awards_for_username(&data.index, "alice")),
        &hits,
    );
    assert!(primary.is_empty());
    assert_eq!(hits.len(), 2);
}

#[test]
fn test_upsert_award_moves_username() {
    let award = Award::new("ribbons", "Army Achievement")
        .with_location("Ribbons Database", "C", 5)
        .with_cell("Alice", "Army Achievement");
    let mut index = HashMap::new();
    index.insert("alice".into(), vec![award]);
    let moved = Award::new("ribbons", "Army Achievement")
        .with_location("Ribbons Database", "C", 5)
        .with_cell("Bob x2", "Army Achievement");
    let key = upsert_award_in_index(&mut index, &moved);
    assert_eq!(key.as_deref(), Some("bob"));
    assert!(!index.contains_key("alice") || index["alice"].is_empty());
    assert_eq!(index["bob"].len(), 1);
    assert_eq!(index["bob"][0].cell, "Bob x2");
    drop_award_location(&mut index, "Ribbons Database", "C", 5);
    assert!(!index.contains_key("bob") || index["bob"].is_empty());
}

#[test]
fn test_shift_column_up_on_delete() {
    let mut rows = vec![
        vec!["".into(), "".into(), "Title".into()],
        vec!["".into(), "".into(), "alice".into()],
        vec!["".into(), "".into(), "bob".into()],
        vec!["".into(), "".into(), "carol".into()],
    ];
    let mut snapshot = rows.clone();
    shift_column_up_in_rows(&mut snapshot, "Ribbons Database", "C", 99);
    assert_eq!(snapshot[3][2], "carol");

    shift_column_up_in_rows(&mut rows, "Ribbons Database", "C", 11); // delete bob
    assert_eq!(rows[1][2], "alice");
    assert_eq!(rows[2][2], "carol");
    assert_eq!(rows[3][2], "");

    let a_alice = Award::new("ribbons", "A")
        .with_location("Ribbons Database", "C", 10)
        .with_cell("alice", "");
    let a_bob = Award::new("ribbons", "B")
        .with_location("Ribbons Database", "C", 11)
        .with_cell("bob", "");
    let a_carol = Award::new("ribbons", "C")
        .with_location("Ribbons Database", "C", 12)
        .with_cell("carol", "");
    let mut index = HashMap::new();
    index.insert("alice".into(), vec![a_alice]);
    index.insert("bob".into(), vec![a_bob]);
    index.insert("carol".into(), vec![a_carol]);
    reindex_column_after_delete(&mut index, "Ribbons Database", "C", 11);
    assert!(!index.contains_key("bob"));
    assert_eq!(index["carol"][0].row, 11);
    assert_eq!(index["alice"][0].row, 10);
}

#[test]
fn test_owned_award_columns_ignores_similar_cells() {
    let alice = Award::new("ribbons", "A")
        .with_location("Ribbons Database", "C", 10)
        .with_cell("alice", "");
    let alicie = Award::new("ribbons", "A")
        .with_location("Ribbons Database", "D", 11)
        .with_cell("alicie", "");
    let owned = owned_award_columns(&[alice, alicie], "alice");
    assert_eq!(
        owned,
        [("Ribbons Database".into(), "C".into())]
            .into_iter()
            .collect()
    );
}

#[test]
fn test_match_row_in_window() {
    let values = vec![
        vec![],
        vec!["Shadow325418 - 75th CSIB".into()],
        vec!["NovaStorm_Commader - ASF CSIB".into()],
    ];
    assert_eq!(
        match_row_in_window(&values, 4413, "NovaStorm_Commader - ASF CSIB", 4413),
        Some(4415)
    );
    let both = vec![
        vec!["alice".into()],
        vec!["bob".into()],
        vec!["alice".into()],
    ];
    assert_eq!(match_row_in_window(&both, 10, "alice", 12), Some(12));
}
