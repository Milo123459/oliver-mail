use serde::Deserialize;
use std::{fs, path::PathBuf};

#[derive(Clone, Debug)]
pub struct Theme {
    pub name: String,
    pub background: String,
    pub surface: String,
    pub surface_hover: String,
    pub border: String,
    pub text: String,
    pub text_muted: String,
    pub text_inactive: String,
    pub selected: String,
    pub selected_text: String,
    pub selected_option: String,
}

#[derive(Debug, Deserialize)]
struct ThemeFile {
    #[serde(default)]
    name: String,
    style: Vec<ThemeColors>,
}

#[derive(Clone, Debug)]
pub struct ThemeInfo {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
struct ThemeColors {
    background: String,
    surface: String,
    #[serde(rename = "surface-hover")]
    surface_hover: String,
    border: String,
    text: String,
    #[serde(rename = "text-muted")]
    text_muted: String,
    #[serde(rename = "text-inactive")]
    text_inactive: String,
    selected: String,
    #[serde(rename = "selected-text")]
    selected_text: String,
    #[serde(rename = "selected-option")]
    selected_option: String,
}

impl Theme {
    pub fn load() -> Self {
        let name = Self::default_name().expect("No themes found in assets/themes");
        Self::load_named(&name)
    }

    pub fn load_named(name: &str) -> Self {
        let path = Self::available()
            .into_iter()
            .find(|theme| theme.id == name)
            .map(|theme| Self::theme_directory().join(format!("{}.json", theme.id)))
            .unwrap_or_else(|| panic!("Unknown theme: {name}"));
        let contents = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("Failed to read theme {}: {error}", path.display()));
        let file: ThemeFile = serde_json::from_str(&contents)
            .unwrap_or_else(|error| panic!("Failed to parse theme {}: {error}", path.display()));
        let colors = file
            .style
            .into_iter()
            .next()
            .expect("The dark theme must define a style");

        Self {
            name: name.to_string(),
            background: colors.background,
            surface: colors.surface,
            surface_hover: colors.surface_hover,
            border: colors.border,
            text: colors.text,
            text_muted: colors.text_muted,
            text_inactive: colors.text_inactive,
            selected: colors.selected,
            selected_text: colors.selected_text,
            selected_option: colors.selected_option,
        }
    }

    pub fn available() -> Vec<ThemeInfo> {
        let directory = Self::theme_directory();
        let entries = fs::read_dir(&directory)
            .unwrap_or_else(|error| panic!("Failed to read {}: {error}", directory.display()));

        let mut themes = entries
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "json")
            })
            .filter_map(|entry| {
                let path = entry.path();
                let id = path.file_stem()?.to_str()?.to_string();
                let contents = fs::read_to_string(&path).ok()?;
                let file: ThemeFile = serde_json::from_str(&contents).ok()?;
                if file.style.is_empty() {
                    return None;
                }
                let fallback_name = path.file_stem()?.to_str()?.to_string();
                Some(ThemeInfo {
                    id,
                    name: if file.name.is_empty() {
                        fallback_name
                    } else {
                        file.name
                    },
                })
            })
            .collect::<Vec<_>>();

        themes.sort_by(|left, right| left.name.cmp(&right.name));
        themes
    }

    pub fn default_name() -> Option<String> {
        let themes = Self::available();
        themes
            .iter()
            .find(|theme| theme.id == "zed")
            .or_else(|| themes.first())
            .map(|theme| theme.id.clone())
    }

    fn theme_directory() -> PathBuf {
        let working_directory = PathBuf::from("assets/themes");
        if working_directory.is_dir() {
            working_directory
        } else {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/themes")
        }
    }

    pub fn color(value: &str) -> u32 {
        u32::from_str_radix(value.trim_start_matches('#'), 16)
            .unwrap_or_else(|_| panic!("Invalid theme color: {value}"))
    }
}
