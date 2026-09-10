pub mod app;
mod ui;

use app::{App, WorkerMsg};
use crossterm::{
    cursor::{SetCursorStyle, Show},
    event::{self, DisableBracketedPaste, EnableBracketedPaste, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self,Stdout};
use std::ops::{Deref, DerefMut};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const TICK_RATE: Duration = Duration::from_millis(100);

struct TerminalSession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    restored: bool,
}

impl TerminalSession {
    fn new() -> anyhow::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        // Bracketed paste lets a multi-line Discord message paste into the TUI as one atomic
        // `Event::Paste` instead of a burst of individual keystrokes (003-discord-paste-quick-add,
        // research.md §1). Not every terminal supports it; failing to enable it is a graceful
        // degradation (the clerk falls back to manual Lookup + Add), not a fatal error.
        if let Err(err) = execute!(
            stdout,
            EnterAlternateScreen,
            SetCursorStyle::BlinkingBar,
            EnableBracketedPaste
        ) {
            let _ = disable_raw_mode();
            return Err(err.into());
        }
        let backend = CrosstermBackend::new(stdout);
        let terminal = match Terminal::new(backend) {
            Ok(terminal) => terminal,
            Err(err) => {
                let mut stdout = io::stdout();
                let _ = execute!(stdout, LeaveAlternateScreen);
                let _ = disable_raw_mode();
                return Err(err.into());
            }
        };
        Ok(Self {
            terminal,
            restored: false,
        })
    }

    fn restore(&mut self) {
        if self.restored {
            return;
        }
        self.restored = true;
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            DisableBracketedPaste,
            LeaveAlternateScreen,
            SetCursorStyle::DefaultUserShape,
            Show
        );
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        self.restore();
    }
}

impl Deref for TerminalSession {
    type Target = Terminal<CrosstermBackend<Stdout>>;

    fn deref(&self) -> &Self::Target {
        &self.terminal
    }
}

impl DerefMut for TerminalSession {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.terminal
    }
}

pub fn run() -> anyhow::Result<()> {
    let mut session = TerminalSession::new()?;
    let result = run_app(&mut session);
    session.restore();
    result
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> anyhow::Result<()> {
    let (tx, rx) = mpsc::channel::<WorkerMsg>();
    let mut app = App::new(tx);
    app.start_sync();

    let mut last_tick = Instant::now();
    loop {
        while let Ok(msg) = rx.try_recv() {
            app.handle_worker_msg(msg);
        }

        terminal.draw(|frame| ui::render(frame, &mut app))?;
        if app.should_quit {
            break;
        }

        let timeout = TICK_RATE
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_millis(0));
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => app.handle_key(key),
                // A bracketed paste delivers the entire pasted string as one event, with no
                // synthesized keypresses in between (003-discord-paste-quick-add, research.md
                // §1) — routed to the open Discord-paste modal, if any; ignored otherwise.
                Event::Paste(text) => app.handle_paste(text),
                _ => {}
            }
        }
        if last_tick.elapsed() >= TICK_RATE {
            last_tick = Instant::now();
        }
    }

    Ok(())
}
