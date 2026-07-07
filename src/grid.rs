//! Cell grid for terminal rendering.
//!
//! The grid is a 2D array of [`Cell`]s representing the terminal screen buffer.
//! Each cell holds a character and its styling attributes.

use crate::style::Color;

/// A single terminal cell with character and style.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Cell {
    pub ch: char,
    pub combining: String,
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub reverse: bool,
    pub dim: bool,
    pub strikethrough: bool,
}

static EMPTY_CELL: Cell = Cell {
    ch: ' ',
    combining: String::new(),
    fg: None,
    bg: None,
    bold: false,
    italic: false,
    underline: false,
    reverse: false,
    dim: false,
    strikethrough: false,
};

const WIDE_CONTINUATION: char = '\0';

impl Default for Cell {
    fn default() -> Self {
        EMPTY_CELL.clone()
    }
}

impl Cell {
    pub fn with_char(ch: char) -> Self {
        Self {
            ch,
            ..Default::default()
        }
    }

    pub fn styled(ch: char, style: &CellStyle) -> Self {
        Self {
            ch,
            combining: String::new(),
            fg: style.fg,
            bg: style.bg,
            bold: style.bold,
            italic: style.italic,
            underline: style.underline,
            reverse: style.reverse,
            dim: style.dim,
            strikethrough: style.strikethrough,
        }
    }

    pub fn to_ansi(&self) -> String {
        if self.ch == WIDE_CONTINUATION {
            return String::new();
        }

        use std::fmt::Write;

        let has_style = self.bold
            || self.dim
            || self.italic
            || self.underline
            || self.reverse
            || self.strikethrough
            || self.fg.is_some()
            || self.bg.is_some();

        if !has_style {
            let mut out = self.ch.to_string();
            out.push_str(&self.combining);
            return out;
        }

        let mut out = String::with_capacity(24);
        out.push_str("\x1b[");
        let mut first = true;

        macro_rules! push_code {
            ($code:expr) => {
                if !first {
                    out.push(';');
                }
                let _ = write!(out, "{}", $code);
                #[allow(unused_assignments)]
                {
                    first = false;
                }
            };
        }

        if self.bold {
            push_code!("1");
        }
        if self.dim {
            push_code!("2");
        }
        if self.italic {
            push_code!("3");
        }
        if self.underline {
            push_code!("4");
        }
        if self.reverse {
            push_code!("7");
        }
        if self.strikethrough {
            push_code!("9");
        }
        if let Some(ref c) = self.fg {
            push_code!(c.fg_ansi());
        }
        if let Some(ref c) = self.bg {
            push_code!(c.bg_ansi());
        }

        out.push('m');
        out.push(self.ch);
        out.push_str(&self.combining);
        out.push_str("\x1b[0m");
        out
    }

    /// Write ANSI representation into an existing buffer (avoids allocation).
    pub fn write_ansi(&self, buf: &mut String) {
        if self.ch == WIDE_CONTINUATION {
            return;
        }

        use std::fmt::Write;

        let has_style = self.bold
            || self.dim
            || self.italic
            || self.underline
            || self.reverse
            || self.strikethrough
            || self.fg.is_some()
            || self.bg.is_some();

        if !has_style {
            buf.push(self.ch);
            buf.push_str(&self.combining);
            return;
        }

        buf.push_str("\x1b[");
        let mut first = true;

        macro_rules! push_code {
            ($code:expr) => {
                if !first {
                    buf.push(';');
                }
                let _ = write!(buf, "{}", $code);
                #[allow(unused_assignments)]
                {
                    first = false;
                }
            };
        }

        if self.bold {
            push_code!("1");
        }
        if self.dim {
            push_code!("2");
        }
        if self.italic {
            push_code!("3");
        }
        if self.underline {
            push_code!("4");
        }
        if self.reverse {
            push_code!("7");
        }
        if self.strikethrough {
            push_code!("9");
        }
        if let Some(ref c) = self.fg {
            push_code!(c.fg_ansi());
        }
        if let Some(ref c) = self.bg {
            push_code!(c.bg_ansi());
        }

        buf.push('m');
        buf.push(self.ch);
        buf.push_str(&self.combining);
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
    pub reverse: bool,
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
        Self {
            cells,
            width,
            height,
        }
    }

