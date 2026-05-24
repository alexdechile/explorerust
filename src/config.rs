use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub terminal: TerminalConfig,
    pub editor: EditorConfig,
    pub gui: GuiConfig,
    pub bookmarks: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalConfig {
    /// "auto" detects installed terminals, or specify: kitty, alacritty,
    /// gnome-terminal, konsole, xterm, tilix, wezterm, foot, custom
    pub preferred: String,
    /// Used when preferred = "custom"
    pub custom_command: String,
    /// {path} is replaced with the directory
    pub custom_args: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    pub syntax_highlight: bool,
    pub tab_size: usize,
    pub word_wrap: bool,
    pub show_line_numbers: bool,
    pub font_size: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiConfig {
    pub show_hidden: bool,
    pub confirm_delete: bool,
    pub sidebar_width: f32,
    pub sort_column: String,
    pub sort_ascending: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            terminal: TerminalConfig {
                preferred: "auto".into(),
                custom_command: String::new(),
                custom_args: "--working-directory {path}".into(),
            },
            editor: EditorConfig {
                syntax_highlight: true,
                tab_size: 4,
                word_wrap: true,
                show_line_numbers: true,
                font_size: 14.0,
            },
            gui: GuiConfig {
                show_hidden: false,
                confirm_delete: true,
                sidebar_width: 200.0,
                sort_column: "name".into(),
                sort_ascending: true,
            },
            bookmarks: vec![
                dirs::home_dir().unwrap_or_default(),
                dirs::document_dir().unwrap_or_default(),
                dirs::download_dir().unwrap_or_default(),
                dirs::picture_dir().unwrap_or_default(),
            ]
            .into_iter()
            .filter(|p| !p.as_os_str().is_empty())
            .collect(),
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("explorerust")
            .join("config.toml")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(s) => toml::from_str(&s).unwrap_or_default(),
                Err(_) => Self::default(),
            }
        } else {
            let cfg = Self::default();
            let _ = cfg.save();
            cfg
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let s = toml::to_string_pretty(self)?;
        std::fs::write(path, s)?;
        Ok(())
    }
}
