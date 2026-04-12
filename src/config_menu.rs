use crate::config::{self, Config, THEMES};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use ratatui::prelude::CrosstermBackend;
use ratatui::Terminal;
use std::io::Stdout;
use std::time::Duration;

const NUM_ITEMS: usize = 9;

struct ConfigMenuState {
    config: Config,
    selected: usize,
    done: bool,
    theme_index: Option<usize>,
}

impl ConfigMenuState {
    fn new(config: Config) -> Self {
        let theme_index = config::current_theme(&config);
        Self {
            config,
            selected: 0,
            done: false,
            theme_index,
        }
    }

    fn theme_name(&self) -> &'static str {
        match self.theme_index {
            Some(i) => THEMES[i].name,
            None => "custom",
        }
    }

    fn apply_theme(&mut self, idx: usize) {
        let theme = &THEMES[idx];
        self.config.colors.inhale = theme.inhale;
        self.config.colors.exhale = theme.exhale;
        self.config.colors.hold = theme.hold;
        self.theme_index = Some(idx);
    }
}

pub fn run(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    config: &Config,
) -> Result<Config, std::io::Error> {
    let mut state = ConfigMenuState::new(config.clone());

    loop {
        terminal.draw(|f| draw(f, &state))?;

        if event::poll(Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if state.selected > 0 {
                        state.selected -= 1;
                    } else {
                        state.selected = NUM_ITEMS - 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if state.selected < NUM_ITEMS - 1 {
                        state.selected += 1;
                    } else {
                        state.selected = 0;
                    }
                }
                KeyCode::Left | KeyCode::Char('h') => adjust(&mut state, -1),
                KeyCode::Right | KeyCode::Char('l') => adjust(&mut state, 1),
                KeyCode::Char('d') => {
                    state.config = Config::default();
                    state.theme_index = config::current_theme(&state.config);
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    state.done = true;
                }
                _ => {}
            }
        }

        if state.done {
            break;
        }
    }

    Ok(state.config)
}

fn adjust(state: &mut ConfigMenuState, delta: i32) {
    match state.selected {
        0 => state.config.bell = !state.config.bell,
        1 => {
            state.config.lead_in = (state.config.lead_in + delta as f64 * 0.5).clamp(0.0, 5.0);
        }
        2 => {
            let new = state.config.petals as i32 + delta;
            state.config.petals = new.clamp(3, 12) as u8;
        }
        3 => {
            let len = THEMES.len() as i32;
            let current = state.theme_index.map(|i| i as i32).unwrap_or(-1);
            let new_idx = if delta > 0 {
                if current < 0 { 0 } else { (current + 1) % len }
            } else if current <= 0 {
                len - 1
            } else {
                current - 1
            };
            state.apply_theme(new_idx as usize);
        }
        4 => adjust_rounds(&mut state.config.rounds.calm, delta),
        5 => adjust_rounds(&mut state.config.rounds.coherent, delta),
        6 => adjust_rounds(&mut state.config.rounds.sigh, delta),
        7 => adjust_rounds(&mut state.config.rounds.box_pattern, delta),
        8 => adjust_rounds(&mut state.config.rounds.energize, delta),
        _ => {}
    }
}

fn adjust_rounds(rounds: &mut u32, delta: i32) {
    let new = *rounds as i32 + delta;
    *rounds = new.clamp(1, 99) as u32;
}

fn value_str(state: &ConfigMenuState, idx: usize) -> String {
    match idx {
        0 => if state.config.bell { "on".into() } else { "off".into() },
        1 => {
            if state.config.lead_in == 0.0 {
                "off".into()
            } else {
                format!("{:.1}s", state.config.lead_in)
            }
        }
        2 => format!("{}", state.config.petals),
        3 => state.theme_name().into(),
        4 => format!("{}", state.config.rounds.calm),
        5 => format!("{}", state.config.rounds.coherent),
        6 => format!("{}", state.config.rounds.sigh),
        7 => format!("{}", state.config.rounds.box_pattern),
        8 => format!("{}", state.config.rounds.energize),
        _ => String::new(),
    }
}

fn draw(frame: &mut Frame, state: &ConfigMenuState) {
    let area = frame.area();
    let layout = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(area);
    draw_content(frame, layout[0], state);
    draw_footer(frame, layout[1]);
}

