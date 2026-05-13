//! Theme system for consistent component styling.
//!
//! Themes define a color palette that components use for rendering.
//! Use built-in themes or create custom ones by implementing [`Theme`].

use crate::style::Color;

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
            primary: Color::Rgb(137, 180, 250),    // blue
            secondary: Color::Rgb(180, 190, 254),  // lavender
            bg: Color::Rgb(30, 30, 46),            // base
            fg: Color::Rgb(205, 214, 244),         // text
            muted: Color::Rgb(127, 132, 156),      // overlay1
            border: Color::Rgb(88, 91, 112),       // surface2
            success: Color::Rgb(166, 227, 161),    // green
            warning: Color::Rgb(249, 226, 175),    // yellow
            error: Color::Rgb(243, 139, 168),      // red
            info: Color::Rgb(116, 199, 236),       // sapphire
            surface: Color::Rgb(49, 50, 68),       // surface0
            highlight: Color::Rgb(69, 71, 90),     // surface1
        }
    }

    /// Tokyo Night-inspired theme.
    pub fn tokyo_night() -> Self {
        Self {
            primary: Color::Rgb(122, 162, 247),    // blue
            secondary: Color::Rgb(187, 154, 247),  // purple
            bg: Color::Rgb(26, 27, 38),            // bg
            fg: Color::Rgb(192, 202, 245),         // fg
            muted: Color::Rgb(86, 95, 137),        // comment
            border: Color::Rgb(61, 89, 161),       // blue dark
            success: Color::Rgb(158, 206, 106),    // green
            warning: Color::Rgb(224, 175, 104),    // orange
            error: Color::Rgb(247, 118, 142),      // red
            info: Color::Rgb(125, 207, 255),       // cyan
            surface: Color::Rgb(36, 40, 59),       // bg_highlight
            highlight: Color::Rgb(41, 46, 66),     // selection
        }
    }
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
        matches!(theme.primary, Color::Rgb(_, _, _));
        matches!(theme.bg, Color::Rgb(_, _, _));
    }

    #[test]
    fn tokyo_night_uses_rgb() {
        let theme = Theme::tokyo_night();
        matches!(theme.primary, Color::Rgb(_, _, _));
        matches!(theme.bg, Color::Rgb(_, _, _));
    }
}
