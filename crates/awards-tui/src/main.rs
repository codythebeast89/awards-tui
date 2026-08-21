use awards_core::{
    collect_sheet_audit, format_audit_report, get_awards_for_username, group_awards,
    CATEGORY_LABELS,
};
use awards_sheets::build_awards_data;
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

fn project_root() -> PathBuf {
    // Prefer cwd (repo root when developing); fall back to crate → workspace root.
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if cwd.join("award_columns.json").is_file() {
        return cwd;
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
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

fn main() -> ExitCode {
    let cli = Cli::parse();

    if cli.login {
        eprintln!("--login not implemented yet (M3). Use `python3 main.py --login`.");
        return ExitCode::from(2);
    }
    if cli.auth_status {
        eprintln!("--auth-status not implemented yet (M3). Use `python3 main.py --auth-status`.");
        return ExitCode::from(2);
    }
    if cli.add.is_some() {
        eprintln!("--add not implemented yet (M3). Use `python3 main.py … --add …`.");
        return ExitCode::from(2);
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
    eprintln!("Or: cargo run -p awards-tui -- <username>   /   --audit");
    ExitCode::from(2)
}
