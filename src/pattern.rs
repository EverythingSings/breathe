/// A single phase of a breathing cycle.
#[derive(Debug, Clone)]
pub struct Phase {
    pub name: &'static str,
    pub duration_secs: f64,
    /// Visual direction: 1.0 = expanding, -1.0 = contracting, 0.0 = hold
    pub direction: f64,
}

/// A complete breathing pattern: a sequence of phases repeated for N rounds.
#[derive(Debug, Clone)]
pub struct Pattern {
    pub name: &'static str,
    pub phases: Vec<Phase>,
    pub default_rounds: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum Preset {
    /// 4:0:8:0 — extended exhale, parasympathetic activation
    Calm,
    /// 5.5:5.5 — HRV resonance frequency (~5.5 breaths/min)
    Coherent,
    /// Double inhale, long exhale — fastest single-cycle downregulator
    Sigh,
    /// 4:4:4:4 — balanced, neutral
    Box,
    /// Fast power breathing — rapid cycles, sympathetic activation
    Energize,
}

impl Preset {
    pub fn pattern(self) -> Pattern {
        match self {
            Preset::Calm => Pattern {
                name: "calm",
                phases: vec![
                    Phase { name: "inhale", duration_secs: 4.0, direction: 1.0 },
                    Phase { name: "exhale", duration_secs: 8.0, direction: -1.0 },
                ],
                default_rounds: 10,
            },
            Preset::Coherent => Pattern {
                name: "coherent",
                phases: vec![
                    Phase { name: "inhale", duration_secs: 5.5, direction: 1.0 },
                    Phase { name: "exhale", duration_secs: 5.5, direction: -1.0 },
                ],
                default_rounds: 10,
            },
            Preset::Sigh => Pattern {
                name: "sigh",
                phases: vec![
                    Phase { name: "inhale", duration_secs: 2.0, direction: 1.0 },
                    Phase { name: "sip", duration_secs: 1.0, direction: 1.0 },
                    Phase { name: "exhale", duration_secs: 6.0, direction: -1.0 },
                ],
                default_rounds: 10,
            },
            Preset::Box => Pattern {
                name: "box",
                phases: vec![
                    Phase { name: "inhale", duration_secs: 4.0, direction: 1.0 },
                    Phase { name: "hold", duration_secs: 4.0, direction: 0.0 },
                    Phase { name: "exhale", duration_secs: 4.0, direction: -1.0 },
                    Phase { name: "hold", duration_secs: 4.0, direction: 0.0 },
                ],
                default_rounds: 8,
            },
            Preset::Energize => Pattern {
                name: "energize",
                phases: vec![
                    Phase { name: "inhale", duration_secs: 1.5, direction: 1.0 },
                    Phase { name: "exhale", duration_secs: 1.0, direction: -1.0 },
                ],
                default_rounds: 30,
            },
        }
    }
}

/// Parse a custom ratio string like "4:7:8" into a Pattern.
/// Phases alternate: inhale, hold, exhale, hold (skipping zero-duration phases).
pub fn parse_custom(ratio: &str) -> Result<Pattern, String> {
    let parts: Result<Vec<f64>, _> = ratio.split(':').map(|s| s.parse::<f64>()).collect();
    let parts = parts.map_err(|_| format!("Invalid ratio: {ratio}"))?;

    let phase_names: &[(&str, f64)] = &[
        ("inhale", 1.0),
        ("hold", 0.0),
        ("exhale", -1.0),
        ("hold", 0.0),
    ];

    if parts.is_empty() || parts.len() > 4 {
        return Err("Ratio must have 1-4 parts (inhale:hold:exhale:hold)".into());
    }

    let mut phases = Vec::new();
    for (i, &dur) in parts.iter().enumerate() {
        if dur > 0.0 {
            let (name, dir) = phase_names[i];
            phases.push(Phase {
                name,
                duration_secs: dur,
                direction: dir,
            });
        }
    }

    if phases.is_empty() {
        return Err("At least one phase must have duration > 0".into());
    }

    Ok(Pattern {
        name: "custom",
        phases,
        default_rounds: 10,
    })
}
