use crate::pattern::Pattern;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use std::time::{Duration, Instant};

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn ease_in_out(t: f64) -> f64 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

fn lerp_color(a: (u8, u8, u8), b: (u8, u8, u8), t: f64) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::Rgb(
        lerp(a.0 as f64, b.0 as f64, t) as u8,
        lerp(a.1 as f64, b.1 as f64, t) as u8,
        lerp(a.2 as f64, b.2 as f64, t) as u8,
    )
}

// Base colors — these deepen as the session progresses
const COLOR_INHALE: (u8, u8, u8) = (90, 140, 190);   // cool blue — alertness, expansion
const COLOR_EXHALE: (u8, u8, u8) = (185, 145, 85);   // warm amber — release, warmth
const COLOR_HOLD: (u8, u8, u8) = (110, 140, 120);     // sage — stillness
const COLOR_DIM: (u8, u8, u8) = (50, 50, 55);
const COLOR_DONE: (u8, u8, u8) = (100, 130, 110);

// Deepened colors — richer versions reached ~halfway through a session
const COLOR_INHALE_DEEP: (u8, u8, u8) = (70, 120, 210);
const COLOR_EXHALE_DEEP: (u8, u8, u8) = (200, 135, 55);
const COLOR_HOLD_DEEP: (u8, u8, u8) = (90, 150, 110);

const LEAD_IN_SECS: f64 = 3.0;
const PETAL_COUNT: f64 = 6.0;
const PETAL_DEPTH: f64 = 0.35; // how much petals indent (0 = circle, 1 = star)

/// Cell aspect ratio compensation. Terminal cells are ~2:1 (tall:wide).
const CELL_ASPECT: f64 = 2.1;

pub struct SessionState {
    pub pattern: Pattern,
    pub rounds: u32,
    pub current_round: u32,
    pub current_phase: usize,
    pub phase_elapsed: f64,
    pub total_elapsed: f64,
    pub done: bool,
    pub paused: bool,
    pub fill_level: f64,
    prev_color: (u8, u8, u8),
    curr_color: (u8, u8, u8),
    color_blend: f64,
    pub lead_in_remaining: f64,
    closing: f64,
    closing_from: f64,
    rotation: f64,
    bell: bool,
    last_tick: Instant,
}

const CLOSING_SECS: f64 = 1.5;

impl SessionState {
    pub fn new(pattern: Pattern, rounds: u32, bell: bool) -> Self {
        let first_color = phase_color_raw(&pattern.phases[0]);
        Self {
            pattern,
            rounds,
            current_round: 0,
            current_phase: 0,
            phase_elapsed: 0.0,
            total_elapsed: 0.0,
            done: false,
            paused: false,
            fill_level: 0.0,
            prev_color: COLOR_DIM,
            curr_color: first_color,
            color_blend: 0.0,
            lead_in_remaining: LEAD_IN_SECS,
            closing: 0.0,
            closing_from: 0.0,
            rotation: 0.0,
            bell,
            last_tick: Instant::now(),
        }
    }

