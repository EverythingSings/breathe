use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, Write};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionLog {
    pub timestamp: DateTime<Utc>,
    pub pattern: String,
    pub rounds_completed: u32,
    pub rounds_target: u32,
    pub total_seconds: f64,
    pub completed: bool,
}

fn log_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("breathe").join("sessions.jsonl"))
}

pub fn save(log: &SessionLog) -> Result<(), String> {
    let path = log_path().ok_or("Could not determine data directory")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let line = serde_json::to_string(log).map_err(|e| e.to_string())?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    writeln!(file, "{line}").map_err(|e| e.to_string())?;
    Ok(())
}

pub fn log_file_location() -> String {
    log_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "unknown".into())
}

/// Read and display recent sessions.
pub fn show_recent(count: usize) {
    let path = match log_path() {
        Some(p) if p.exists() => p,
        _ => {
            println!("No sessions yet.");
            return;
        }
    };

    let file = match fs::File::open(&path) {
        Ok(f) => f,
        Err(_) => {
            println!("No sessions yet.");
            return;
        }
    };

    let lines: Vec<String> = std::io::BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .collect();

    if lines.is_empty() {
        println!("No sessions yet.");
        return;
    }

    let recent = &lines[lines.len().saturating_sub(count)..];

    println!(
        "  {:<17} {:<10} {:>11}  {:>5}",
        "date", "pattern", "rounds", "time"
    );

    for line in recent {
        if let Ok(log) = serde_json::from_str::<SessionLog>(line) {
            let status = if log.completed { " " } else { "*" };
            let mins = log.total_seconds / 60.0;
            let local = log.timestamp.with_timezone(&chrono::Local);
            let date = local.format("%Y-%m-%d %H:%M");
            println!(
                "{status} {date}  {:<10} {}/{} rounds  {:.1}m",
                log.pattern, log.rounds_completed, log.rounds_target, mins
            );
        }
    }

    let has_interrupted = recent.iter().any(|l| {
        serde_json::from_str::<SessionLog>(l)
            .map(|log| !log.completed)
            .unwrap_or(false)
    });
    if has_interrupted {
        println!("\n* interrupted");
    }
    println!("{}", log_file_location());
}
