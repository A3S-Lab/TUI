//! Cell grid for terminal rendering.
//!
//! The grid is a 2D array of [`Cell`]s representing the terminal screen buffer.
//! Each cell holds a character and its styling attributes.

use crate::style::Color;

/// A single terminal cell with character and style.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Cell {
    pub ch: char,
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub dim: bool,
    pub strikethrough: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: None,
            bg: None,
            bold: false,
            italic: false,
            underline: false,
            dim: false,
            strikethrough: false,
        }
    }
}

impl Cell {
    pub fn with_char(ch: char) -> Self {
        Self { ch, ..Default::default() }
    }

    pub fn styled(ch: char, style: &CellStyle) -> Self {
        Self {
            ch,
            fg: style.fg,
            bg: style.bg,
            bold: style.bold,
            italic: style.italic,
            underline: style.underline,
            dim: style.dim,
            strikethrough: style.strikethrough,
        }
    }

    pub fn to_ansi(&self) -> String {
        use std::fmt::Write;

        let has_style = self.bold || self.dim || self.italic || self.underline
            || self.strikethrough || self.fg.is_some() || self.bg.is_some();

        if !has_style {
            return self.ch.to_string();
        }

        let mut out = String::with_capacity(24);
        out.push_str("\x1b[");
        let mut first = true;

        macro_rules! push_code {
            ($code:expr) => {
                if !first { out.push(';'); }
                let _ = write!(out, "{}", $code);
                #[allow(unused_assignments)]
                { first = false; }
            };
        }

        if self.bold { push_code!("1"); }
        if self.dim { push_code!("2"); }
        if self.italic { push_code!("3"); }
        if self.underline { push_code!("4"); }
        if self.strikethrough { push_code!("9"); }
        if let Some(ref c) = self.fg { push_code!(c.fg_ansi()); }
        if let Some(ref c) = self.bg { push_code!(c.bg_ansi()); }

        out.push('m');
        out.push(self.ch);
        out.push_str("\x1b[0m");
        out
    }

    /// Write ANSI representation into an existing buffer (avoids allocation).
    pub fn write_ansi(&self, buf: &mut String) {
        use std::fmt::Write;

        let has_style = self.bold || self.dim || self.italic || self.underline
            || self.strikethrough || self.fg.is_some() || self.bg.is_some();

        if !has_style {
            buf.push(self.ch);
            return;
        }

        buf.push_str("\x1b[");
        let mut first = true;

        macro_rules! push_code {
            ($code:expr) => {
                if !first { buf.push(';'); }
                let _ = write!(buf, "{}", $code);
                #[allow(unused_assignments)]
                { first = false; }
            };
        }

        if self.bold { push_code!("1"); }
        if self.dim { push_code!("2"); }
        if self.italic { push_code!("3"); }
        if self.underline { push_code!("4"); }
        if self.strikethrough { push_code!("9"); }
        if let Some(ref c) = self.fg { push_code!(c.fg_ansi()); }
        if let Some(ref c) = self.bg { push_code!(c.bg_ansi()); }

        buf.push('m');
        buf.push(self.ch);
        buf.push_str("\x1b[0m");
    }
}

#[derive(Clone, Debug, Default)]
pub struct CellStyle {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub dim: bool,
    pub strikethrough: bool,
}

pub struct Grid {
    pub cells: Vec<Vec<Cell>>,
    pub width: u16,
    pub height: u16,
}

impl Grid {
    pub fn new(width: u16, height: u16) -> Self {
        let cells = vec![vec![Cell::default(); width as usize]; height as usize];
        Self { cells, width, height }
    }

    pub fn get(&self, x: u16, y: u16) -> &Cell {
        &self.cells[y as usize][x as usize]
    }

    pub fn set(&mut self, x: u16, y: u16, cell: Cell) {
        if x < self.width && y < self.height {
            self.cells[y as usize][x as usize] = cell;
        }
    }

    pub fn write_str(&mut self, x: u16, y: u16, text: &str, style: &CellStyle) {
        let mut col = x as usize;
        let row = y as usize;
        if row >= self.height as usize {
            return;
        }
        for ch in text.chars() {
            if col >= self.width as usize {
                break;
            }
            self.cells[row][col] = Cell::styled(ch, style);
            col += unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
        }
    }

    pub fn fill_bg(&mut self, x: u16, y: u16, w: u16, h: u16, color: Color) {
        for row in y..(y + h).min(self.height) {
            for col in x..(x + w).min(self.width) {
                self.cells[row as usize][col as usize].bg = Some(color);
            }
        }
    }

