use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    error::Error,
    io::stdout,
    panic,
    time::{Duration, Instant},
};
use tomatui::{
    app::App,
    cli::{execute_cli, normalize_shorthand_args, Cli, CliDispatch},
    db::Database,
    ui,
};

struct TerminalGuard;

impl TerminalGuard {
    fn acquire() -> Result<Self, Box<dyn Error>> {
        enable_raw_mode()?;
        execute!(
            stdout(),
            EnterAlternateScreen,
            EnableMouseCapture,
            crossterm::cursor::Hide
        )?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            stdout(),
            DisableMouseCapture,
            LeaveAlternateScreen,
            crossterm::cursor::Show
        );
    }
}

fn setup_panic_hook() {
    let orig = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(
            stdout(),
            DisableMouseCapture,
            LeaveAlternateScreen,
            crossterm::cursor::Show
        );
        orig(info);
    }));
}

fn main() -> Result<(), Box<dyn Error>> {
    let normalized = normalize_shorthand_args(std::env::args());
    let cli = Cli::parse_from(normalized);

    let db = match &cli.db {
        Some(path) => Database::open(path)?,
        None => Database::open_default()?,
    };

    let launch = match execute_cli(cli, &db) {
        Ok(CliDispatch::Handled) => return Ok(()),
        Ok(CliDispatch::LaunchTui(opts)) => opts,
        Err(msg) => {
            eprintln!("error: {}", msg);
            std::process::exit(1);
        }
    };

    setup_panic_hook();
    let _guard = TerminalGuard::acquire()?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::new(db, launch);

    run_loop(&mut terminal, &mut app)?;
    Ok(())
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> Result<(), Box<dyn Error>> {
    const TICK_RATE: Duration = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui::render(f, app))?;

        let timeout = TICK_RATE
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key)
                    if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
                {
                    app.on_key(key);
                }
                Event::Mouse(mouse) => app.on_mouse(mouse),
                _ => {}
            }
        }

        let elapsed = last_tick.elapsed();
        if elapsed >= TICK_RATE {
            let missed = (elapsed.as_millis() / TICK_RATE.as_millis()).max(1);
            let advance = missed.min(u32::MAX as u128) as u32;
            let fire = missed.min(10);
            last_tick += TICK_RATE * advance;
            for _ in 0..fire {
                app.on_tick();
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}
