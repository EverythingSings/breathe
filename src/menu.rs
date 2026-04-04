use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use std::time::Duration;

use crate::pattern::Preset;

struct MenuItem {
    preset: Preset,
    name: &'static str,
    ratio: &'static str,
    desc: &'static str,
    default_rounds: u32,
    /// Total seconds per cycle (sum of all phase durations)
    cycle_secs: f64,
}

const ITEMS: &[MenuItem] = &[
    MenuItem { preset: Preset::Calm, name: "calm", ratio: "4:8", desc: "parasympathetic activation", default_rounds: 10, cycle_secs: 12.0 },
    MenuItem { preset: Preset::Coherent, name: "coherent", ratio: "5.5:5.5", desc: "HRV resonance", default_rounds: 10, cycle_secs: 11.0 },
    MenuItem { preset: Preset::Sigh, name: "sigh", ratio: "2:1:6", desc: "physiological sigh", default_rounds: 10, cycle_secs: 9.0 },
    MenuItem { preset: Preset::Box, name: "box", ratio: "4:4:4:4", desc: "balance and focus", default_rounds: 8, cycle_secs: 16.0 },
    MenuItem { preset: Preset::Energize, name: "energize", ratio: "1.5:1", desc: "rapid sympathetic activation", default_rounds: 30, cycle_secs: 2.5 },
];

pub struct MenuState {
    pub selected: usize,
    pub chosen: Option<(Preset, u32)>,
    pub quit: bool,
    rounds: [u32; 5],
}

impl MenuState {
    pub fn new() -> Self {
        let rounds = std::array::from_fn(|i| ITEMS[i].default_rounds);
        Self {
            selected: 0,
            chosen: None,
            quit: false,
            rounds,
        }
    }

    pub fn reset(&mut self) {
        self.chosen = None;
    }

    fn selected_rounds(&self) -> u32 {
        self.rounds[self.selected]
    }
}

pub fn draw(frame: &mut Frame, state: &MenuState) {
    let area = frame.area();

    let layout = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(area);

    draw_menu(frame, layout[0], state);
    draw_footer(frame, layout[1]);
}

fn draw_menu(frame: &mut Frame, area: Rect, state: &MenuState) {
    let menu_height = ITEMS.len() as u16 + 4;
    let start_y = if area.height > menu_height {
        area.y + (area.height - menu_height) / 2
    } else {
        area.y
    };

    // Title
    let title = Paragraph::new(Line::from(Span::styled(
        "breathe",
        Style::default().fg(Color::Rgb(90, 140, 190)),
    )))
    .alignment(Alignment::Center);

    if start_y < area.y + area.height {
        frame.render_widget(title, Rect::new(area.x, start_y, area.width, 1));
    }

    // Menu items
    let items_start = start_y + 2;
    for (i, item) in ITEMS.iter().enumerate() {
        let y = items_start + i as u16;
        if y >= area.y + area.height {
            break;
        }

        let is_selected = i == state.selected;
        let rounds = state.rounds[i];
        let is_default = rounds == item.default_rounds;

        let num = format!("{}", i + 1);
        let cursor = if is_selected { ">" } else { " " };
        let num_style = Style::default().fg(Color::Rgb(45, 45, 50));
        let name_style = if is_selected {
            Style::default()
                .fg(Color::Rgb(90, 140, 190))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Rgb(100, 100, 105))
        };
        let ratio_style = if is_selected {
            Style::default().fg(Color::Rgb(140, 140, 145))
        } else {
            Style::default().fg(Color::Rgb(60, 60, 65))
        };
        let desc_style = if is_selected {
            Style::default().fg(Color::Rgb(110, 110, 115))
        } else {
            Style::default().fg(Color::Rgb(50, 50, 55))
        };

        // Rounds + estimated duration
        let total_secs = rounds as f64 * item.cycle_secs;
        let duration_str = if total_secs < 60.0 {
            format!("~{:.0}s", total_secs)
        } else {
            let mins = total_secs / 60.0;
            if mins < 10.0 {
                format!("~{:.1}m", mins)
            } else {
                format!("~{:.0}m", mins)
            }
        };

        let rounds_style = if is_selected && !is_default {
            Style::default().fg(Color::Rgb(90, 140, 190))
        } else if is_selected {
            Style::default().fg(Color::Rgb(70, 70, 75))
        } else {
            Style::default().fg(Color::Rgb(45, 45, 50))
        };

        let line = Line::from(vec![
            Span::styled(format!(" {num}"), num_style),
            Span::styled(format!(" {cursor} "), name_style),
            Span::styled(format!("{:<12}", item.name), name_style),
            Span::styled(format!("{:<10}", item.ratio), ratio_style),
            Span::styled(format!("{:<6}", format!("{}r", rounds)), rounds_style),
            Span::styled(format!("{:<7}", duration_str), rounds_style),
            Span::styled(item.desc, desc_style),
        ]);

        frame.render_widget(
            Paragraph::new(line).alignment(Alignment::Left),
            Rect::new(
                area.x + area.width.saturating_sub(74) / 2,
                y,
                74.min(area.width),
                1,
            ),
        );
    }
}

fn draw_footer(frame: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(Span::styled(
        "1-5 or ↑↓ enter   ←→ rounds   q quit",
        Style::default().fg(Color::Rgb(50, 50, 55)),
    )))
    .alignment(Alignment::Center);

    frame.render_widget(footer, area);
}

pub fn handle_input(state: &mut MenuState) -> Result<(), std::io::Error> {
    if event::poll(Duration::from_millis(50))?
        && let Event::Key(key) = event::read()?
        && key.kind == KeyEventKind::Press
    {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if state.selected > 0 {
                    state.selected -= 1;
                } else {
                    state.selected = ITEMS.len() - 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if state.selected < ITEMS.len() - 1 {
                    state.selected += 1;
                } else {
                    state.selected = 0;
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                let r = &mut state.rounds[state.selected];
                if *r > 1 {
                    *r -= 1;
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                let r = &mut state.rounds[state.selected];
                if *r < 99 {
                    *r += 1;
                }
            }
            KeyCode::Enter => {
                let rounds = state.selected_rounds();
                state.chosen = Some((ITEMS[state.selected].preset, rounds));
            }
            KeyCode::Char(c @ '1'..='5') => {
                let idx = (c as usize) - ('1' as usize);
                if idx < ITEMS.len() {
                    state.selected = idx;
                    let rounds = state.rounds[idx];
                    state.chosen = Some((ITEMS[idx].preset, rounds));
                }
            }
            KeyCode::Char('q') | KeyCode::Esc => {
                state.quit = true;
            }
            _ => {}
        }
    }
    Ok(())
}