    pub fn tick(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f64();
        self.last_tick = now;

        // Closing animation — smooth shrink from captured start
        if self.closing > 0.0 {
            self.closing += dt;
            self.rotation += dt * 0.04; // very slow drift while closing
            let t = (self.closing / CLOSING_SECS).clamp(0.0, 1.0);
            let resting = 0.08; // small resting flower, not empty
            self.fill_level = lerp(self.closing_from, resting, ease_in_out(t));
            if self.closing >= CLOSING_SECS {
                self.done = true;
                self.fill_level = resting;
            }
            return;
        }

        if self.paused {
            self.rotation += dt * 0.015; // barely perceptible drift — app is alive
            return;
        }

        if self.done {
            self.rotation += dt * 0.03; // very gentle drift at rest
            return;
        }

        if self.lead_in_remaining > 0.0 {
            self.lead_in_remaining -= dt;
            self.color_blend = (1.0 - self.lead_in_remaining / LEAD_IN_SECS).clamp(0.0, 1.0);
            self.rotation += dt * 0.08;
            if self.lead_in_remaining <= 0.0 {
                // Lead-in just ended — bell on first inhale
                if self.bell {
                    eprint!("\x07");
                }
            } else {
                return;
            }
        }

        self.phase_elapsed += dt;
        self.total_elapsed += dt;

        // Accumulate rotation incrementally — speed varies by phase
        self.rotation += dt * self.rotation_speed();

        self.color_blend = (self.color_blend + dt * 3.0).clamp(0.0, 1.0);

        let phase = &self.pattern.phases[self.current_phase];
        let t = (self.phase_elapsed / phase.duration_secs).clamp(0.0, 1.0);
        let eased = ease_in_out(t);

        if phase.direction != 0.0 {
            let start = self.fill_at_phase_start();
            let end = self.fill_at_phase_end();
            self.fill_level = lerp(start, end, eased);
        }

        if self.phase_elapsed >= phase.duration_secs {
            self.phase_elapsed = 0.0;
            self.current_phase += 1;

            if self.current_phase >= self.pattern.phases.len() {
                self.current_phase = 0;
                self.current_round += 1;

                if self.current_round >= self.rounds {
                    self.closing_from = self.fill_level;
                    self.closing = 0.001;
                    return;
                }
            }

            self.prev_color = self.current_color_rgb();
            self.curr_color = phase_color_at_depth(&self.pattern.phases[self.current_phase], self.depth());
            self.color_blend = 0.0;

            if self.bell {
                eprint!("\x07");
            }
        }
    }

    fn current_color_rgb(&self) -> (u8, u8, u8) {
        match lerp_color(self.prev_color, self.curr_color, self.color_blend) {
            Color::Rgb(r, g, b) => (r, g, b),
            _ => COLOR_DIM,
        }
    }

    pub fn current_color(&self) -> Color {
        if self.done {
            Color::Rgb(COLOR_DONE.0, COLOR_DONE.1, COLOR_DONE.2)
        } else {
            lerp_color(self.prev_color, self.curr_color, self.color_blend)
        }
    }

    fn fill_at_phase_start(&self) -> f64 {
        let phase = &self.pattern.phases[self.current_phase];
        if phase.direction > 0.0 && self.current_phase == 0 {
            0.0
        } else {
            self.fill_level
        }
    }

    fn fill_at_phase_end(&self) -> f64 {
        let phase = &self.pattern.phases[self.current_phase];
        if phase.direction > 0.0 {
            let next = (self.current_phase + 1) % self.pattern.phases.len();
            if self.pattern.phases[next].direction > 0.0 {
                0.7
            } else {
                1.0
            }
        } else if phase.direction < 0.0 {
            0.0
        } else {
            self.fill_level
        }
    }

    pub fn in_lead_in(&self) -> bool {
        self.lead_in_remaining > 0.0
    }

    pub fn is_closing(&self) -> bool {
        self.closing > 0.0 && !self.done
    }

    /// How deep into the session we are (0.0 → 1.0). Ramps up over first half.
    fn depth(&self) -> f64 {
        if self.rounds == 0 { return 0.0; }
        let progress = self.current_round as f64 / self.rounds as f64;
        // Ease-in: deepens quickly at first, plateaus in second half
        (progress * 2.0).min(1.0).sqrt()
    }

    /// Rotation speed that breathes with you.
    fn rotation_speed(&self) -> f64 {
        if self.done || self.in_lead_in() {
            return 0.08;
        }
        let phase = &self.pattern.phases[self.current_phase];
        if phase.direction > 0.0 {
            0.12 // slightly faster on inhale
        } else if phase.direction < 0.0 {
            0.06 // slower on exhale
        } else {
            0.03 // near-still on hold
        }
    }
}

// ── Flower rendering ──────────────────────────────────────────────

/// Compute the flower's radius at a given angle.
/// Creates a polar rose with `PETAL_COUNT` petals.
/// Organic variation makes each petal slightly different and shift over time.
fn flower_radius(angle: f64, base_radius: f64, time: f64) -> f64 {
    let petal = (angle * PETAL_COUNT).cos().abs();
    let petal = petal.powf(0.6);
    // Slow wobble: petals breathe slightly out of sync with each other
    let variation = (angle * 1.3 + time * 0.07).sin() * 0.04;
    base_radius * (1.0 - PETAL_DEPTH + (PETAL_DEPTH + variation) * petal)
}

