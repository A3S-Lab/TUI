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
        execute!(self.stdout, cursor::Hide)?;
        Ok(())
    }

    pub fn exit(&mut self) -> io::Result<()> {
        execute!(self.stdout, cursor::Show)?;
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
        write!(self.stdout, "{}", content)?;
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