    pub fn diff(&self, other: &Grid) -> Vec<CellChange> {
        let mut changes = Vec::new();
        let max_h = self.height.min(other.height);
        let max_w = self.width.min(other.width);

        for y in 0..max_h {
            for x in 0..max_w {
                let old = &self.cells[y as usize][x as usize];
                let new = &other.cells[y as usize][x as usize];
                if old != new {
                    changes.push(CellChange { x, y, cell: new.clone() });
                }
            }
        }

        if other.height > self.height {
            for y in self.height..other.height {
                for x in 0..other.width {
                    let cell = &other.cells[y as usize][x as usize];
                    if *cell != Cell::default() {
                        changes.push(CellChange { x, y, cell: cell.clone() });
                    }
                }
            }
        }

        changes
    }

    /// Convert grid to a string representation (for testing/debugging).
    pub fn render_to_string(&self) -> String {
        let mut output = String::with_capacity((self.width as usize * self.height as usize) * 4);
        for y in 0..self.height {
            if y > 0 {
                output.push('\n');
            }
            for x in 0..self.width {
                self.cells[y as usize][x as usize].write_ansi(&mut output);
            }
        }
        output
    }
}

#[derive(Debug)]
pub struct CellChange {
    pub x: u16,
    pub y: u16,
    pub cell: Cell,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_new_creates_empty_cells() {
        let grid = Grid::new(10, 5);
        assert_eq!(grid.width, 10);
        assert_eq!(grid.height, 5);
        assert_eq!(grid.cells.len(), 5);
        assert_eq!(grid.cells[0].len(), 10);
        assert_eq!(grid.get(0, 0).ch, ' ');
    }

    #[test]
    fn grid_set_and_get() {
        let mut grid = Grid::new(10, 5);
        let cell = Cell::with_char('X');
        grid.set(3, 2, cell.clone());
        assert_eq!(grid.get(3, 2).ch, 'X');
    }

    #[test]
    fn grid_set_out_of_bounds_is_noop() {
        let mut grid = Grid::new(10, 5);
        grid.set(15, 2, Cell::with_char('X'));
        grid.set(3, 10, Cell::with_char('Y'));
    }

    #[test]
    fn grid_write_str() {
        let mut grid = Grid::new(20, 5);
        let style = CellStyle::default();
        grid.write_str(2, 1, "hello", &style);
        assert_eq!(grid.get(2, 1).ch, 'h');
        assert_eq!(grid.get(3, 1).ch, 'e');
        assert_eq!(grid.get(4, 1).ch, 'l');
        assert_eq!(grid.get(5, 1).ch, 'l');
        assert_eq!(grid.get(6, 1).ch, 'o');
    }

    #[test]
    fn grid_write_str_clips_at_boundary() {
        let mut grid = Grid::new(5, 1);
        let style = CellStyle::default();
        grid.write_str(3, 0, "hello", &style);
        assert_eq!(grid.get(3, 0).ch, 'h');
        assert_eq!(grid.get(4, 0).ch, 'e');
    }

    #[test]
    fn grid_fill_bg() {
        let mut grid = Grid::new(10, 5);
        grid.fill_bg(1, 1, 3, 2, Color::Red);
        assert_eq!(grid.get(1, 1).bg, Some(Color::Red));
        assert_eq!(grid.get(3, 2).bg, Some(Color::Red));
        assert_eq!(grid.get(0, 0).bg, None);
    }

    #[test]
    fn grid_diff_detects_changes() {
        let grid1 = Grid::new(5, 3);
        let mut grid2 = Grid::new(5, 3);
        grid2.set(2, 1, Cell::with_char('A'));

        let changes = grid1.diff(&grid2);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].x, 2);
        assert_eq!(changes[0].y, 1);
        assert_eq!(changes[0].cell.ch, 'A');
    }

    #[test]
    fn grid_diff_no_changes() {
        let grid1 = Grid::new(5, 3);
        let grid2 = Grid::new(5, 3);
        let changes = grid1.diff(&grid2);
        assert!(changes.is_empty());
    }

    #[test]
    fn cell_ansi_plain() {
        let cell = Cell::with_char('A');
        assert_eq!(cell.to_ansi(), "A");
    }

    #[test]
    fn cell_ansi_styled() {
        let cell = Cell {
            ch: 'B',
            bold: true,
            fg: Some(Color::Red),
            ..Default::default()
        };
        let ansi = cell.to_ansi();
        assert!(ansi.contains("\x1b["));
        assert!(ansi.contains("1"));
        assert!(ansi.contains('B'));
    }
}