/// Two-level shading: solid core, soft edge. Color does the gradient work.
fn shade_char(normalized_dist: f64) -> char {
    if normalized_dist < 0.7 { '█' } else { '░' }
}

/// Color intensity based on distance from center.
/// The curve is shaped so the █→░ transition at 0.7 isn't a visible ring:
/// brightness drops faster in the core, then levels off near the boundary.
fn shade_color(base: (u8, u8, u8), normalized_dist: f64) -> Color {
    let brightness = lerp(1.0, 0.2, normalized_dist.powf(1.4));
    Color::Rgb(
        (base.0 as f64 * brightness) as u8,
        (base.1 as f64 * brightness) as u8,
        (base.2 as f64 * brightness) as u8,
    )
}

/// Render the flower into a character grid.
fn render_flower(
    width: u16,
    height: u16,
    fill_level: f64,
    color_rgb: (u8, u8, u8),
    rotation: f64,
    time: f64,
) -> Vec<Vec<(char, Color)>> {
    let mut grid: Vec<Vec<(char, Color)>> = vec![vec![(' ', Color::Reset); width as usize]; height as usize];

    let cx = width as f64 / 2.0;
    let cy = height as f64 / 2.0;

    // Max radius in cell units — use the smaller dimension
    let max_radius_cells = (width as f64 / 2.0).min(height as f64 * CELL_ASPECT / 2.0) * 0.85;

    // Minimum visible radius (a seed point)
    let min_radius = 1.5;
    let base_radius = lerp(min_radius, max_radius_cells, fill_level);

    for (row_idx, row) in grid.iter_mut().enumerate() {
        for (col_idx, cell) in row.iter_mut().enumerate() {
            let dx = col_idx as f64 - cx;
            let dy = (row_idx as f64 - cy) * CELL_ASPECT;

            let dist = (dx * dx + dy * dy).sqrt();
            let angle = dy.atan2(dx) + rotation;

            let r = flower_radius(angle, base_radius, time);

            if dist < r {
                let normalized = dist / r;
                *cell = (shade_char(normalized), shade_color(color_rgb, normalized));
            }
        }
    }

    grid
}

// ── Drawing ───────────────────────────────────────────────────────

pub fn draw(frame: &mut Frame, state: &SessionState) {
    let area = frame.area();

    let layout = Layout::vertical([
        Constraint::Min(4),
        Constraint::Length(1),
    ])
    .split(area);

    draw_main(frame, layout[0], state);
    draw_footer(frame, layout[1], state);
}

fn draw_main(frame: &mut Frame, area: Rect, state: &SessionState) {
    if area.height < 3 || area.width < 6 {
        return;
    }

    let color = state.current_color();

    // Reserve space for phase label below flower
    let label_rows = 2_u16;
    let flower_h = area.height.saturating_sub(label_rows);
    let flower_w = area.width;

    if flower_h < 3 {
        return;
    }

    let color_rgb = if state.done || state.is_closing() {
        COLOR_DONE
    } else {
        state.current_color_rgb()
    };

    // During lead-in, seed pulses at ~1Hz to prime the body's rhythm
    let fill = if state.in_lead_in() {
        let pulse_t = (state.lead_in_remaining.fract() * std::f64::consts::TAU).sin();
        lerp(0.02, 0.06, (pulse_t + 1.0) / 2.0)
    } else {
        state.fill_level
    };

    let grid = render_flower(flower_w, flower_h, fill, color_rgb, state.rotation, state.total_elapsed);

    let lines: Vec<Line> = grid
        .iter()
        .map(|row| {
            Line::from(
                row.iter()
                    .map(|&(ch, col)| Span::styled(ch.to_string(), Style::default().fg(col)))
                    .collect::<Vec<_>>(),
            )
        })
        .collect();

    frame.render_widget(
        Paragraph::new(lines),
        Rect::new(area.x, area.y, flower_w, flower_h),
    );

    // Phase label
    let label_y = area.y + flower_h;
    if label_y < area.y + area.height {
        let label_text = if state.done || state.is_closing() {
            String::new()
        } else if state.paused {
            "paused".to_string()
        } else if state.in_lead_in() {
            // Dim dot instead of number — the pulse IS the countdown
            "·".to_string()
        } else {
            state.pattern.phases[state.current_phase]
                .name
                .to_lowercase()
        };

        let label = Paragraph::new(Line::from(Span::styled(
            label_text,
            Style::default().fg(color),
        )))
        .alignment(Alignment::Center);

        frame.render_widget(label, Rect::new(area.x, label_y, area.width, 1));
    }
}

