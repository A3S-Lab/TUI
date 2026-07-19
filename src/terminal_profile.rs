//! Conservative terminal capability detection.
//!
//! Terminal applications should not infer support from one environment
//! variable or assume every terminal and multiplexer forwards the same control
//! sequences. This module turns the small set of process-local facts that are
//! safe to inspect into a typed profile. Unknown capabilities stay unknown so
//! callers can choose a documented fallback instead of emitting an unsafe
//! sequence.

use std::fmt;
use std::io::IsTerminal;

const MAX_ENV_VALUE_CHARS: usize = 128;

/// Terminal emulator family detected from well-known, non-secret variables.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TerminalFamily {
    Alacritty,
    AppleTerminal,
    Dumb,
    Ghostty,
    Iterm2,
    Kitty,
    Konsole,
    Vscode,
    Vte,
    WezTerm,
    WindowsTerminal,
    Xterm,
    #[default]
    Unknown,
}

impl fmt::Display for TerminalFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Alacritty => "Alacritty",
            Self::AppleTerminal => "Apple Terminal",
            Self::Dumb => "dumb",
            Self::Ghostty => "Ghostty",
            Self::Iterm2 => "iTerm2",
            Self::Kitty => "Kitty",
            Self::Konsole => "Konsole",
            Self::Vscode => "VS Code",
            Self::Vte => "VTE",
            Self::WezTerm => "WezTerm",
            Self::WindowsTerminal => "Windows Terminal",
            Self::Xterm => "xterm",
            Self::Unknown => "unknown",
        })
    }
}

/// Terminal multiplexer between the application and emulator.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TerminalMultiplexer {
    Screen,
    Tmux,
    Zellij,
    #[default]
    None,
}

impl fmt::Display for TerminalMultiplexer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Screen => "GNU Screen",
            Self::Tmux => "tmux",
            Self::Zellij => "Zellij",
            Self::None => "none",
        })
    }
}

/// Effective color depth inferred from the terminal environment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TerminalColorLevel {
    None,
    Ansi16,
    Ansi256,
    TrueColor,
    #[default]
    Unknown,
}

impl fmt::Display for TerminalColorLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::None => "none",
            Self::Ansi16 => "16 colors",
            Self::Ansi256 => "256 colors",
            Self::TrueColor => "truecolor",
            Self::Unknown => "unknown",
        })
    }
}

/// Confidence-aware support result for one terminal capability.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TerminalSupport {
    Supported,
    Unsupported,
    RequiresPassthrough,
    #[default]
    Unknown,
}

impl TerminalSupport {
    pub fn is_supported(self) -> bool {
        matches!(self, Self::Supported)
    }
}

impl fmt::Display for TerminalSupport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Supported => "supported",
            Self::Unsupported => "unsupported",
            Self::RequiresPassthrough => "multiplexer passthrough required",
            Self::Unknown => "unknown",
        })
    }
}

/// Presentation mode that is safe for the detected I/O boundary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TerminalDisplayMode {
    Fullscreen,
    #[default]
    Inline,
}

impl fmt::Display for TerminalDisplayMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Fullscreen => "fullscreen",
            Self::Inline => "inline fallback",
        })
    }
}

/// Typed terminal facts and conservative capability decisions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalProfile {
    family: TerminalFamily,
    multiplexer: TerminalMultiplexer,
    term: Option<String>,
    term_program: Option<String>,
    stdin_tty: bool,
    stdout_tty: bool,
    stderr_tty: bool,
    color_level: TerminalColorLevel,
    display_mode: TerminalDisplayMode,
    alternate_screen: TerminalSupport,
    bracketed_paste: TerminalSupport,
    mouse_capture: TerminalSupport,
    enhanced_keyboard: TerminalSupport,
    hyperlinks: TerminalSupport,
    clipboard: TerminalSupport,
}

impl TerminalProfile {
    /// Detect a profile from the current process without reading arbitrary
    /// environment values.
    pub fn detect() -> Self {
        let io = TerminalIo {
            stdin: std::io::stdin().is_terminal(),
            stdout: std::io::stdout().is_terminal(),
            stderr: std::io::stderr().is_terminal(),
        };
        let enhanced_keyboard = match crossterm::terminal::supports_keyboard_enhancement() {
            Ok(true) => TerminalSupport::Supported,
            Ok(false) => TerminalSupport::Unsupported,
            Err(_) => TerminalSupport::Unknown,
        };
        Self::detect_with(|name| std::env::var(name).ok(), io, enhanced_keyboard)
    }

    pub fn family(&self) -> TerminalFamily {
        self.family
    }

    pub fn multiplexer(&self) -> TerminalMultiplexer {
        self.multiplexer
    }

