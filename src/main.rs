mod config;
mod config_menu;
mod menu;
mod pattern;
mod session;
mod ui;

use std::io::IsTerminal;

use chrono::Utc;
use clap::{Parser, Subcommand};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use pattern::Preset;
use ratatui::prelude::CrosstermBackend;
use ratatui::Terminal;
use std::io::stdout;

#[derive(Parser)]
#[command(name = "breathe", version, about = "Breathing pacer for the terminal")]
struct Cli {
    /// Ring terminal bell on each phase transition (for eyes-closed use)
    #[arg(long, global = true)]
    bell: bool,

    /// Suppress terminal bell even if configured on
    #[arg(long, global = true, conflicts_with = "bell")]
    no_bell: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// 4:0:8:0 — extended exhale, parasympathetic activation
    Calm {
        #[arg(short, long)]
        rounds: Option<u32>,
    },
    /// 5.5:5.5 — HRV resonance frequency
    Coherent {
        #[arg(short, long)]
        rounds: Option<u32>,
    },
    /// Double inhale, long exhale — fastest single-cycle downregulator
    Sigh {
        #[arg(short, long)]
        rounds: Option<u32>,
    },
    /// 4:4:4:4 — balanced, neutral
    Box {
        #[arg(short, long)]
        rounds: Option<u32>,
    },
    /// Fast power breathing — 30 rapid cycles, sympathetic activation
    Energize {
        #[arg(short, long)]
        rounds: Option<u32>,
    },
    /// Custom ratio (e.g., "4:7:8" for inhale:hold:exhale)
    Custom {
        /// Phase durations in seconds (inhale:hold:exhale:hold)
        ratio: String,
        #[arg(short, long)]
        rounds: Option<u32>,
    },
    /// Show recent sessions
    Log {
        /// Number of sessions to show (default 10)
        #[arg(short = 'n', long, default_value = "10")]
        last: usize,
    },
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(completed) => {
            if completed { std::process::ExitCode::from(0) }
            else { std::process::ExitCode::from(1) }
        }
        Err(e) => {
            eprintln!("breathe: {e}");
            std::process::ExitCode::from(2)
        }
    }
}

fn run() -> Result<bool, Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let config = config::load();
    let cli_bell = cli.bell;
    let cli_no_bell = cli.no_bell;

    if let Some(Command::Log { last }) = &cli.command {
        session::show_recent(*last);
        return Ok(true);
    }

    // Terminal setup (shared by menu and session)
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;

    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stderr(), LeaveAlternateScreen);
        original_hook(info);
    }));

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Direct subcommand: run once and exit
    if let Some(cmd) = cli.command {
        let bell = if cli_bell { true } else if cli_no_bell { false } else { config.bell };
        let (pat, rounds_override, preset) = resolve_command(cmd)?;
        let rounds = rounds_override
            .or_else(|| preset.map(|p| config.rounds.for_preset(p)))
            .unwrap_or(pat.default_rounds);
        let (log, _exit) = run_session(&mut terminal, pat, rounds, bell, &config)?;

        disable_raw_mode()?;
        execute!(stdout(), LeaveAlternateScreen)?;

        if log.rounds_target == 0 {
            println!(
                "{}  {} rounds  {:.0}s",
                log.pattern, log.rounds_completed, log.total_seconds
            );
        } else if log.completed {
            println!(
                "{}  {}  {:.0}s",
                log.pattern, log.rounds_completed, log.total_seconds
            );
        } else {
            println!(
                "{}  {}/{}  {:.0}s (interrupted)",
                log.pattern, log.rounds_completed, log.rounds_target, log.total_seconds
            );
        }

        // JSON when piped
        if !std::io::stdout().is_terminal() {
            println!("{}", serde_json::to_string(&log)?);
        }

        return Ok(log.completed);
    }

    // Interactive menu: loop until quit
    let mut config = config;
    let mut bell = if cli_bell { true } else if cli_no_bell { false } else { config.bell };
    let mut menu_state = menu::MenuState::new(&config);

    loop {
        // Show menu
        loop {
            terminal.draw(|f| menu::draw(f, &menu_state))?;
            menu::handle_input(&mut menu_state)?;

            if menu_state.quit {
                disable_raw_mode()?;
                execute!(stdout(), LeaveAlternateScreen)?;
                return Ok(true);
            }

            if menu_state.open_config {
                terminal.clear()?;
                config = config_menu::run(&mut terminal, &config)?;
                if let Err(e) = config::save(&config) {
                    eprintln!("Warning: could not save config: {e}");
                }
                bell = if cli_bell { true } else if cli_no_bell { false } else { config.bell };
                let selected = menu_state.selected;
                menu_state.reset_from_config(&config);
                menu_state.selected = selected;
                terminal.clear()?;
                continue;
            }

            if menu_state.chosen.is_some() {
                break;
            }
        }

        let (preset, rounds) = menu_state.chosen.unwrap();
        let pat = preset.pattern();

        terminal.clear()?;
        let (_log, exit) = run_session(&mut terminal, pat, rounds, bell, &config)?;

        match exit {
            SessionExit::Quit => {
                disable_raw_mode()?;
                execute!(stdout(), LeaveAlternateScreen)?;
                return Ok(true);
            }
            SessionExit::Menu => {
                menu_state.reset();
                terminal.clear()?;
            }
        }
    }
}

pub enum SessionExit {
    Quit,
    Menu,
}

fn run_session(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    pat: pattern::Pattern,
    rounds: u32,
    bell: bool,
    config: &config::Config,
) -> Result<(session::SessionLog, SessionExit), Box<dyn std::error::Error>> {
    let pattern_name = pat.name.to_string();
    let mut state = ui::SessionState::new(pat, rounds, bell, config);

    let start = std::time::Instant::now();

    let exit = loop {
        state.tick();
        terminal.draw(|f| ui::draw(f, &state))?;

        match ui::handle_input(&mut state)? {
            ui::SessionAction::Quit => break SessionExit::Quit,
            ui::SessionAction::Menu => break SessionExit::Menu,
            ui::SessionAction::Continue => {}
        }
    };

    let elapsed = start.elapsed().as_secs_f64();

    let log = session::SessionLog {
        timestamp: Utc::now(),
        pattern: pattern_name,
        rounds_completed: state.current_round,
        rounds_target: state.rounds,
        total_seconds: elapsed,
        completed: state.done,
    };

    if let Err(e) = session::save(&log) {
        eprintln!("Warning: could not save session log: {e}");
    }

    Ok((log, exit))
}

type ResolvedCommand = (pattern::Pattern, Option<u32>, Option<Preset>);

fn resolve_command(cmd: Command) -> Result<ResolvedCommand, Box<dyn std::error::Error>> {
    Ok(match cmd {
        Command::Calm { rounds } => (Preset::Calm.pattern(), rounds, Some(Preset::Calm)),
        Command::Coherent { rounds } => (Preset::Coherent.pattern(), rounds, Some(Preset::Coherent)),
        Command::Sigh { rounds } => (Preset::Sigh.pattern(), rounds, Some(Preset::Sigh)),
        Command::Box { rounds } => (Preset::Box.pattern(), rounds, Some(Preset::Box)),
        Command::Energize { rounds } => (Preset::Energize.pattern(), rounds, Some(Preset::Energize)),
        Command::Custom { ratio, rounds } => {
            let p = pattern::parse_custom(&ratio)?;
            (p, rounds, None)
        }
        Command::Log { .. } => unreachable!(),
    })
}