    pub fn get(&self, x: u16, y: u16) -> &Cell {
        self.try_get(x, y).unwrap_or(&EMPTY_CELL)
    }

    pub fn try_get(&self, x: u16, y: u16) -> Option<&Cell> {
        self.cells
            .get(y as usize)
            .and_then(|row| row.get(x as usize))
    }

    pub fn set(&mut self, x: u16, y: u16, cell: Cell) {
        if x < self.width && y < self.height {
            let row = y as usize;
            let col = x as usize;
            let width = unicode_width::UnicodeWidthChar::width(cell.ch).unwrap_or(1);
            if width == 0 || col.saturating_add(width) > self.width as usize {
                return;
            }
            for target_col in col..col.saturating_add(width).max(col + 1) {
                self.clear_wide_span_at(row, target_col);
            }
            self.cells[row][col] = cell.clone();
            for offset in 1..width {
                let mut continuation = cell.clone();
                continuation.ch = WIDE_CONTINUATION;
                continuation.combining.clear();
                self.cells[row][col + offset] = continuation;
            }
        }
    }

    pub fn write_str(&mut self, x: u16, y: u16, text: &str, style: &CellStyle) {
        let mut col = x as usize;
        let row = y as usize;
        if row >= self.height as usize {
            return;
        }
        let mut last_base_col: Option<usize> = None;
        for ch in text.chars() {
            let width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
            if width == 0 {
                if let Some(base_col) =
                    last_base_col.or_else(|| self.leading_combining_base_col(row, col))
                {
                    self.cells[row][base_col].combining.push(ch);
                }
                continue;
            }
            if col.saturating_add(width) > self.width as usize {
                break;
            }
            for target_col in col..col.saturating_add(width).max(col + 1) {
                self.clear_wide_span_at(row, target_col);
            }
            self.cells[row][col] = Cell::styled(ch, style);
            for offset in 1..width {
                self.cells[row][col + offset] = Cell::styled(WIDE_CONTINUATION, style);
            }
            last_base_col = Some(col);
            col += width;
        }
    }

    fn leading_combining_base_col(&self, row: usize, col: usize) -> Option<usize> {
        if col == 0 || col >= self.width as usize {
            return None;
        }

        let (start, _) = self.wide_span_bounds_at(row, col - 1);
        let cell = self.cells.get(row)?.get(start)?;
        if cell.ch == ' ' || cell.ch == WIDE_CONTINUATION {
            return None;
        }

        Some(start)
    }

    fn clear_wide_span_at(&mut self, row: usize, col: usize) {
        if row >= self.height as usize || col >= self.width as usize {
            return;
        }

        let row_cells = &mut self.cells[row];
        let mut start = col;
        if row_cells[col].ch == WIDE_CONTINUATION {
            while start > 0 && row_cells[start].ch == WIDE_CONTINUATION {
                start -= 1;
            }
        }

        let width = unicode_width::UnicodeWidthChar::width(row_cells[start].ch).unwrap_or(1);
        if width <= 1 || start.saturating_add(width) <= col {
            row_cells[col] = Cell::default();
            return;
        }

        let end = start.saturating_add(width).min(row_cells.len());
        for cell in row_cells.iter_mut().take(end).skip(start) {
            *cell = Cell::default();
        }
    }

    fn wide_span_bounds_at(&self, row: usize, col: usize) -> (usize, usize) {
        let Some(row_cells) = self.cells.get(row) else {
            return (col, col);
        };
        if col >= row_cells.len() {
            return (col, col);
        }

        let mut start = col;
        if row_cells[col].ch == WIDE_CONTINUATION {
            while start > 0 && row_cells[start].ch == WIDE_CONTINUATION {
                start -= 1;
            }
        }

        let width = unicode_width::UnicodeWidthChar::width(row_cells[start].ch).unwrap_or(1);
        if width <= 1 || start.saturating_add(width) <= col {
            return (col, col.saturating_add(1).min(row_cells.len()));
        }

        (start, start.saturating_add(width).min(row_cells.len()))
    }

