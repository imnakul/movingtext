use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeSelection {
    pub top: bool,
    pub right: bool,
    pub bottom: bool,
    pub left: bool,
}

impl Default for EdgeSelection {
    fn default() -> Self {
        Self {
            top: true,
            right: false,
            bottom: true,
            left: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaddingConfig {
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
    pub left: u32,
}

impl Default for PaddingConfig {
    fn default() -> Self {
        Self {
            top: 0,
            right: 0,
            bottom: 0,
            left: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontConfig {
    pub family: String,
    pub size: f32,
    pub bold: bool,
    pub italic: bool,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: "Segoe UI".to_string(),
            size: 20.0,
            bold: true,
            italic: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorConfig {
    pub text_color: [f32; 4], // RGBA 0.0 - 1.0
    pub bg_color: [f32; 4],   // RGBA 0.0 - 1.0
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            text_color: [1.0, 0.9, 0.2, 1.0],   // Vibrant Gold / Yellow
            bg_color: [0.08, 0.08, 0.12, 0.85], // Sleek Dark Semi-Transparent
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimConfig {
    pub speed: f32, // pixels per second
    pub reverse: bool,
}

impl Default for AnimConfig {
    fn default() -> Self {
        Self {
            speed: 120.0,
            reverse: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub text: String,
    #[serde(default = "default_phrase_spacing")]
    pub phrase_spacing: u32,
    pub edges: EdgeSelection,
    pub padding: PaddingConfig,
    pub thickness: u32,
    pub font: FontConfig,
    pub colors: ColorConfig,
    pub animation: AnimConfig,
    pub click_through: bool,
    pub always_on_top: bool,
    pub monitor_index: usize,
}

fn default_phrase_spacing() -> u32 {
    6
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            text: "⭐ REMINDER: Stay focused & stay hydrated! • MovingText Desktop Marquee ⭐"
                .to_string(),
            phrase_spacing: 6,
            edges: EdgeSelection::default(),
            padding: PaddingConfig::default(),
            thickness: 36,
            font: FontConfig::default(),
            colors: ColorConfig::default(),
            animation: AnimConfig::default(),
            click_through: true,
            always_on_top: true,
            monitor_index: 0,
        }
    }
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        if let Some(mut path) = dirs::config_dir() {
            path.push("movingtext");
            let _ = fs::create_dir_all(&path);
            path.push("config.json");
            path
        } else {
            PathBuf::from("config.json")
        }
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
                    return config;
                }
            }
        }
        let default_cfg = Self::default();
        default_cfg.save();
        default_cfg
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, content);
        }
    }
}