    pub fn term(&self) -> Option<&str> {
        self.term.as_deref()
    }

    pub fn term_program(&self) -> Option<&str> {
        self.term_program.as_deref()
    }

    pub fn stdin_is_terminal(&self) -> bool {
        self.stdin_tty
    }

    pub fn stdout_is_terminal(&self) -> bool {
        self.stdout_tty
    }

    pub fn stderr_is_terminal(&self) -> bool {
        self.stderr_tty
    }

    pub fn color_level(&self) -> TerminalColorLevel {
        self.color_level
    }

    pub fn display_mode(&self) -> TerminalDisplayMode {
        self.display_mode
    }

    pub fn alternate_screen(&self) -> TerminalSupport {
        self.alternate_screen
    }

    pub fn bracketed_paste(&self) -> TerminalSupport {
        self.bracketed_paste
    }

    pub fn mouse_capture(&self) -> TerminalSupport {
        self.mouse_capture
    }

    pub fn enhanced_keyboard(&self) -> TerminalSupport {
        self.enhanced_keyboard
    }

    pub fn hyperlinks(&self) -> TerminalSupport {
        self.hyperlinks
    }

    pub fn clipboard(&self) -> TerminalSupport {
        self.clipboard
    }

    /// Actionable, bounded warnings suitable for a diagnostics screen.
    pub fn warnings(&self) -> Vec<&'static str> {
        let mut warnings = Vec::new();
        if self.display_mode == TerminalDisplayMode::Inline {
            warnings.push("fullscreen mode is unsafe because stdin/stdout are not interactive");
        }
        if self.family == TerminalFamily::Dumb {
            warnings.push("TERM=dumb disables interactive terminal features");
        } else if self.family == TerminalFamily::Unknown {
            warnings.push("terminal family is unknown; optional escape sequences use fallbacks");
        }
        if self.multiplexer != TerminalMultiplexer::None {
            if self.hyperlinks == TerminalSupport::RequiresPassthrough {
                warnings.push("hyperlinks depend on multiplexer escape-sequence passthrough");
            }
            if self.clipboard == TerminalSupport::RequiresPassthrough {
                warnings.push("clipboard copy depends on multiplexer OSC 52 passthrough");
            }
        }
        if !self.enhanced_keyboard.is_supported() {
            warnings.push("enhanced key reporting is unavailable; modified-key fallbacks apply");
        }
        warnings
    }

    fn detect_with(
        lookup: impl Fn(&str) -> Option<String>,
        io: TerminalIo,
        enhanced_keyboard: TerminalSupport,
    ) -> Self {
        let term = env_value(&lookup, "TERM");
        let term_program = env_value(&lookup, "TERM_PROGRAM");
        let multiplexer = detect_multiplexer(&lookup, term.as_deref());
        let family = detect_family(&lookup, term.as_deref(), term_program.as_deref());
        let interactive = io.stdin && io.stdout && family != TerminalFamily::Dumb;
        let baseline = if interactive {
            TerminalSupport::Supported
        } else {
            TerminalSupport::Unsupported
        };
        let display_mode = if interactive {
            TerminalDisplayMode::Fullscreen
        } else {
            TerminalDisplayMode::Inline
        };
        let enhanced_keyboard = if interactive {
            enhanced_keyboard
        } else {
            TerminalSupport::Unsupported
        };
        let color_level = detect_color_level(&lookup, term.as_deref(), family, interactive);
        let (hyperlinks, clipboard) =
            detect_escape_capabilities(&lookup, family, multiplexer, interactive);

        Self {
            family,
            multiplexer,
            term,
            term_program,
            stdin_tty: io.stdin,
            stdout_tty: io.stdout,
            stderr_tty: io.stderr,
            color_level,
            display_mode,
            alternate_screen: baseline,
            bracketed_paste: baseline,
            mouse_capture: baseline,
            enhanced_keyboard,
            hyperlinks,
            clipboard,
        }
    }
}

#[derive(Clone, Copy)]
struct TerminalIo {
    stdin: bool,
    stdout: bool,
    stderr: bool,
}

fn env_value(lookup: &impl Fn(&str) -> Option<String>, name: &str) -> Option<String> {
    lookup(name).and_then(|value| sanitize_env_value(&value))
}

fn env_present(lookup: &impl Fn(&str) -> Option<String>, name: &str) -> bool {
    lookup(name).is_some_and(|value| !value.is_empty())
}

fn sanitize_env_value(value: &str) -> Option<String> {
    let value = value
        .chars()
        .filter(|character| !character.is_control())
        .take(MAX_ENV_VALUE_CHARS)
        .collect::<String>();
    (!value.is_empty()).then_some(value)
}