fn draw_footer(frame: &mut Frame, area: Rect, state: &SessionState) {
    if state.in_lead_in() || state.is_closing() {
        return;
    }

    let color = state.current_color();
    let dim = Color::Rgb(COLOR_DIM.0, COLOR_DIM.1, COLOR_DIM.2);
    let elapsed = format_duration(state.total_elapsed);

    if state.done {
        let hint = Color::Rgb(60, 60, 65);
        let sep = Color::Rgb(40, 40, 43);
        let footer = Line::from(vec![
            Span::styled(format!(" {elapsed} "), Style::default().fg(dim)),
            Span::raw("   "),
            Span::styled("↵ again", Style::default().fg(hint)),
            Span::styled("  ·  ", Style::default().fg(sep)),
            Span::styled("q quit", Style::default().fg(hint)),
        ]);
        frame.render_widget(Paragraph::new(footer).alignment(Alignment::Center), area);
        return;
    }

    let round_display = format!(
        "{}/{}",
        (state.current_round + 1).min(state.rounds),
        state.rounds
    );

    let phase = &state.pattern.phases[state.current_phase];
    let remaining = (phase.duration_secs - state.phase_elapsed).max(0.0);
    let countdown = remaining.ceil().max(1.0) as u32; // never flash "0"

    let status = format!("{countdown}");

    let bell_indicator = if state.bell { " ♪" } else { "" };

    let footer = Line::from(vec![
        Span::styled(format!(" {status} "), Style::default().fg(color)),
        Span::raw(" "),
        Span::styled(round_display, Style::default().fg(dim)),
        Span::raw(" "),
        Span::styled(elapsed, Style::default().fg(dim)),
        Span::styled(bell_indicator, Style::default().fg(dim)),
    ]);

    frame.render_widget(Paragraph::new(footer).alignment(Alignment::Center), area);
}

fn phase_color_at_depth(phase: &crate::pattern::Phase, depth: f64) -> (u8, u8, u8) {
    let (base, deep) = if phase.direction > 0.0 {
        (COLOR_INHALE, COLOR_INHALE_DEEP)
    } else if phase.direction < 0.0 {
        (COLOR_EXHALE, COLOR_EXHALE_DEEP)
    } else {
        (COLOR_HOLD, COLOR_HOLD_DEEP)
    };
    (
        lerp(base.0 as f64, deep.0 as f64, depth) as u8,
        lerp(base.1 as f64, deep.1 as f64, depth) as u8,
        lerp(base.2 as f64, deep.2 as f64, depth) as u8,
    )
}

fn phase_color_raw(phase: &crate::pattern::Phase) -> (u8, u8, u8) {
    phase_color_at_depth(phase, 0.0)
}

fn format_duration(secs: f64) -> String {
    let m = (secs / 60.0) as u64;
    let s = (secs % 60.0) as u64;
    format!("{m}:{s:02}")
}

pub enum SessionAction {
    Continue,
    Quit,
    Menu,
}

pub fn handle_input(state: &mut SessionState) -> Result<SessionAction, std::io::Error> {
    if event::poll(Duration::from_millis(16))?
        && let Event::Key(key) = event::read()?
        && key.kind == KeyEventKind::Press
    {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(SessionAction::Quit),
            KeyCode::Char(' ') if !state.done && !state.is_closing() => {
                state.paused = !state.paused;
            }
            KeyCode::Char('b') if !state.done => {
                state.bell = !state.bell;
            }
            KeyCode::Enter if state.done => return Ok(SessionAction::Menu),
            _ => {}
        }
    }
    Ok(SessionAction::Continue)
}
