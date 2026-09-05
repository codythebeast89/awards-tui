//! Clerk assist: check whether a requested decoration may be granted.
//!
//! Pure logic over an in-memory award list — no Sheets or Discord I/O.
//! Master combat badges (MCAB / MCIB / MCMB): expert badge + combat badge → `- MC`.

use crate::parse::{build_cell_value, clean_cell};
use crate::types::Award;

/// Known clerk request kinds with prerequisite rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssistAward {
    /// Master Combat Action Badge — ESB + CAB.
    MasterCab,
    /// Master Combat Infantryman Badge — EIB + CIB.
    MasterCib,
    /// Master Combat Medical Badge — EFMB + CMB.
    MasterCmb,
}

impl AssistAward {
    pub fn id(self) -> &'static str {
        match self {
            Self::MasterCab => "MCAB",
            Self::MasterCib => "MCIB",
            Self::MasterCmb => "MCMB",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::MasterCab => "Master Combat Action Badge",
            Self::MasterCib => "Master Combat Infantryman Badge",
            Self::MasterCmb => "Master Combat Medical Badge",
        }
    }

    fn short_name(self) -> &'static str {
        match self {
            Self::MasterCab => "Master CAB",
            Self::MasterCib => "Master CIB",
            Self::MasterCmb => "Master CMB",
        }
    }

    fn rule(self) -> MasterCombatRule {
        match self {
            Self::MasterCab => MasterCombatRule {
                award: self,
                combat_base: "Combat Action Badge",
                combat_short: "CAB",
                expert_base: "Expert Soldier Badge",
                expert_short: "ESB",
                expert_cell_tag: "ESB",
            },
            Self::MasterCib => MasterCombatRule {
                award: self,
                combat_base: "Combat Infantryman Badge",
                combat_short: "CIB",
                expert_base: "Expert Infantryman Badge",
                expert_short: "EIB",
                expert_cell_tag: "EIB",
            },
            Self::MasterCmb => MasterCombatRule {
                award: self,
                combat_base: "Combat Medical Badge",
                combat_short: "CMB",
                expert_base: "Expert Field Medical Badge",
                expert_short: "EFMB",
                expert_cell_tag: "EFMB",
            },
        }
    }
}

struct MasterCombatRule {
    award: AssistAward,
    combat_base: &'static str,
    combat_short: &'static str,
    expert_base: &'static str,
    expert_short: &'static str,
    expert_cell_tag: &'static str,
}

/// How to apply an approved grant on the Decorations Database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrantPlan {
    /// Find the user's award whose `base_name` matches, rewrite cell to `new_cell`.
    UpgradeCell {
        base_name: String,
        new_cell: String,
    },
}

/// Discord / clerk workflow reminders (manual — no bot).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssistReminders {
    pub forward_to_clerks_entries: bool,
    pub forward_to_pending_approvals: bool,
    pub denial_channel: &'static str,
}

impl AssistReminders {
    fn for_award(award: AssistAward) -> Self {
        match award {
            AssistAward::MasterCab | AssistAward::MasterCib | AssistAward::MasterCmb => Self {
                forward_to_clerks_entries: true,
                forward_to_pending_approvals: false,
                denial_channel: "request-denial",
            },
        }
    }

