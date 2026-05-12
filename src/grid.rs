use crate::style::Color;

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
        let mut codes = Vec::new();
        if self.bold { codes.push("1".to_string()); }
        if self.dim { codes.push("2".to_string()); }
        if self.italic { codes.push("3".to_string()); }
        if self.underline { codes.push("4".to_string()); }
        if self.strikethrough { codes.push("9".to_string()); }
        if let Some(ref c) = self.fg { codes.push(c.fg_ansi()); }
        if let Some(ref c) = self.bg { codes.push(c.bg_ansi()); }

        if codes.is_empty() {
            self.ch.to_string()
        } else {
            format!("\x1b[{}m{}\x1b[0m", codes.join(";"), self.ch)
        }
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
}

#[derive(Debug)]
pub struct CellChange {
    pub x: u16,
    pub y: u16,
    pub cell: Cell,
}
