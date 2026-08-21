use regex::Regex;
use std::sync::OnceLock;

fn badge_abbrev_special(key: &str) -> Option<&'static str> {
    match key {
        "ESB" => Some("Expert Soldier Badge"),
        _ => None,
    }
}

pub fn ordinal_award(n: i32) -> String {
    match n {
        2 => "2nd Award".to_string(),
        3 => "3rd Award".to_string(),
        _ => format!("{n}th Award"),
    }
}

pub fn format_ribbon_award(base_name: &str, cell: &str) -> String {
    static DEVICE: OnceLock<Regex> = OnceLock::new();
    static COUNT: OnceLock<Regex> = OnceLock::new();
    let device = DEVICE.get_or_init(|| Regex::new(r#"-\s*"([^"]+)""#).unwrap());
    let count = COUNT.get_or_init(|| Regex::new(r"(?i)\bx(\d+)\b").unwrap());

    let mut name = base_name.trim().to_string();
    if let Some(caps) = device.captures(cell) {
        name.push_str(&format!(" (\"{}\")", &caps[1]));
    }
    if let Some(caps) = count.captures(cell) {
        let n: i32 = caps[1].parse().unwrap_or(0);
        name.push_str(&format!(" ({})", ordinal_award(n)));
    }
    name
}

fn cjs_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)\(?\s*(?:(\d+)\s*x|x\s*(\d+))\s*CJS\s*\)?").unwrap())
}

pub fn cjs_phrase(n: i32) -> String {
    if n <= 1 {
        "Combat Jump Star".to_string()
    } else {
        format!("{n} Combat Jump Stars")
    }
}

pub fn extract_cjs(cell: &str) -> (String, Option<String>) {
    let re = cjs_re();
    let Some(caps) = re.captures(cell) else {
        return (cell.to_string(), None);
    };
    let n: i32 = caps
        .get(1)
        .or_else(|| caps.get(2))
        .and_then(|m| m.as_str().parse().ok())
        .unwrap_or(0);
    let mut rest = re.replace_all(cell, "").into_owned();
    static EMPTY_PARENS: OnceLock<Regex> = OnceLock::new();
    static TRAIL_DASH: OnceLock<Regex> = OnceLock::new();
    static SPACES: OnceLock<Regex> = OnceLock::new();
    let empty = EMPTY_PARENS.get_or_init(|| Regex::new(r"\(\s*\)").unwrap());
    let trail = TRAIL_DASH.get_or_init(|| Regex::new(r"\s*-\s*$").unwrap());
    let spaces = SPACES.get_or_init(|| Regex::new(r"\s+").unwrap());
    rest = empty.replace_all(&rest, "").into_owned();
    rest = trail.replace_all(&rest, "").into_owned();
    rest = spaces.replace_all(&rest, " ").trim().to_string();
    (rest, Some(cjs_phrase(n)))
}

pub fn attach_cjs(name: &str, cjs: Option<&str>) -> String {
    let Some(cjs) = cjs else {
        return name.to_string();
    };
    if name.ends_with(')') && name.contains('(') {
        format!("{}, {cjs})", &name[..name.len() - 1])
    } else {
        format!("{name} ({cjs})")
    }
}

pub fn expand_badge_abbrev(base_name: &str, abbrev: &str) -> String {
    let base = base_name.trim();
    let key = abbrev.trim().to_ascii_uppercase();
    if key == "MC" {
        if base.to_ascii_lowercase().starts_with("master ") {
            return base.to_string();
        }
        return format!("Master {base}");
    }
    if let Some(special) = badge_abbrev_special(&key) {
        return special.to_string();
    }
    base.to_string()
}

pub fn format_badge_award(base_name: &str, cell: &str) -> String {
    static COUNT: OnceLock<Regex> = OnceLock::new();
    static XCOUNT: OnceLock<Regex> = OnceLock::new();
    let count_re = COUNT.get_or_init(|| Regex::new(r"(?i)\bx(\d+)\b").unwrap());
    let xcount = XCOUNT.get_or_init(|| Regex::new(r"(?i)\s*x\d+\b").unwrap());

    let base = base_name.trim();
    let (cell, cjs) = extract_cjs(cell);
    let cjs_ref = cjs.as_deref();
    let dash = cell.find(" - ");
    let Some(dash) = dash else {
        return attach_cjs(&format_ribbon_award(base, &cell), cjs_ref);
    };
    let detail = cell[dash + 3..].trim();
    if detail.is_empty() {
        return attach_cjs(&format_ribbon_award(base, &cell), cjs_ref);
    }

    let count = count_re
        .captures(&cell)
        .and_then(|c| c[1].parse::<i32>().ok());
    let mut label = xcount.replace_all(detail, "").into_owned();
    label = label.split_whitespace().collect::<Vec<_>>().join(" ");
    label = label.trim_matches(|c: char| c == ' ' || c == '-').to_string();

    let label_upper = label.to_ascii_uppercase();
    if (label_upper == "MC" || label_upper == "ESB") && !detail.contains(',') {
        let mut name = expand_badge_abbrev(base, &label);
        if let Some(n) = count {
            name.push_str(&format!(" ({})", ordinal_award(n)));
        }
        return attach_cjs(&name, cjs_ref);
    }

    if let Some(n) = count {
        if !detail.contains(',') {
            let mut name = base.to_string();
            if !label.is_empty() && !name.to_ascii_lowercase().contains(&label.to_ascii_lowercase())
            {
                if label.len() <= 4 {
                    name = expand_badge_abbrev(base, &label);
                } else {
                    name = format!("{name} ({label})");
                }
            }
            name.push_str(&format!(" ({})", ordinal_award(n)));
            return attach_cjs(&name, cjs_ref);
        }
    }

    attach_cjs(&format!("{base} ({detail})"), cjs_ref)
}

pub fn format_award_name(category: &str, base_name: Option<&str>, cell: &str) -> Option<String> {
    let base = base_name?.trim();
    if base.is_empty() {
        return None;
    }
    Some(if category == "badges" {
        format_badge_award(base, cell)
    } else {
        format_ribbon_award(base, cell)
    })
}