    pub fn lines(&self, approved: bool) -> Vec<String> {
        let mut out = Vec::new();
        if approved {
            if self.forward_to_clerks_entries {
                out.push(
                    "Forward the original request to #logistics-clerks-entries as proof."
                        .to_string(),
                );
            }
            if self.forward_to_pending_approvals {
                out.push("Also forward to #pending-approvals before granting.".to_string());
            }
        } else {
            out.push(format!(
                "Ping the requester in #{} with the denial reason.",
                self.denial_channel
            ));
        }
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssistVerdict {
    Approve {
        award: AssistAward,
        grant: GrantPlan,
        reminders: AssistReminders,
    },
    Deny {
        award: AssistAward,
        reasons: Vec<String>,
        /// Draft Discord denial text (without the @mention).
        denial_message: String,
        reminders: AssistReminders,
    },
    AlreadyHas {
        award: AssistAward,
        cell: String,
    },
    Unknown {
        query: String,
    },
}

impl AssistVerdict {
    pub fn approved(&self) -> bool {
        matches!(self, Self::Approve { .. })
    }

    pub fn format_report(&self, username: &str) -> String {
        match self {
            Self::Approve {
                award,
                grant,
                reminders,
            } => {
                let mut lines = vec![
                    format!("APPROVE @{username} → {}", award.display_name()),
                    String::new(),
                    "Prerequisites: satisfied.".to_string(),
                ];
                match grant {
                    GrantPlan::UpgradeCell { base_name, new_cell } => {
                        lines.push(format!("Grant plan: edit `{base_name}` → `{new_cell}`"));
                    }
                }
                lines.push(String::new());
                lines.push("Next (manual Discord):".to_string());
                for tip in reminders.lines(true) {
                    lines.push(format!("  • {tip}"));
                }
                lines.join("\n")
            }
            Self::Deny {
                award,
                reasons,
                denial_message,
                reminders,
            } => {
                let mut lines = vec![
                    format!("DENY @{username} → {}", award.display_name()),
                    String::new(),
                    "Missing / failed:".to_string(),
                ];
                for reason in reasons {
                    lines.push(format!("  • {reason}"));
                }
                lines.push(String::new());
                lines.push(format!("Draft denial: {denial_message}"));
                lines.push(String::new());
                lines.push("Next (manual Discord):".to_string());
                for tip in reminders.lines(false) {
                    lines.push(format!("  • {tip}"));
                }
                lines.join("\n")
            }
            Self::AlreadyHas { award, cell } => format!(
                "ALREADY HAS @{username} → {}\nCurrent cell: {cell}\nNo sheet write needed.",
                award.display_name()
            ),
            Self::Unknown { query } => format!(
                "Unknown award request {query:?}.\nKnown: MCAB, MCIB, MCMB (Master CAB/CIB/CMB)."
            ),
        }
    }
}

/// Resolve common clerk aliases to a known assist award.
pub fn parse_assist_award(query: &str) -> Option<AssistAward> {
    let q = query.trim().to_ascii_lowercase();
    let compact: String = q.chars().filter(|c| !c.is_whitespace()).collect();
    match compact.as_str() {
        "mcab" | "mastercab" | "mastercombatactionbadge" | "mastercombataction" => {
            Some(AssistAward::MasterCab)
        }
        "mcib" | "mastercib" | "mastercombatinfantrymanbadge" | "mastercombatinfantryman" => {
            Some(AssistAward::MasterCib)
        }
        "mcmb" | "mastercmb" | "mastercombatmedicalbadge" | "mastercombatmedical" => {
            Some(AssistAward::MasterCmb)
        }
        _ if q.contains("master") && q.contains("combat") && q.contains("action") => {
            Some(AssistAward::MasterCab)
        }
        _ if q.contains("master") && q.contains("infantryman") => Some(AssistAward::MasterCib),
        _ if q.contains("master") && q.contains("medical") && q.contains("combat") => {
            Some(AssistAward::MasterCmb)
        }
        _ if q == "master cab" || q.starts_with("master cab") => Some(AssistAward::MasterCab),
        _ if q == "master cib" || q.starts_with("master cib") => Some(AssistAward::MasterCib),
        _ if q == "master cmb" || q.starts_with("master cmb") => Some(AssistAward::MasterCmb),
        _ => None,
    }
}

fn base_eq(award: &Award, want: &str) -> bool {
    award.base_name.eq_ignore_ascii_case(want)
}

fn name_eq(award: &Award, want: &str) -> bool {
    award.name.eq_ignore_ascii_case(want)
}

fn cell_has_tag(cell: &str, tag: &str) -> bool {
    let upper = cell.to_ascii_uppercase();
    let tag = tag.to_ascii_uppercase();
    upper.contains(&format!(" - {tag}")) || upper.ends_with(&format!(" {tag}"))
}

fn has_expert_badge(awards: &[Award], rule: &MasterCombatRule) -> bool {
    awards.iter().any(|a| {
        base_eq(a, rule.expert_base)
            || name_eq(a, rule.expert_base)
            || cell_has_tag(&a.cell, rule.expert_cell_tag)
    })
}

fn combat_rows<'a>(awards: &'a [Award], combat_base: &str) -> Vec<&'a Award> {
    awards
        .iter()
        .filter(|a| base_eq(a, combat_base))
        .collect()
}

