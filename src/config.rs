use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub bell: bool,
    pub lead_in: f64,
    pub petals: u8,
    pub rounds: RoundsConfig,
    pub colors: ColorConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RoundsConfig {
    pub calm: u32,
    pub coherent: u32,
    pub sigh: u32,
    #[serde(rename = "box")]
    pub box_pattern: u32,
    pub energize: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ColorConfig {
    pub inhale: [u8; 3],
    pub exhale: [u8; 3],
    pub hold: [u8; 3],
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bell: false,
            lead_in: 3.0,
            petals: 6,
            rounds: RoundsConfig::default(),
            colors: ColorConfig::default(),
        }
    }
}

impl Default for RoundsConfig {
    fn default() -> Self {
        Self {
            calm: 10,
            coherent: 10,
            sigh: 10,
            box_pattern: 8,
            energize: 30,
        }
    }
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            inhale: [90, 140, 190],
            exhale: [185, 145, 85],
            hold: [110, 140, 120],
        }
    }
}

impl RoundsConfig {
    pub fn for_preset(&self, preset: crate::pattern::Preset) -> u32 {
        match preset {
            crate::pattern::Preset::Calm => self.calm,
            crate::pattern::Preset::Coherent => self.coherent,
            crate::pattern::Preset::Sigh => self.sigh,
            crate::pattern::Preset::Box => self.box_pattern,
            crate::pattern::Preset::Energize => self.energize,
        }
    }
}

pub struct Theme {
    pub name: &'static str,
    pub inhale: [u8; 3],
    pub exhale: [u8; 3],
    pub hold: [u8; 3],
}

pub const THEMES: &[Theme] = &[
    Theme { name: "default", inhale: [90, 140, 190], exhale: [185, 145, 85], hold: [110, 140, 120] },
    Theme { name: "ocean", inhale: [70, 160, 180], exhale: [200, 120, 100], hold: [90, 150, 140] },
    Theme { name: "night", inhale: [90, 90, 200], exhale: [160, 90, 170], hold: [70, 110, 140] },
    Theme { name: "earth", inhale: [140, 120, 80], exhale: [180, 150, 60], hold: [100, 120, 80] },
    Theme { name: "mono", inhale: [180, 180, 185], exhale: [120, 120, 125], hold: [90, 90, 95] },
    Theme { name: "ember", inhale: [200, 100, 60], exhale: [200, 150, 50], hold: [140, 100, 80] },
    Theme { name: "forest", inhale: [80, 160, 100], exhale: [190, 170, 70], hold: [100, 130, 80] },
];

pub fn current_theme(config: &Config) -> Option<usize> {
    THEMES.iter().position(|t| {
        t.inhale == config.colors.inhale
            && t.exhale == config.colors.exhale
            && t.hold == config.colors.hold
    })
}

fn config_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("breathe").join("config.toml"))
}

pub fn load() -> Config {
    let Some(path) = config_path() else {
        return Config::default();
    };
    match fs::read_to_string(&path) {
        Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
        Err(_) => Config::default(),
    }
}

pub fn save(config: &Config) -> Result<(), String> {
    let path = config_path().ok_or("Could not determine data directory")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let contents = toml::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(&path, contents).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn config_file_location() -> String {
    config_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "unknown".into())
}
