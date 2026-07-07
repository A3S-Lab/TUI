//! Theme system for consistent component styling.
//!
//! Themes define a color palette that components use for rendering.
//! Use built-in themes or create custom ones by implementing [`Theme`].

use crate::style::{Color, Style};
use std::{error::Error, fmt, str::FromStr};

static BUILTIN_THEMES: [BuiltinTheme; 4] = BuiltinTheme::ALL;
static THEME_ROLES: [ThemeRole; 12] = ThemeRole::ALL;

/// Built-in theme presets shipped with A3S TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinTheme {
    Dark,
    Light,
    Catppuccin,
    TokyoNight,
}

impl BuiltinTheme {
    pub const ALL: [Self; 4] = [Self::Dark, Self::Light, Self::Catppuccin, Self::TokyoNight];

    /// Stable machine-readable name for configuration and persistence.
    pub fn name(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
            Self::Catppuccin => "catppuccin",
            Self::TokyoNight => "tokyo-night",
        }
    }

    /// Human-readable label for pickers and help text.
    pub fn label(self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Light => "Light",
            Self::Catppuccin => "Catppuccin",
            Self::TokyoNight => "Tokyo Night",
        }
    }

    /// Build the concrete [`Theme`] for this preset.
    pub fn theme(self) -> Theme {
        match self {
            Self::Dark => Theme::dark(),
            Self::Light => Theme::light(),
            Self::Catppuccin => Theme::catppuccin(),
            Self::TokyoNight => Theme::tokyo_night(),
        }
    }
}

impl FromStr for BuiltinTheme {
    type Err = ParseBuiltinThemeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match normalized_token_name(value).as_str() {
            "dark" => Ok(Self::Dark),
            "light" => Ok(Self::Light),
            "catppuccin" | "catppuccin-mocha" => Ok(Self::Catppuccin),
            "tokyo-night" | "tokyonight" => Ok(Self::TokyoNight),
            _ => Err(ParseBuiltinThemeError),
        }
    }
}

/// Error returned when parsing a built-in theme name fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseBuiltinThemeError;

impl fmt::Display for ParseBuiltinThemeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("unknown built-in theme")
    }
}

impl Error for ParseBuiltinThemeError {}

/// Semantic color roles available in every [`Theme`].
///
/// Roles are stable tokens for application code. Prefer roles over direct field
/// access when storing user preferences or mapping design-system names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeRole {
    Primary,
    Secondary,
    Background,
    Foreground,
    Muted,
    Border,
    Success,
    Warning,
    Error,
    Info,
    Surface,
    Highlight,
}

impl ThemeRole {
    pub const ALL: [Self; 12] = [
        Self::Primary,
        Self::Secondary,
        Self::Background,
        Self::Foreground,
        Self::Muted,
        Self::Border,
        Self::Success,
        Self::Warning,
        Self::Error,
        Self::Info,
        Self::Surface,
        Self::Highlight,
    ];

    /// Stable machine-readable role name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Secondary => "secondary",
            Self::Background => "background",
            Self::Foreground => "foreground",
            Self::Muted => "muted",
            Self::Border => "border",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Info => "info",
            Self::Surface => "surface",
            Self::Highlight => "highlight",
        }
    }

    /// Human-readable role label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Primary => "Primary",
            Self::Secondary => "Secondary",
            Self::Background => "Background",
            Self::Foreground => "Foreground",
            Self::Muted => "Muted",
            Self::Border => "Border",
            Self::Success => "Success",
            Self::Warning => "Warning",
            Self::Error => "Error",
            Self::Info => "Info",
            Self::Surface => "Surface",
            Self::Highlight => "Highlight",
        }
    }
}

impl FromStr for ThemeRole {
    type Err = ParseThemeRoleError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match normalized_token_name(value).as_str() {
            "primary" => Ok(Self::Primary),
            "secondary" => Ok(Self::Secondary),
            "background" | "bg" => Ok(Self::Background),
            "foreground" | "fg" | "text" => Ok(Self::Foreground),
            "muted" | "dim" => Ok(Self::Muted),
            "border" => Ok(Self::Border),
            "success" | "ok" => Ok(Self::Success),
            "warning" | "warn" => Ok(Self::Warning),
            "error" | "danger" => Ok(Self::Error),
            "info" => Ok(Self::Info),
            "surface" => Ok(Self::Surface),
            "highlight" | "selection" => Ok(Self::Highlight),
            _ => Err(ParseThemeRoleError),
        }
    }
}