fn detect_multiplexer(
    lookup: &impl Fn(&str) -> Option<String>,
    term: Option<&str>,
) -> TerminalMultiplexer {
    if env_present(lookup, "TMUX") {
        return TerminalMultiplexer::Tmux;
    }
    if env_present(lookup, "ZELLIJ") || env_present(lookup, "ZELLIJ_SESSION_NAME") {
        return TerminalMultiplexer::Zellij;
    }
    if env_present(lookup, "STY") || term.is_some_and(|term| term.starts_with("screen")) {
        return TerminalMultiplexer::Screen;
    }
    TerminalMultiplexer::None
}

fn detect_family(
    lookup: &impl Fn(&str) -> Option<String>,
    term: Option<&str>,
    term_program: Option<&str>,
) -> TerminalFamily {
    if term.is_some_and(|term| term.eq_ignore_ascii_case("dumb")) {
        return TerminalFamily::Dumb;
    }

    let program = term_program.unwrap_or_default().to_ascii_lowercase();
    if program.contains("ghostty") || env_present(lookup, "GHOSTTY_RESOURCES_DIR") {
        return TerminalFamily::Ghostty;
    }
    if program.contains("wezterm") || env_present(lookup, "WEZTERM_PANE") {
        return TerminalFamily::WezTerm;
    }
    if program.contains("iterm") || env_present(lookup, "ITERM_SESSION_ID") {
        return TerminalFamily::Iterm2;
    }
    if program.contains("apple_terminal") {
        return TerminalFamily::AppleTerminal;
    }
    if program.contains("vscode") || env_present(lookup, "VSCODE_INJECTION") {
        return TerminalFamily::Vscode;
    }
    if env_present(lookup, "WT_SESSION") {
        return TerminalFamily::WindowsTerminal;
    }
    if program.contains("kitty")
        || env_present(lookup, "KITTY_WINDOW_ID")
        || term.is_some_and(|term| term.contains("kitty"))
    {
        return TerminalFamily::Kitty;
    }
    if env_present(lookup, "KONSOLE_VERSION") {
        return TerminalFamily::Konsole;
    }
    if program.contains("alacritty") || term.is_some_and(|term| term.contains("alacritty")) {
        return TerminalFamily::Alacritty;
    }
    if env_present(lookup, "VTE_VERSION") {
        return TerminalFamily::Vte;
    }
    if term.is_some_and(|term| term.contains("xterm")) {
        return TerminalFamily::Xterm;
    }
    TerminalFamily::Unknown
}

fn detect_color_level(
    lookup: &impl Fn(&str) -> Option<String>,
    term: Option<&str>,
    family: TerminalFamily,
    interactive: bool,
) -> TerminalColorLevel {
    if !interactive {
        return TerminalColorLevel::None;
    }
    let color_term = env_value(lookup, "COLORTERM")
        .unwrap_or_default()
        .to_ascii_lowercase();
    if matches!(color_term.as_str(), "truecolor" | "24bit") {
        return TerminalColorLevel::TrueColor;
    }
    if term.is_some_and(|term| term.contains("256color")) {
        return TerminalColorLevel::Ansi256;
    }
    if matches!(
        family,
        TerminalFamily::Ghostty
            | TerminalFamily::Iterm2
            | TerminalFamily::Kitty
            | TerminalFamily::Vscode
            | TerminalFamily::WezTerm
            | TerminalFamily::WindowsTerminal
    ) {
        return TerminalColorLevel::TrueColor;
    }
    TerminalColorLevel::Ansi16
}

fn detect_escape_capabilities(
    lookup: &impl Fn(&str) -> Option<String>,
    family: TerminalFamily,
    multiplexer: TerminalMultiplexer,
    interactive: bool,
) -> (TerminalSupport, TerminalSupport) {
    if !interactive {
        return (TerminalSupport::Unsupported, TerminalSupport::Unsupported);
    }
    if multiplexer != TerminalMultiplexer::None {
        return (
            TerminalSupport::RequiresPassthrough,
            TerminalSupport::RequiresPassthrough,
        );
    }

    let hyperlinks = match family {
        TerminalFamily::Ghostty
        | TerminalFamily::Iterm2
        | TerminalFamily::Kitty
        | TerminalFamily::Konsole
        | TerminalFamily::Vscode
        | TerminalFamily::WezTerm
        | TerminalFamily::WindowsTerminal => TerminalSupport::Supported,
        TerminalFamily::Vte if vte_supports_hyperlinks(lookup) => TerminalSupport::Supported,
        TerminalFamily::Dumb => TerminalSupport::Unsupported,
        _ => TerminalSupport::Unknown,
    };
    let clipboard = match family {
        TerminalFamily::Ghostty
        | TerminalFamily::Iterm2
        | TerminalFamily::Kitty
        | TerminalFamily::Vscode
        | TerminalFamily::WezTerm
        | TerminalFamily::WindowsTerminal => TerminalSupport::Supported,
        TerminalFamily::Dumb => TerminalSupport::Unsupported,
        _ => TerminalSupport::Unknown,
    };
    (hyperlinks, clipboard)
}