fn is_master_on_combat(award: &Award, display_name: &str) -> bool {
    name_eq(award, display_name)
        || award
            .name
            .to_ascii_lowercase()
            .starts_with(&display_name.to_ascii_lowercase())
        || cell_has_tag(&award.cell, "MC")
}

/// Check eligibility for a clerk assist request against the user's awards.
pub fn check_assist(username: &str, awards: &[Award], query: &str) -> AssistVerdict {
    let Some(award) = parse_assist_award(query) else {
        return AssistVerdict::Unknown {
            query: query.trim().to_string(),
        };
    };
    let reminders = AssistReminders::for_award(award);
    let display_user = username.trim().trim_start_matches('@');
    check_master_combat(display_user, awards, award.rule(), reminders)
}

/// Prefer the casing already on the sheet cell; fall back to the typed username.
fn sheet_username_display(cell: &str, fallback: &str) -> String {
    let text = clean_cell(Some(cell));
    if text.is_empty() {
        return fallback.to_string();
    }
    if let Some(dash) = text.find(" - ") {
        return text[..dash].trim().to_string();
    }
    // Strip trailing " xN" ordinal if present.
    if let Some(idx) = text.to_ascii_lowercase().rfind(" x") {
        let rest = &text[idx + 2..];
        if rest.chars().all(|c| c.is_ascii_digit()) {
            return text[..idx].trim().to_string();
        }
    }
    text
}

fn check_master_combat(
    username: &str,
    awards: &[Award],
    rule: MasterCombatRule,
    reminders: AssistReminders,
) -> AssistVerdict {
    let rows = combat_rows(awards, rule.combat_base);
    let master_name = rule.award.display_name();
    if let Some(existing) = rows.iter().find(|a| is_master_on_combat(a, master_name)) {
        return AssistVerdict::AlreadyHas {
            award: rule.award,
            cell: existing.cell.clone(),
        };
    }

    let mut missing = Vec::new();
    if !has_expert_badge(awards, &rule) {
        missing.push(format!(
            "Missing {} ({})",
            rule.expert_base, rule.expert_short
        ));
    }
    if rows.is_empty() {
        missing.push(format!(
            "Missing {} ({})",
            rule.combat_base, rule.combat_short
        ));
    }

    if !missing.is_empty() {
        let denial_message = if missing.len() == 2 {
            format!(
                "Request requires both {} and {} before {}.",
                rule.expert_base,
                rule.combat_base,
                rule.award.short_name()
            )
        } else {
            let need = if missing[0].contains("Expert") || missing[0].contains("Field Medical") {
                rule.expert_base
            } else {
                rule.combat_base
            };
            format!(
                "Request {} after obtaining {}.",
                rule.award.short_name(),
                need
            )
        };
        return AssistVerdict::Deny {
            award: rule.award,
            reasons: missing,
            denial_message,
            reminders,
        };
    }

    let sheet_user = rows
        .first()
        .map(|a| sheet_username_display(&a.cell, username))
        .unwrap_or_else(|| username.to_string());
    let new_cell = build_cell_value(&sheet_user, "MC");
    AssistVerdict::Approve {
        award: rule.award,
        grant: GrantPlan::UpgradeCell {
            base_name: rule.combat_base.to_string(),
            new_cell,
        },
        reminders,
    }
}