/// Error returned when parsing a theme role fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseThemeRoleError;

impl fmt::Display for ParseThemeRoleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("unknown theme role")
    }
}

impl Error for ParseThemeRoleError {}

/// A color palette for theming UI components.
///
/// Components reference these semantic colors rather than hard-coding values,
/// allowing the entire UI to be restyled by swapping themes.
#[derive(Debug, Clone)]
pub struct Theme {
    /// Primary accent color (buttons, active elements).
    pub primary: Color,
    /// Secondary accent color (less prominent elements).
    pub secondary: Color,
    /// Background color for the main content area.
    pub bg: Color,
    /// Default foreground/text color.
    pub fg: Color,
    /// Muted/dimmed text color.
    pub muted: Color,
    /// Border color for containers.
    pub border: Color,
    /// Success state color.
    pub success: Color,
    /// Warning state color.
    pub warning: Color,
    /// Error/danger state color.
    pub error: Color,
    /// Informational state color.
    pub info: Color,
    /// Surface color (cards, panels, elevated elements).
    pub surface: Color,
    /// Highlight/selection background.
    pub highlight: Color,
}

impl Theme {
    /// Return the list of built-in theme presets.
    pub fn builtins() -> &'static [BuiltinTheme] {
        &BUILTIN_THEMES
    }

    /// Build one of the built-in theme presets.
    pub fn builtin(theme: BuiltinTheme) -> Self {
        theme.theme()
    }

    /// Build a built-in theme from a configuration-friendly name.
    pub fn from_builtin_name(name: &str) -> Option<Self> {
        name.parse::<BuiltinTheme>().ok().map(Self::builtin)
    }

    /// Return the list of stable semantic roles supported by every theme.
    pub fn roles() -> &'static [ThemeRole] {
        &THEME_ROLES
    }

    /// Resolve a semantic role to a concrete color.
    pub fn color(&self, role: ThemeRole) -> Color {
        match role {
            ThemeRole::Primary => self.primary,
            ThemeRole::Secondary => self.secondary,
            ThemeRole::Background => self.bg,
            ThemeRole::Foreground => self.fg,
            ThemeRole::Muted => self.muted,
            ThemeRole::Border => self.border,
            ThemeRole::Success => self.success,
            ThemeRole::Warning => self.warning,
            ThemeRole::Error => self.error,
            ThemeRole::Info => self.info,
            ThemeRole::Surface => self.surface,
            ThemeRole::Highlight => self.highlight,
        }
    }

    /// Create a foreground-only style from a semantic role.
    pub fn foreground_style(&self, role: ThemeRole) -> Style {
        Style::new().fg(self.color(role))
    }

    /// Create a background-only style from a semantic role.
    pub fn background_style(&self, role: ThemeRole) -> Style {
        Style::new().bg(self.color(role))
    }

    /// Create a style from separate foreground and background roles.
    pub fn pair_style(&self, fg: ThemeRole, bg: ThemeRole) -> Style {
        Style::new().fg(self.color(fg)).bg(self.color(bg))
    }

    /// Standard selected-row style for theme-aware widgets.
    pub fn selection_style(&self) -> Style {
        self.pair_style(ThemeRole::Foreground, ThemeRole::Highlight)
            .bold()
    }

    /// Standard elevated-surface style for panels and overlays.
    pub fn surface_style(&self) -> Style {
        self.pair_style(ThemeRole::Foreground, ThemeRole::Surface)
    }

    /// Dark theme — light text on dark background.
    pub fn dark() -> Self {
        Self {
            primary: Color::Cyan,
            secondary: Color::Blue,
            bg: Color::Black,
            fg: Color::White,
            muted: Color::BrightBlack,
            border: Color::BrightBlack,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            info: Color::Blue,
            surface: Color::BrightBlack,
            highlight: Color::BrightBlue,
        }
    }

    /// Light theme — dark text on light background.
    pub fn light() -> Self {
        Self {
            primary: Color::Blue,
            secondary: Color::Cyan,
            bg: Color::White,
            fg: Color::Black,
            muted: Color::BrightBlack,
            border: Color::BrightBlack,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            info: Color::Blue,
            surface: Color::BrightWhite,
            highlight: Color::BrightCyan,
        }
    }

    /// Catppuccin Mocha-inspired theme.
    pub fn catppuccin() -> Self {
        Self {
            primary: Color::Rgb(137, 180, 250),   // blue
            secondary: Color::Rgb(180, 190, 254), // lavender
            bg: Color::Rgb(30, 30, 46),           // base
            fg: Color::Rgb(205, 214, 244),        // text
            muted: Color::Rgb(127, 132, 156),     // overlay1
            border: Color::Rgb(88, 91, 112),      // surface2
            success: Color::Rgb(166, 227, 161),   // green
            warning: Color::Rgb(249, 226, 175),   // yellow
            error: Color::Rgb(243, 139, 168),     // red
            info: Color::Rgb(116, 199, 236),      // sapphire
            surface: Color::Rgb(49, 50, 68),      // surface0
            highlight: Color::Rgb(69, 71, 90),    // surface1
        }
    }

    /// Tokyo Night-inspired theme.
    pub fn tokyo_night() -> Self {
        Self {
            primary: Color::Rgb(122, 162, 247),   // blue
            secondary: Color::Rgb(187, 154, 247), // purple
            bg: Color::Rgb(26, 27, 38),           // bg
            fg: Color::Rgb(192, 202, 245),        // fg
            muted: Color::Rgb(86, 95, 137),       // comment
            border: Color::Rgb(61, 89, 161),      // blue dark
            success: Color::Rgb(158, 206, 106),   // green
            warning: Color::Rgb(224, 175, 104),   // orange
            error: Color::Rgb(247, 118, 142),     // red
            info: Color::Rgb(125, 207, 255),      // cyan
            surface: Color::Rgb(36, 40, 59),      // bg_highlight
            highlight: Color::Rgb(41, 46, 66),    // selection
        }
    }
}

