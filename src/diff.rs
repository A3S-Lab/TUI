//! Incremental diff renderer — only redraws cells that changed between frames.

use crate::grid::{CellChange, Grid};
use crate::terminal::Terminal;
use crossterm::{cursor, queue};
use std::io::{self, Write};

pub struct DiffRenderer {
    previous: Option<Grid>,
}

impl DiffRenderer {
    pub fn new() -> Self {
        Self { previous: None }
    }

    pub fn render(&mut self, terminal: &mut Terminal, grid: Grid) -> io::Result<()> {
        match &self.previous {
            None => {
                self.full_paint(terminal, &grid)?;
            }
            Some(prev) => {
                let changes = prev.diff(&grid);
                if !changes.is_empty() {
                    self.apply_changes(terminal, &changes)?;
                }
            }
        }
        self.previous = Some(grid);
        Ok(())
    }

    fn full_paint(&self, terminal: &mut Terminal, grid: &Grid) -> io::Result<()> {
        let mut output = String::with_capacity((grid.width as usize * grid.height as usize) * 4);
        output.push_str("\x1b[H\x1b[2J");

        for row in 0..grid.height {
            if row > 0 {
                output.push_str("\r\n");
            }
            for col in 0..grid.width {
                grid.cells[row as usize][col as usize].write_ansi(&mut output);
            }
        }

        terminal.write_raw(&output)?;
        terminal.flush()
    }

    fn apply_changes(&self, terminal: &mut Terminal, changes: &[CellChange]) -> io::Result<()> {
        let stdout = terminal.stdout_mut();
        let mut last_x: i32 = -1;
        let mut last_y: i32 = -1;

        for change in changes {
            if change.y as i32 != last_y || change.x as i32 != last_x + 1 {
                queue!(stdout, cursor::MoveTo(change.x, change.y))?;
            }
            write!(stdout, "{}", change.cell.to_ansi())?;
            last_x = change.x as i32;
            last_y = change.y as i32;
        }

        stdout.flush()
    }
}

impl Default for DiffRenderer {
    fn default() -> Self {
        Self::new()
    }
}