fn draw_content(frame: &mut Frame, area: Rect, state: &ConfigMenuState) {
    // 2 (title+gap) + 12 (rows including blank separator) + 2 (gap + path)
    let total_rows = 16_u16;
    let start_y = if area.height > total_rows {
        area.y + (area.height - total_rows) / 2
    } else {
        area.y
    };

    let content_width = 54_u16.min(area.width);
    let left_x = area.x + area.width.saturating_sub(content_width) / 2;

    // Title
    let title = Paragraph::new(Line::from(Span::styled(
        "settings",
        Style::default().fg(Color::Rgb(90, 140, 190)),
    )))
    .alignment(Alignment::Center);
    if start_y < area.y + area.height {
        frame.render_widget(title, Rect::new(area.x, start_y, area.width, 1));
    }

    // Rows: (y_offset from start_y, selectable_index or None for header, label)
    let rows: &[(u16, Option<usize>, &str)] = &[
        (2, None, "general"),
        (3, Some(0), "bell"),
        (4, Some(1), "lead-in"),
        (5, Some(2), "petals"),
        (6, Some(3), "theme"),
        (8, None, "default rounds"),
        (9, Some(4), "calm"),
        (10, Some(5), "coherent"),
        (11, Some(6), "sigh"),
        (12, Some(7), "box"),
        (13, Some(8), "energize"),
    ];

    let header_style = Style::default().fg(Color::Rgb(70, 70, 75));

    for &(offset, sel_idx, label) in rows {
        let y = start_y + offset;
        if y >= area.y + area.height {
            break;
        }

        if sel_idx.is_none() {
            if !label.is_empty() {
                let line = Line::from(Span::styled(format!("  {label}"), header_style));
                frame.render_widget(
                    Paragraph::new(line),
                    Rect::new(left_x, y, content_width, 1),
                );
            }
            continue;
        }

        let idx = sel_idx.unwrap();
        let is_selected = idx == state.selected;

        let cursor = if is_selected { ">" } else { " " };
        let name_style = if is_selected {
            Style::default()
                .fg(Color::Rgb(90, 140, 190))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Rgb(100, 100, 105))
        };
        let value_style = if is_selected {
            Style::default().fg(Color::Rgb(140, 140, 145))
        } else {
            Style::default().fg(Color::Rgb(60, 60, 65))
        };

        if idx == 3 {
            // Theme row with color swatches
            let c = &state.config.colors;
            let theme_name = state.theme_name();
            let line = Line::from(vec![
                Span::styled(format!("    {cursor} {:<12}", label), name_style),
                Span::styled(
                    "\u{2588}\u{2588}",
                    Style::default().fg(Color::Rgb(c.inhale[0], c.inhale[1], c.inhale[2])),
                ),
                Span::raw(" "),
                Span::styled(
                    "\u{2588}\u{2588}",
                    Style::default().fg(Color::Rgb(c.exhale[0], c.exhale[1], c.exhale[2])),
                ),
                Span::raw(" "),
                Span::styled(
                    "\u{2588}\u{2588}",
                    Style::default().fg(Color::Rgb(c.hold[0], c.hold[1], c.hold[2])),
                ),
                Span::styled(format!("  {theme_name}"), value_style),
            ]);
            frame.render_widget(
                Paragraph::new(line),
                Rect::new(left_x, y, content_width, 1),
            );
        } else {
            let val = value_str(state, idx);
            let line = Line::from(vec![
                Span::styled(format!("    {cursor} {:<12}", label), name_style),
                Span::styled(val, value_style),
            ]);
            frame.render_widget(
                Paragraph::new(line),
                Rect::new(left_x, y, content_width, 1),
            );
        }
    }

    // Config file path
    let path_y = start_y + 15;
    if path_y < area.y + area.height {
        let path = config::config_file_location();
        let line = Line::from(Span::styled(
            path,
            Style::default().fg(Color::Rgb(40, 40, 43)),
        ));
        frame.render_widget(
            Paragraph::new(line).alignment(Alignment::Center),
            Rect::new(area.x, path_y, area.width, 1),
        );
    }
}

fn draw_footer(frame: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(Span::styled(
        "\u{2191}\u{2193} select   \u{2190}\u{2192} adjust   d defaults   esc back",
        Style::default().fg(Color::Rgb(50, 50, 55)),
    )))
    .alignment(Alignment::Center);
    frame.render_widget(footer, area);
}
