use awards_core::{
    collect_sheet_audit, format_audit_report, get_awards_for_username, group_awards,
    CATEGORY_LABELS,
};
use awards_sheets::{
    add_award_to_user, auth_status, build_awards_data, credentials_path, login, project_root,
    remove_award, rename_username, service_account_path, token_path, update_award_cell,
};
use chrono::Utc;
use clap::Parser;
use std::path::PathBuf;
use std::process::ExitCode;

mod config;
mod tui;

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

    /// Edit an existing award cell for USERNAME (match by award name).
    #[arg(long, value_name = "AWARD")]
    edit: Option<String>,

    /// New cell value when using --edit (e.g. "User x2").
    #[arg(long, value_name = "CELL")]
    cell: Option<String>,

    /// Delete an existing award for USERNAME (match by award name).
    #[arg(long, value_name = "AWARD")]
    delete: Option<String>,

    /// Rewrite USERNAME to NEW across every matching sheet cell.
    #[arg(long, value_name = "NEW")]
    rename: Option<String>,

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
        project_root()
            .join("audits")
            .join(format!("audit-{stamp}.txt"))
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
    println!("Starting Google login…");
    match login() {
        Ok(message) => {
            println!("{message}");
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
    println!("data root: {}", project_root().display());
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

fn find_user_award<'a>(
    awards: &'a [awards_core::Award],
    award_query: &str,
) -> Result<&'a awards_core::Award, ExitCode> {
    let q = award_query.to_ascii_lowercase();
    let matches: Vec<_> = awards
        .iter()
        .filter(|a| {
            a.name.to_ascii_lowercase().contains(&q)
                || a.base_name.to_ascii_lowercase().contains(&q)
        })
        .collect();
    if matches.is_empty() {
        eprintln!("No owned award matched {award_query:?}");
        return Err(ExitCode::from(1));
    }
    if matches.len() > 1 {
        eprintln!("Ambiguous ({} matches). Be more specific:", matches.len());
        for a in matches.iter().take(20) {
            eprintln!(
                "  {} · {}{} · {:?}",
                a.name,
                a.col,
                a.row,
                a.cell
            );
        }
        return Err(ExitCode::from(1));
    }
    Ok(matches[0])
}

fn cmd_edit(username: &str, award_query: &str, new_cell: &str) -> anyhow::Result<ExitCode> {
    eprintln!("Syncing awards…");
    let data = build_awards_data(None)?;
    let awards = get_awards_for_username(&data.index, username);
    let award = match find_user_award(&awards, award_query) {
        Ok(a) => a.clone(),
        Err(code) => return Ok(code),
    };
    let result = update_award_cell(&award, new_cell, false);
    println!("{}", result.message);
    Ok(if result.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn cmd_delete(username: &str, award_query: &str) -> anyhow::Result<ExitCode> {
    eprintln!("Syncing awards…");
    let data = build_awards_data(None)?;
    let awards = get_awards_for_username(&data.index, username);
    let award = match find_user_award(&awards, award_query) {
        Ok(a) => a.clone(),
        Err(code) => return Ok(code),
    };
    let result = remove_award(&award, false);
    println!("{}", result.message);
    Ok(if result.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn cmd_rename(old_username: &str, new_username: &str) -> anyhow::Result<ExitCode> {
    eprintln!("Syncing awards…");
    let data = build_awards_data(None)?;
    let result = rename_username(old_username, new_username, Some(&data), false);
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
    if let Some(award) = cli.edit.as_deref() {
        let Some(username) = cli.username.as_deref() else {
            eprintln!("username is required with --edit");
            return ExitCode::from(2);
        };
        let Some(cell) = cli.cell.as_deref() else {
            eprintln!("--cell is required with --edit");
            return ExitCode::from(2);
        };
        return match cmd_edit(username, award, cell) {
            Ok(code) => code,
            Err(err) => {
                eprintln!("{err:#}");
                ExitCode::FAILURE
            }
        };
    }
    if let Some(award) = cli.delete.as_deref() {
        let Some(username) = cli.username.as_deref() else {
            eprintln!("username is required with --delete");
            return ExitCode::from(2);
        };
        return match cmd_delete(username, award) {
            Ok(code) => code,
            Err(err) => {
                eprintln!("{err:#}");
                ExitCode::FAILURE
            }
        };
    }
    if let Some(new_username) = cli.rename.as_deref() {
        let Some(username) = cli.username.as_deref() else {
            eprintln!("username is required with --rename");
            return ExitCode::from(2);
        };
        return match cmd_rename(username, new_username) {
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

    match tui::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err:#}");
            ExitCode::FAILURE
        }
    }
}