fn vte_supports_hyperlinks(lookup: &impl Fn(&str) -> Option<String>) -> bool {
    env_value(lookup, "VTE_VERSION")
        .and_then(|version| version.parse::<u32>().ok())
        .is_some_and(|version| version >= 5_000)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn profile(
        values: &[(&str, &str)],
        io: TerminalIo,
        keyboard: TerminalSupport,
    ) -> TerminalProfile {
        let values = values
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect::<HashMap<_, _>>();
        TerminalProfile::detect_with(|name| values.get(name).cloned(), io, keyboard)
    }

    fn interactive_io() -> TerminalIo {
        TerminalIo {
            stdin: true,
            stdout: true,
            stderr: true,
        }
    }

    #[test]
    fn kitty_inside_tmux_requires_explicit_escape_passthrough() {
        let profile = profile(
            &[
                ("TERM", "screen-256color"),
                ("TERM_PROGRAM", "kitty"),
                ("TMUX", "/tmp/tmux-501/default,123,0"),
                ("COLORTERM", "truecolor"),
            ],
            interactive_io(),
            TerminalSupport::Supported,
        );

        assert_eq!(profile.family(), TerminalFamily::Kitty);
        assert_eq!(profile.multiplexer(), TerminalMultiplexer::Tmux);
        assert_eq!(profile.color_level(), TerminalColorLevel::TrueColor);
        assert_eq!(profile.display_mode(), TerminalDisplayMode::Fullscreen);
        assert_eq!(profile.hyperlinks(), TerminalSupport::RequiresPassthrough);
        assert_eq!(profile.clipboard(), TerminalSupport::RequiresPassthrough);
        assert!(profile
            .warnings()
            .iter()
            .any(|warning| warning.contains("OSC 52")));
    }

    #[test]
    fn dumb_or_noninteractive_term_uses_inline_fallbacks() {
        let profile = profile(
            &[("TERM", "dumb")],
            TerminalIo {
                stdin: false,
                stdout: false,
                stderr: true,
            },
            TerminalSupport::Supported,
        );

        assert_eq!(profile.family(), TerminalFamily::Dumb);
        assert_eq!(profile.color_level(), TerminalColorLevel::None);
        assert_eq!(profile.display_mode(), TerminalDisplayMode::Inline);
        assert_eq!(profile.enhanced_keyboard(), TerminalSupport::Unsupported);
        assert_eq!(profile.mouse_capture(), TerminalSupport::Unsupported);
        assert_eq!(profile.bracketed_paste(), TerminalSupport::Unsupported);
    }

    #[test]
    fn modern_emulators_expose_known_native_capabilities() {
        let profile = profile(
            &[("TERM", "xterm-256color"), ("TERM_PROGRAM", "WezTerm")],
            interactive_io(),
            TerminalSupport::Supported,
        );

        assert_eq!(profile.family(), TerminalFamily::WezTerm);
        assert_eq!(profile.multiplexer(), TerminalMultiplexer::None);
        assert_eq!(profile.color_level(), TerminalColorLevel::Ansi256);
        assert_eq!(profile.hyperlinks(), TerminalSupport::Supported);
        assert_eq!(profile.clipboard(), TerminalSupport::Supported);
        assert_eq!(profile.alternate_screen(), TerminalSupport::Supported);
    }

    #[test]
    fn recent_vte_reports_hyperlink_support_without_guessing_clipboard() {
        let profile = profile(
            &[("TERM", "xterm-256color"), ("VTE_VERSION", "7200")],
            interactive_io(),
            TerminalSupport::Unknown,
        );

        assert_eq!(profile.family(), TerminalFamily::Vte);
        assert_eq!(profile.hyperlinks(), TerminalSupport::Supported);
        assert_eq!(profile.clipboard(), TerminalSupport::Unknown);
    }

    #[test]
    fn displayed_environment_values_are_bounded_and_control_free() {
        let hostile = format!("xterm\u{1b}[31m{}", "x".repeat(300));
        let profile = profile(
            &[("TERM", &hostile)],
            interactive_io(),
            TerminalSupport::Unknown,
        );
        let term = profile.term().expect("TERM should be retained safely");

        assert!(!term.contains('\u{1b}'));
        assert_eq!(term.chars().count(), MAX_ENV_VALUE_CHARS);
    }
}