fn normalized_token_name(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|ch| match ch {
            '_' | ' ' => '-',
            ch => ch.to_ascii_lowercase(),
        })
        .collect()
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_dark() {
        let theme = Theme::default();
        assert_eq!(theme.bg, Color::Black);
        assert_eq!(theme.fg, Color::White);
    }

    #[test]
    fn light_theme_has_white_bg() {
        let theme = Theme::light();
        assert_eq!(theme.bg, Color::White);
        assert_eq!(theme.fg, Color::Black);
    }

    #[test]
    fn catppuccin_uses_rgb() {
        let theme = Theme::catppuccin();
        assert!(matches!(theme.primary, Color::Rgb(_, _, _)));
        assert!(matches!(theme.bg, Color::Rgb(_, _, _)));
    }

    #[test]
    fn tokyo_night_uses_rgb() {
        let theme = Theme::tokyo_night();
        assert!(matches!(theme.primary, Color::Rgb(_, _, _)));
        assert!(matches!(theme.bg, Color::Rgb(_, _, _)));
    }

    #[test]
    fn built_in_themes_parse_friendly_names() {
        assert_eq!("dark".parse::<BuiltinTheme>(), Ok(BuiltinTheme::Dark));
        assert_eq!(
            "Tokyo Night".parse::<BuiltinTheme>(),
            Ok(BuiltinTheme::TokyoNight)
        );
        assert_eq!(
            "catppuccin_mocha".parse::<BuiltinTheme>(),
            Ok(BuiltinTheme::Catppuccin)
        );
        assert!("missing".parse::<BuiltinTheme>().is_err());
        assert_eq!(
            Theme::from_builtin_name("tokyo-night").unwrap().primary,
            Theme::tokyo_night().primary
        );
    }

    #[test]
    fn theme_roles_map_to_palette_colors() {
        let theme = Theme::dark();

        assert_eq!(Theme::roles().len(), ThemeRole::ALL.len());
        assert_eq!(theme.color(ThemeRole::Primary), theme.primary);
        assert_eq!(theme.color(ThemeRole::Background), theme.bg);
        assert_eq!(theme.color(ThemeRole::Highlight), theme.highlight);
        assert_eq!("fg".parse::<ThemeRole>(), Ok(ThemeRole::Foreground));
        assert_eq!("danger".parse::<ThemeRole>(), Ok(ThemeRole::Error));
        assert!("unknown".parse::<ThemeRole>().is_err());
    }

    #[test]
    fn theme_style_helpers_use_roles() {
        let theme = Theme::tokyo_night();
        let primary = theme.foreground_style(ThemeRole::Primary);
        let selected = theme.selection_style();
        let surface = theme.surface_style();

        assert_eq!(primary.foreground(), Some(theme.primary));
        assert_eq!(selected.foreground(), Some(theme.fg));
        assert_eq!(selected.background(), Some(theme.highlight));
        assert!(selected.is_bold());
        assert_eq!(surface.foreground(), Some(theme.fg));
        assert_eq!(surface.background(), Some(theme.surface));
    }
}
