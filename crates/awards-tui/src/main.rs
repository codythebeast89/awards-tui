use awards_core::{
    collect_sheet_audit, format_audit_report, get_awards_for_username, group_awards,
    CATEGORY_LABELS,
};
use awards_sheets::{
    add_award_to_user, auth_status, build_awards_data, credentials_path, login, project_root,
    service_account_path, token_path,
};
use chrono::Utc;
use clap::Parser;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "awards-tui",
    about = "Look up and edit decorations in the FORSCOM Decorations Database.",
    version
)]
struct Cli {
    /// If provided alone, print awards once and exit (no TUI).
    username: Option<String>,

    /// Force non-interactive lookup.
    #[arg(long)]
    cli: bool,

    #[arg(long)]
    login: bool,

    #[arg(long)]
    auth_status: bool,

    /// Read-only scan for duplicates; writes a timestamped .txt report under audits/.
    #[arg(long)]
    audit: bool,

    /// Write the --audit report to FILE instead of audits/audit-TIMESTAMP.txt.
    #[arg(long, value_name = "FILE")]
    audit_out: Option<PathBuf>,

    #[arg(long, value_name = "AWARD")]
    add: Option<String>,

    /// Optional cell suffix when using --add (e.g. x2).
    #[arg(long, default_value = "")]
    suffix: String,
}

fn print_awards(username: &str) -> anyhow::Result<ExitCode> {
    eprintln!("Syncing awards from Google Sheets…");
    let data = build_awards_data(None)?;
    let awards = get_awards_for_username(&data.index, username);
    if awards.is_empty() {
        println!("No awards found for {username}");
        return Ok(ExitCode::from(1));
    }
    println!("Awards for @{username} ({} total)\n", awards.len());
    let grouped = group_awards(&awards);
    for (_, label) in CATEGORY_LABELS {
        let names = grouped.get(*label).cloned().unwrap_or_default();
        println!("{label} ({})", names.len());
        if names.is_empty() {
            println!("  (none)");
        } else {
            for name in names {
                println!("  • {name}");
            }
        }
        println!();
    }
    Ok(ExitCode::SUCCESS)
}

fn cmd_audit(out_path: Option<PathBuf>) -> anyhow::Result<ExitCode> {
    eprintln!("Syncing awards from Google Sheets (read-only)…");
    let data = build_awards_data(None)?;
    let report = collect_sheet_audit(&data);
    let generated = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
    let body = format_audit_report(&report, &generated);

    let dest = if let Some(path) = out_path {
        path
    } else {
        let stamp = Utc::now().format("%Y-%m-%d_%H%M%S");
        project_root().join("audits").join(format!("audit-{stamp}.txt"))
    };
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&dest, body)?;

    let identical = report
        .duplicate_groups
        .iter()
        .filter(|g| g.kind == "identical")
        .count();
    let conflict = report
        .duplicate_groups
        .iter()
        .filter(|g| g.kind == "conflict")
        .count();
    println!("Wrote {}", dest.display());
    println!(
        "{} columns · {} cells · {} identical copies · {} conflicting · {} similar · {} malformed",
        report.columns,
        report.cells,
        identical,
        conflict,
        report.similar_pairs.len(),
        report.malformed.len()
    );
    Ok(ExitCode::SUCCESS)
}

fn cmd_login() -> ExitCode {
    println!("Starting Google OAuth login (browser will open)…");
    match login() {
        Ok(hint) => {
            println!("Logged in ({hint}). token.json saved — you can use add/edit/delete.");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("Login failed: {err}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_auth_status() -> ExitCode {
    let status = auth_status();
    println!("status: {status}");
    println!(
        "oauth client: {}",
        credentials_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(none)".into())
    );
    println!(
        "service account: {}",
        service_account_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(none)".into())
    );
    let tok = token_path();
    println!(
        "token: {}",
        if tok.is_file() {
            tok.display().to_string()
        } else {
            "(none)".into()
        }
    );
    ExitCode::SUCCESS
}

fn cmd_add(username: &str, award_query: &str, suffix: &str) -> anyhow::Result<ExitCode> {
    eprintln!("Syncing catalog…");
    let data = build_awards_data(None)?;
    let q = award_query.to_ascii_lowercase();
    let matches: Vec<_> = data
        .catalog
        .iter()
        .filter(|d| d.base_name.to_ascii_lowercase().contains(&q))
        .collect();
    if matches.is_empty() {
        eprintln!("No award matched {award_query:?}");
        return Ok(ExitCode::from(1));
    }
    if matches.len() > 1 {
        eprintln!("Ambiguous ({} matches). Be more specific:", matches.len());
        for d in matches.iter().take(20) {
            let label = CATEGORY_LABELS
                .iter()
                .find(|(cat, _)| *cat == d.category)
                .map(|(_, l)| *l)
                .unwrap_or(d.category.as_str());
            eprintln!("  [{label}] {}", d.base_name);
        }
        return Ok(ExitCode::from(1));
    }
    let award_def = matches[0];
    let result = add_award_to_user(username, award_def, suffix, false);
    println!("{}", result.message);
    Ok(if result.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    if cli.login {
        return cmd_login();
    }
    if cli.auth_status {
        return cmd_auth_status();
    }
    if cli.audit || cli.audit_out.is_some() {
        return match cmd_audit(cli.audit_out) {
            Ok(code) => code,
            Err(err) => {
                eprintln!("{err:#}");
                ExitCode::FAILURE
            }
        };
    }
    if let Some(award) = cli.add.as_deref() {
        let Some(username) = cli.username.as_deref() else {
            eprintln!("username is required with --add");
            return ExitCode::from(2);
        };
        return match cmd_add(username, award, &cli.suffix) {
            Ok(code) => code,
            Err(err) => {
                eprintln!("{err:#}");
                ExitCode::FAILURE
            }
        };
    }
    if cli.username.is_some() || cli.cli {
        let Some(username) = cli.username.as_deref() else {
            eprintln!("username is required with --cli");
            return ExitCode::from(2);
        };
        return match print_awards(username) {
            Ok(code) => code,
            Err(err) => {
                eprintln!("{err:#}");
                ExitCode::FAILURE
            }
        };
    }

    eprintln!("TUI not implemented yet (M4). Use `python3 main.py` for the Textual UI.");
    eprintln!("Or: cargo run -p awards-tui -- <username> / --audit / --auth-status / --add …");
    ExitCode::from(2)
}
