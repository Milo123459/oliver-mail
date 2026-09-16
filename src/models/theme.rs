use crate::assets::Assets;
use serde::Deserialize;

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
        let asset_path = Self::available()
            .into_iter()
            .find(|theme| theme.id == name)
            .map(|theme| format!("themes/{}.json", theme.id))
            .unwrap_or_else(|| panic!("Unknown theme: {name}"));
        let file = Self::read_asset(&asset_path);
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
        let mut themes = Assets::iter()
            .filter_map(|path| path.strip_prefix("themes/").map(str::to_owned))
            .filter(|path| path.ends_with(".json"))
            .filter_map(|path| {
                let id = path.strip_suffix(".json")?.to_string();
                let file = Self::read_asset(&format!("themes/{id}.json"));
                if file.style.is_empty() {
                    return None;
                }
                Some(ThemeInfo {
                    id: id.clone(),
                    name: if file.name.is_empty() { id } else { file.name },
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

    fn read_asset(path: &str) -> ThemeFile {
        let asset = Assets::get(path).unwrap_or_else(|| panic!("Theme asset not found: {path}"));
        serde_json::from_slice(&asset.data)
            .unwrap_or_else(|error| panic!("Failed to parse theme {path}: {error}"))
    }

    pub fn color(value: &str) -> u32 {
        u32::from_str_radix(value.trim_start_matches('#'), 16)
            .unwrap_or_else(|_| panic!("Invalid theme color: {value}"))
    }
}
