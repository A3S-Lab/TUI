use crossterm::{cursor, execute, queue, terminal};
use std::io::{self, Stdout, Write};

pub struct Terminal {
    stdout: Stdout,
    alt_screen: bool,
    mouse_support: bool,
    raw_mode: bool,
}

pub struct TerminalOptions {
    pub alt_screen: bool,
    pub mouse_support: bool,
    pub raw_mode: bool,
}

impl Default for TerminalOptions {
    fn default() -> Self {
        Self {
            alt_screen: true,
            mouse_support: false,
            raw_mode: true,
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
        // Kitty keyboard protocol where supported → modified keys like
        // Shift+Enter are reported distinctly. Terminals without it ignore this.
        if matches!(terminal::supports_keyboard_enhancement(), Ok(true)) {
            let _ = execute!(
                self.stdout,
                crossterm::event::PushKeyboardEnhancementFlags(
                    crossterm::event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                )
            );
        }
        execute!(self.stdout, cursor::Hide)?;
        Ok(())
    }

    pub fn exit(&mut self) -> io::Result<()> {
        execute!(self.stdout, cursor::Show)?;
        // Harmless if nothing was pushed (empty stack / unsupported terminal).
        let _ = execute!(self.stdout, crossterm::event::PopKeyboardEnhancementFlags);
        if self.mouse_support {
            execute!(self.stdout, crossterm::event::DisableMouseCapture)?;
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
        queue!(
            self.stdout,
            cursor::MoveTo(0, 0),
            terminal::Clear(terminal::ClearType::All),
        )?;
        // Raw mode: '\n' is line-feed only and does NOT reset the column, so a
        // bare multi-line write makes each line start where the previous ended
        // (a staircase). Position every line at column 0 explicitly.
        for (row, line) in content.lines().enumerate() {
            queue!(self.stdout, cursor::MoveTo(0, row as u16))?;
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
