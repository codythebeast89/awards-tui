//! User config and theme (TOML).

use ratatui::style::Color;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Theme {
    pub bg: Color,
    pub panel: Color,
    pub panel_alt: Color,
    pub purple: Color,
    pub purple_dark: Color,
    pub border: Color,
    pub text: Color,
    pub muted: Color,
    pub dup: Color,
    pub input_bg: Color,
    pub highlight_bg: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: Color::Rgb(12, 12, 15),
            panel: Color::Rgb(16, 16, 24),
            panel_alt: Color::Rgb(18, 18, 26),
            purple: Color::Rgb(167, 139, 250),
            purple_dark: Color::Rgb(124, 58, 237),
            border: Color::Rgb(59, 51, 88),
            text: Color::Rgb(245, 243, 255),
            muted: Color::Rgb(156, 163, 175),
            dup: Color::Rgb(248, 113, 113),
            input_bg: Color::Rgb(30, 27, 75),
            highlight_bg: Color::Rgb(49, 46, 129),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct FileConfig {
    #[serde(default)]
    theme: ThemeFile,
}

#[derive(Debug, Default, Deserialize)]
struct ThemeFile {
    bg: Option<String>,
    panel: Option<String>,
    panel_alt: Option<String>,
    purple: Option<String>,
    purple_dark: Option<String>,
    border: Option<String>,
    text: Option<String>,
    muted: Option<String>,
    dup: Option<String>,
    input_bg: Option<String>,
    highlight_bg: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AppConfig {
    pub theme: Theme,
    pub loaded_from: Option<PathBuf>,
}

impl AppConfig {
    pub fn load() -> Self {
        for path in config_candidates() {
            if let Ok(text) = std::fs::read_to_string(&path) {
                match parse_config(&text) {
                    Ok(theme) => {
                        return Self {
                            theme,
                            loaded_from: Some(path),
                        };
                    }
                    Err(err) => {
                        eprintln!("awards-tui: ignoring config {}: {err}", path.display());
                    }
                }
            }
        }
        Self::default()
    }
}

fn config_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(path) = std::env::var("AWARDS_TUI_CONFIG") {
        out.push(PathBuf::from(path));
    }
    if let Ok(cwd) = std::env::current_dir() {
        out.push(cwd.join("awards-tui.toml"));
    }
    if let Some(config_dir) = dirs::config_dir() {
        out.push(config_dir.join("awards-tui").join("config.toml"));
    }
    out
}

fn parse_config(text: &str) -> Result<Theme, String> {
    let file: FileConfig = toml::from_str(text).map_err(|e| e.to_string())?;
    let mut theme = Theme::default();
    apply_hex(&mut theme.bg, file.theme.bg.as_deref())?;
    apply_hex(&mut theme.panel, file.theme.panel.as_deref())?;
    apply_hex(&mut theme.panel_alt, file.theme.panel_alt.as_deref())?;
    apply_hex(&mut theme.purple, file.theme.purple.as_deref())?;
    apply_hex(&mut theme.purple_dark, file.theme.purple_dark.as_deref())?;
    apply_hex(&mut theme.border, file.theme.border.as_deref())?;
    apply_hex(&mut theme.text, file.theme.text.as_deref())?;
    apply_hex(&mut theme.muted, file.theme.muted.as_deref())?;
    apply_hex(&mut theme.dup, file.theme.dup.as_deref())?;
    apply_hex(&mut theme.input_bg, file.theme.input_bg.as_deref())?;
    apply_hex(&mut theme.highlight_bg, file.theme.highlight_bg.as_deref())?;
    Ok(theme)
}

fn apply_hex(slot: &mut Color, value: Option<&str>) -> Result<(), String> {
    let Some(value) = value else {
        return Ok(());
    };
    *slot = parse_hex_color(value)?;
    Ok(())
}

fn parse_hex_color(raw: &str) -> Result<Color, String> {
    let s = raw.trim().trim_start_matches('#');
    if s.len() != 6 {
        return Err(format!("expected #RRGGBB, got {raw:?}"));
    }
    let r = u8::from_str_radix(&s[0..2], 16).map_err(|e| e.to_string())?;
    let g = u8::from_str_radix(&s[2..4], 16).map_err(|e| e.to_string())?;
    let b = u8::from_str_radix(&s[4..6], 16).map_err(|e| e.to_string())?;
    Ok(Color::Rgb(r, g, b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn parse_hex() {
        assert_eq!(parse_hex_color("#a78bfa").unwrap(), Color::Rgb(167, 139, 250));
    }

    #[test]
    fn parse_theme_file() {
        let theme = parse_config(
            r##"
[theme]
bg = "#010203"
dup = "#ff0000"
"##,
        )
        .unwrap();
        assert_eq!(theme.bg, Color::Rgb(1, 2, 3));
        assert_eq!(theme.dup, Color::Rgb(255, 0, 0));
        assert_eq!(theme.purple, Theme::default().purple);
    }

    #[test]
    fn candidates_include_cwd() {
        let paths = config_candidates();
        assert!(paths.iter().any(|p| p.file_name() == Some(Path::new("awards-tui.toml").as_os_str())));
    }
}
