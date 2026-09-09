use crossterm::style::{Color as CtColor, ResetColor, SetBackgroundColor};
use crossterm::{cursor, execute, queue, terminal};
use std::io::{self, Stdout, Write};

pub(crate) fn terminal_row(row: usize) -> Option<u16> {
    u16::try_from(row).ok()
}

/// Toggle mouse capture at runtime. Capture ON lets the app read wheel/scroll
/// events but disables the terminal's native click-drag text selection; OFF
/// restores it. `Terminal::exit` always disables, so leftover capture can't
/// leak into the shell.
pub fn set_mouse_capture(on: bool) {
    let mut out = io::stdout();
    let _ = if on {
        execute!(out, crossterm::event::EnableMouseCapture)
    } else {
        execute!(out, crossterm::event::DisableMouseCapture)
    };
}

pub struct Terminal {
    stdout: Stdout,
    alt_screen: bool,
    mouse_support: bool,
    raw_mode: bool,
    /// When set, erase/default background uses this RGB so host theme charcoal
    /// cannot bleed through unpainted cells after `Clear` / SGR 0.
    canvas_rgb: Option<(u8, u8, u8)>,
}

pub struct TerminalOptions {
    pub alt_screen: bool,
    pub mouse_support: bool,
    pub raw_mode: bool,
    pub canvas_rgb: Option<(u8, u8, u8)>,
}

impl Default for TerminalOptions {
    fn default() -> Self {
        Self {
            alt_screen: true,
            mouse_support: false,
            raw_mode: true,
            canvas_rgb: None,
        }
    }
}

impl Terminal {
    pub fn new(options: &TerminalOptions) -> io::Result<Self> {
        Ok(Self {
            stdout: io::stdout(),
            alt_screen: options.alt_screen,
            mouse_support: options.mouse_support,
            raw_mode: options.raw_mode,
            canvas_rgb: options.canvas_rgb,
        })
    }

    pub fn enter(&mut self) -> io::Result<()> {
        if self.raw_mode {
            terminal::enable_raw_mode()?;
        }
        if self.alt_screen {
            execute!(self.stdout, terminal::EnterAlternateScreen)?;
        }
        if self.mouse_support {
            execute!(self.stdout, crossterm::event::EnableMouseCapture)?;
        }
        // Bracketed paste: deliver a paste as one Event::Paste(text) instead of
        // a stream of keystrokes, so multi-line paste fills the input rather than
        // submitting line by line. Harmless where unsupported.
        let _ = execute!(self.stdout, crossterm::event::EnableBracketedPaste);
        // Kitty keyboard protocol: modified keys like Shift+Enter are reported
        // distinctly where supported. Always push — unsupported terminals ignore
        // the sequence. Do NOT call `supports_keyboard_enhancement()` here:
        // crossterm's probe waits up to 2s for a primary-device reply, which
        // blocks alternate-screen takeover and delays the welcome logo.
        let _ = execute!(
            self.stdout,
            crossterm::event::PushKeyboardEnhancementFlags(
                crossterm::event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
            )
        );
        // OSC 11: make the terminal's *default* background match the app canvas
        // so SGR 0 / unstyled padding no longer reveals the host theme.
        if let Some((r, g, b)) = self.canvas_rgb {
            let _ = write!(self.stdout, "\x1b]11;#{r:02x}{g:02x}{b:02x}\x07");
            let _ = execute!(self.stdout, SetBackgroundColor(CtColor::Rgb { r, g, b }));
        }
        execute!(self.stdout, cursor::Hide)?;
        Ok(())
    }

    pub fn exit(&mut self) -> io::Result<()> {
        execute!(self.stdout, cursor::Show)?;
        // Harmless if nothing was pushed (empty stack / unsupported terminal).
        let _ = execute!(self.stdout, crossterm::event::PopKeyboardEnhancementFlags);
        // Always disable — harmless if never enabled, and the app may have
        // toggled capture on at runtime via `set_mouse_capture`.
        let _ = execute!(self.stdout, crossterm::event::DisableMouseCapture);
        let _ = execute!(self.stdout, crossterm::event::DisableBracketedPaste);
        if self.canvas_rgb.is_some() {
            // OSC 111 restores the terminal's default background.
            let _ = write!(self.stdout, "\x1b]111\x07");
            let _ = execute!(self.stdout, ResetColor);
        }
        if self.alt_screen {
            execute!(self.stdout, terminal::LeaveAlternateScreen)?;
        }
        if self.raw_mode {
            terminal::disable_raw_mode()?;
        }
        Ok(())
    }

    pub fn draw(&mut self, content: &str) -> io::Result<()> {
        // Erase with the canvas color. Without this, Clear fills the host
        // theme background (often charcoal ~#15181d) and it shows through any
        // cell that is not explicitly painted.
        if let Some((r, g, b)) = self.canvas_rgb {
            queue!(
                self.stdout,
                SetBackgroundColor(CtColor::Rgb { r, g, b }),
                cursor::MoveTo(0, 0),
                terminal::Clear(terminal::ClearType::All),
            )?;
        } else {
            queue!(
                self.stdout,
                cursor::MoveTo(0, 0),
                terminal::Clear(terminal::ClearType::All),
            )?;
        }
        // Raw mode: '\n' is line-feed only and does NOT reset the column, so a
        // bare multi-line write makes each line start where the previous ended
        // (a staircase). Position every line at column 0 explicitly.
        for (row, line) in content.lines().enumerate() {
            let Some(row) = terminal_row(row) else {
                break;
            };
            queue!(self.stdout, cursor::MoveTo(0, row))?;
            write!(self.stdout, "{}", line)?;
        }
        self.stdout.flush()
    }

    pub fn draw_line(&mut self, row: u16, content: &str) -> io::Result<()> {
        queue!(
            self.stdout,
            cursor::MoveTo(0, row),
            terminal::Clear(terminal::ClearType::CurrentLine),
        )?;
        write!(self.stdout, "{}", content)?;
        Ok(())
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.stdout.flush()
    }

    /// Show the real terminal cursor at `(col, row)` (e.g. the input insertion
    /// point). Drawn after content so it lands on top.
    pub fn show_cursor_at(&mut self, col: u16, row: u16) -> io::Result<()> {
        queue!(self.stdout, cursor::MoveTo(col, row), cursor::Show)?;
        self.stdout.flush()
    }

    pub fn hide_cursor(&mut self) -> io::Result<()> {
        queue!(self.stdout, cursor::Hide)?;
        self.stdout.flush()
    }

    pub fn write_raw(&mut self, s: &str) -> io::Result<()> {
        write!(self.stdout, "{}", s)
    }

    pub fn stdout_mut(&mut self) -> &mut Stdout {
        &mut self.stdout
    }

    pub fn size() -> io::Result<(u16, u16)> {
        terminal::size()
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.exit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_row_rejects_values_that_would_wrap() {
        assert_eq!(terminal_row(u16::MAX as usize), Some(u16::MAX));
        assert_eq!(terminal_row(u16::MAX as usize + 1), None);
        assert_eq!(terminal_row(usize::MAX), None);
    }
}
