use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "awards-tui",
    about = "Look up and edit FORSCOM decorations (Rust rewrite in progress; Python UI is still default)",
    version
)]
struct Cli {
    /// Username to look up (CLI print; TUI when omitted — not yet implemented)
    username: Option<String>,

    #[arg(long)]
    login: bool,

    #[arg(long)]
    auth_status: bool,

    #[arg(long)]
    audit: bool,

    #[arg(long)]
    audit_out: Option<String>,

    #[arg(long)]
    add: Option<String>,

    #[arg(long)]
    suffix: Option<String>,
}

fn main() {
    let cli = Cli::parse();
    if cli.login || cli.auth_status || cli.audit || cli.add.is_some() || cli.username.is_some() {
        eprintln!(
            "Rust CLI stubs only — use `python3 main.py` until M2/M3. Try: cargo run -p awards-tui -- --help"
        );
        std::process::exit(2);
    }
    eprintln!("TUI not implemented yet (M4). Use `python3 main.py` for the Textual UI.");
    std::process::exit(2);
}