    pub fn fill_bg(&mut self, x: u16, y: u16, w: u16, h: u16, color: Color) {
        let end_y = y.saturating_add(h).min(self.height);
        let end_x = x.saturating_add(w).min(self.width);
        for row in y as usize..end_y as usize {
            let mut col = x as usize;
            while col < end_x as usize {
                let (span_start, span_end) = self.wide_span_bounds_at(row, col);
                for target_col in span_start..span_end {
                    self.cells[row][target_col].bg = Some(color);
                }
                col = span_end.max(col.saturating_add(1));
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
                    changes.push(CellChange {
                        x,
                        y,
                        cell: new.clone(),
                    });
                }
            }

            if other.width > self.width {
                for x in self.width..other.width {
                    let cell = &other.cells[y as usize][x as usize];
                    if *cell != Cell::default() {
                        changes.push(CellChange {
                            x,
                            y,
                            cell: cell.clone(),
                        });
                    }
                }
            }
        }

        if other.height > self.height {
            for y in self.height..other.height {
                for x in 0..other.width {
                    let cell = &other.cells[y as usize][x as usize];
                    if *cell != Cell::default() {
                        changes.push(CellChange {
                            x,
                            y,
                            cell: cell.clone(),
                        });
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
        assert_eq!(grid.try_get(3, 2), Some(&cell));
    }

    #[test]
    fn grid_get_out_of_bounds_returns_empty_cell() {
        let grid = Grid::new(2, 1);
        assert!(grid.try_get(2, 0).is_none());
        assert!(grid.try_get(0, 1).is_none());
        assert_eq!(grid.get(2, 0), &Cell::default());
        assert_eq!(grid.get(0, 1), &Cell::default());
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
    fn grid_write_str_drops_wide_char_that_would_overflow() {
        let mut grid = Grid::new(4, 1);
        let style = CellStyle::default();

        grid.write_str(3, 0, "界", &style);

        assert_eq!(grid.get(3, 0).ch, ' ');
    }

    #[test]
    fn grid_write_str_keeps_zero_width_marks_with_base_glyph() {
        let mut grid = Grid::new(2, 1);
        let style = CellStyle::default();

        grid.write_str(0, 0, "ab\u{0301}", &style);
        grid.write_str(2, 0, "\u{0301}", &style);

        assert_eq!(grid.get(1, 0).ch, 'b');
        assert_eq!(grid.get(1, 0).combining, "\u{0301}");
        assert_eq!(grid.render_to_string(), "ab\u{0301}");
        assert_eq!(crate::style::visible_len(&grid.render_to_string()), 2);
    }

    #[test]
    fn grid_write_str_attaches_leading_zero_width_mark_to_previous_cell() {
        let mut grid = Grid::new(2, 1);
        let style = CellStyle::default();

        grid.write_str(0, 0, "a", &style);
        grid.write_str(1, 0, "\u{0301}", &style);

        assert_eq!(grid.get(0, 0).ch, 'a');
        assert_eq!(grid.get(0, 0).combining, "\u{0301}");
        assert_eq!(grid.render_to_string(), "a\u{0301} ");
    }

    #[test]
    fn grid_write_str_attaches_leading_zero_width_mark_to_previous_wide_cell() {
        let mut grid = Grid::new(3, 1);
        let style = CellStyle::default();

        grid.write_str(0, 0, "界", &style);
        grid.write_str(2, 0, "\u{0301}", &style);

        assert_eq!(grid.get(0, 0).ch, '界');
        assert_eq!(grid.get(0, 0).combining, "\u{0301}");
        assert_eq!(grid.get(1, 0).ch, WIDE_CONTINUATION);
        assert_eq!(crate::style::visible_len(&grid.render_to_string()), 3);
    }

    #[test]
    fn grid_render_does_not_add_visible_space_after_wide_char() {
        let mut grid = Grid::new(2, 1);
        let style = CellStyle::default();

        grid.write_str(0, 0, "界", &style);

        assert_eq!(crate::style::visible_len(&grid.render_to_string()), 2);
    }

    #[test]
    fn grid_write_str_clears_stale_wide_continuation_after_narrow_overwrite() {
        let mut grid = Grid::new(2, 1);
        let style = CellStyle::default();

        grid.write_str(0, 0, "界", &style);
        grid.write_str(0, 0, "A", &style);

        assert_eq!(grid.render_to_string(), "A ");
    }

    #[test]
    fn grid_write_str_clears_wide_char_when_overwriting_its_continuation() {
        let mut grid = Grid::new(2, 1);
        let style = CellStyle::default();

        grid.write_str(0, 0, "界", &style);
        grid.write_str(1, 0, "B", &style);

        assert_eq!(grid.render_to_string(), " B");
    }

    #[test]
    fn grid_set_clears_stale_wide_continuation_after_narrow_overwrite() {
        let mut grid = Grid::new(2, 1);
        let style = CellStyle::default();

        grid.write_str(0, 0, "界", &style);
        grid.set(0, 0, Cell::with_char('A'));

        assert_eq!(grid.render_to_string(), "A ");
    }

    #[test]
    fn grid_set_clears_wide_char_when_overwriting_its_continuation() {
        let mut grid = Grid::new(2, 1);
        let style = CellStyle::default();

        grid.write_str(0, 0, "界", &style);
        grid.set(1, 0, Cell::with_char('B'));

        assert_eq!(grid.render_to_string(), " B");
    }

    #[test]
    fn grid_set_marks_wide_char_continuation() {
        let mut grid = Grid::new(2, 1);

        grid.set(0, 0, Cell::with_char('界'));

        assert_eq!(grid.get(0, 0).ch, '界');
        assert_eq!(grid.get(1, 0).ch, WIDE_CONTINUATION);
        assert_eq!(crate::style::visible_len(&grid.render_to_string()), 2);
    }

    #[test]
    fn grid_set_drops_wide_char_that_would_overflow() {
        let mut grid = Grid::new(2, 1);

        grid.set(1, 0, Cell::with_char('界'));

        assert_eq!(grid.render_to_string(), "  ");
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
    fn grid_fill_bg_styles_wide_span_when_continuation_is_covered() {
        let mut grid = Grid::new(2, 1);
        let style = CellStyle::default();

        grid.write_str(0, 0, "界", &style);
        grid.fill_bg(1, 0, 1, 1, Color::Blue);

        assert_eq!(grid.get(0, 0).bg, Some(Color::Blue));
        assert_eq!(grid.get(1, 0).bg, Some(Color::Blue));
        assert!(grid.render_to_string().contains("\x1b["));
    }

    #[test]
    fn grid_fill_bg_saturates_overflowing_bounds() {
        let mut grid = Grid::new(4, 2);
        grid.fill_bg(u16::MAX - 1, u16::MAX - 1, 10, 10, Color::Red);
        assert!(grid.cells.iter().flatten().all(|cell| cell.bg.is_none()));

        grid.fill_bg(2, 1, u16::MAX, u16::MAX, Color::Blue);
        assert_eq!(grid.get(2, 1).bg, Some(Color::Blue));
        assert_eq!(grid.get(3, 1).bg, Some(Color::Blue));
        assert_eq!(grid.get(1, 1).bg, None);
        assert_eq!(grid.get(2, 0).bg, None);
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
    fn grid_diff_detects_non_empty_cells_in_new_columns() {
        let grid1 = Grid::new(1, 2);
        let mut grid2 = Grid::new(3, 2);
        grid2.set(2, 1, Cell::with_char('B'));

        let changes = grid1.diff(&grid2);

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].x, 2);
        assert_eq!(changes[0].y, 1);
        assert_eq!(changes[0].cell.ch, 'B');
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
            reverse: true,
            fg: Some(Color::Red),
            ..Default::default()
        };
        let ansi = cell.to_ansi();
        assert!(ansi.contains("\x1b["));
        assert!(ansi.contains("1"));
        assert!(ansi.contains("7"));
        assert!(ansi.contains('B'));
    }
}