/// Find the sheet award to upgrade for a [`GrantPlan::UpgradeCell`].
pub fn find_grant_target<'a>(awards: &'a [Award], plan: &GrantPlan) -> Option<&'a Award> {
    match plan {
        GrantPlan::UpgradeCell { base_name, .. } => awards
            .iter()
            .find(|a| a.base_name.eq_ignore_ascii_case(base_name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn award(base: &str, name: &str, cell: &str) -> Award {
        Award::new("badges", name).with_cell(cell, base)
    }

    #[test]
    fn parse_master_aliases() {
        assert_eq!(parse_assist_award("MCAB"), Some(AssistAward::MasterCab));
        assert_eq!(parse_assist_award("MCIB"), Some(AssistAward::MasterCib));
        assert_eq!(parse_assist_award("MCMB"), Some(AssistAward::MasterCmb));
        assert_eq!(
            parse_assist_award("Master CIB"),
            Some(AssistAward::MasterCib)
        );
        assert_eq!(
            parse_assist_award("Master Combat Medical Badge"),
            Some(AssistAward::MasterCmb)
        );
        assert_eq!(parse_assist_award("OSB"), None);
    }

    #[test]
    fn mcab_approve_with_esb_and_cab() {
        let awards = vec![
            award(
                "Expert Soldier Badge",
                "Expert Soldier Badge",
                "MaoZhuuDong",
            ),
            award("Combat Action Badge", "Combat Action Badge", "MaoZhuuDong"),
        ];
        let v = check_assist("MaoZhuuDong", &awards, "MCAB");
        match v {
            AssistVerdict::Approve { grant, .. } => match grant {
                GrantPlan::UpgradeCell { base_name, new_cell } => {
                    assert_eq!(base_name, "Combat Action Badge");
                    assert_eq!(new_cell, "MaoZhuuDong - MC");
                }
            },
            other => panic!("expected Approve, got {other:?}"),
        }
    }

    #[test]
    fn mcab_deny_missing_esb() {
        let awards = vec![award(
            "Combat Action Badge",
            "Combat Action Badge",
            "alice",
        )];
        let v = check_assist("alice", &awards, "Master CAB");
        match v {
            AssistVerdict::Deny { reasons, .. } => {
                assert!(reasons.iter().any(|r| r.contains("Expert Soldier")));
            }
            other => panic!("expected Deny, got {other:?}"),
        }
    }

    #[test]
    fn mcab_already_has() {
        let awards = vec![award(
            "Combat Action Badge",
            "Master Combat Action Badge",
            "bob - MC",
        )];
        let v = check_assist("bob", &awards, "MCAB");
        assert!(matches!(v, AssistVerdict::AlreadyHas { .. }));
    }

    #[test]
    fn mcib_approve_with_eib_and_cib() {
        let awards = vec![
            award(
                "Expert Infantryman Badge",
                "Expert Infantryman Badge",
                "user",
            ),
            award(
                "Combat Infantryman Badge",
                "Combat Infantryman Badge",
                "user",
            ),
        ];
        let v = check_assist("user", &awards, "MCIB");
        match v {
            AssistVerdict::Approve { grant, award, .. } => {
                assert_eq!(award, AssistAward::MasterCib);
                match grant {
                    GrantPlan::UpgradeCell { base_name, new_cell } => {
                        assert_eq!(base_name, "Combat Infantryman Badge");
                        assert_eq!(new_cell, "user - MC");
                    }
                }
            }
            other => panic!("expected Approve, got {other:?}"),
        }
    }

    #[test]
    fn mcmb_deny_missing_cmb() {
        let awards = vec![award(
            "Expert Field Medical Badge",
            "Expert Field Medical Badge",
            "doc",
        )];
        let v = check_assist("doc", &awards, "MCMB");
        match v {
            AssistVerdict::Deny { reasons, .. } => {
                assert!(reasons.iter().any(|r| r.contains("Combat Medical")));
            }
            other => panic!("expected Deny, got {other:?}"),
        }
    }

    #[test]
    fn mcmb_approve_with_efmb_and_cmb() {
        let awards = vec![
            award(
                "Expert Field Medical Badge",
                "Expert Field Medical Badge",
                "MedicOne",
            ),
            award("Combat Medical Badge", "Combat Medical Badge", "MedicOne"),
        ];
        assert!(check_assist("MedicOne", &awards, "Master CMB").approved());
    }
}
