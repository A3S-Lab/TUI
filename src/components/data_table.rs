use crate::style::{truncate_visible, visible_len, Color, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellAlign {
    Left,
    Right,
    Center,
}

#[derive(Debug, Clone)]
pub struct DataColumn {
    header: String,
    header_suffix: String,
    width: Option<usize>,
    min_width: usize,
    align: CellAlign,
    visible: bool,
    priority: Option<u8>,
}

impl DataColumn {
    pub fn new(header: impl Into<String>) -> Self {
        Self {
            header: header.into(),
            header_suffix: String::new(),
            width: None,
            min_width: 3,
            align: CellAlign::Left,
            visible: true,
            priority: None,
        }
    }

    pub fn width(mut self, width: usize) -> Self {
        self.width = Some(width.max(1));
        self
    }

    pub fn min_width(mut self, width: usize) -> Self {
        self.min_width = width.max(1);
        self
    }

    pub fn align(mut self, align: CellAlign) -> Self {
        self.align = align;
        self
    }

    pub fn header_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.header_suffix = suffix.into();
        self
    }

    pub fn hidden(mut self) -> Self {
        self.visible = false;
        self
    }

    /// Set the responsive keep priority for this column.
    ///
    /// Columns with a lower priority are hidden first when the requested table
    /// width cannot fit. Columns without a priority keep the legacy behavior
    /// and are never hidden automatically.
    pub fn priority(mut self, priority: u8) -> Self {
        self.priority = Some(priority);
        self
    }

    fn header_label(&self) -> String {
        format!("{}{}", self.header, self.header_suffix)
    }
}

#[derive(Debug, Clone)]
pub struct DataRow {
    cells: Vec<String>,
    cell_fg: Vec<Option<Color>>,
    fg: Option<Color>,
    bg: Option<Color>,
    selected_fg: Option<Color>,
    selected_bg: Option<Color>,
    bold: bool,
}

impl DataRow {
    pub fn new(cells: Vec<impl Into<String>>) -> Self {
        let cells = cells.into_iter().map(Into::into).collect::<Vec<_>>();
        Self {
            cell_fg: vec![None; cells.len()],
            cells,
            fg: None,
            bg: None,
            selected_fg: Some(Color::Black),
            selected_bg: None,
            bold: false,
        }
    }

    pub fn fg(mut self, color: Color) -> Self {
        self.fg = Some(color);
        self
    }

    pub fn cell_fg(mut self, index: usize, color: Color) -> Self {
        if index >= self.cell_fg.len() {
            self.cell_fg.resize(index + 1, None);
        }
        self.cell_fg[index] = Some(color);
        self
    }

    pub fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    pub fn selected(mut self, fg: Color, bg: Color) -> Self {
        self.selected_fg = Some(fg);
        self.selected_bg = Some(bg);
        self
    }

    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }
}

#[derive(Debug, Clone)]
pub struct DataTable {
    columns: Vec<DataColumn>,
    rows: Vec<DataRow>,
    selected: Option<usize>,
    scroll: usize,
    gap: usize,
    header_fg: Color,
    separator_fg: Color,
    empty: Option<String>,
}

impl DataTable {
    pub fn new(columns: Vec<DataColumn>) -> Self {
        Self {
            columns,
            rows: Vec::new(),
            selected: None,
            scroll: 0,
            gap: 2,
            header_fg: Color::BrightWhite,
            separator_fg: Color::BrightBlack,
            empty: None,
        }
    }

    pub fn row(mut self, row: DataRow) -> Self {
        self.rows.push(row);
        self
    }

    pub fn add_row(&mut self, row: DataRow) {
        self.rows.push(row);
    }

    pub fn selected(mut self, selected: Option<usize>) -> Self {
        self.selected = selected;
        self
    }

    pub fn scroll(mut self, scroll: usize) -> Self {
        self.scroll = scroll;
        self
    }

    pub fn gap(mut self, gap: usize) -> Self {
        self.gap = gap;
        self
    }

    pub fn header_fg(mut self, color: Color) -> Self {
        self.header_fg = color;
        self
    }

    pub fn separator_fg(mut self, color: Color) -> Self {
        self.separator_fg = color;
        self
    }

    pub fn empty(mut self, message: impl Into<String>) -> Self {
        self.empty = Some(message.into());
        self
    }

    pub fn rows(&self) -> &[DataRow] {
        &self.rows
    }

    pub fn view(&self, width: u16, height: usize) -> String {
        let width = width as usize;
        if width == 0 || height == 0 {
            return String::new();
        }

        let cols = self.visible_columns_for_width(width);
        if cols.is_empty() {
            return String::new();
        }

        let widths = self.column_widths(&cols, width);
        let mut lines = Vec::new();
        let header = cols
            .iter()
            .zip(widths.iter())
            .map(|(idx, w)| {
                let column = &self.columns[*idx];
                format_cell(&column.header_label(), *w, column.align)
            })
            .collect::<Vec<_>>()
            .join(&" ".repeat(self.gap));
        lines.push(
            Style::new()
                .fg(self.header_fg)
                .bold()
                .render(&pad_or_truncate(&header, width)),
        );

        if height == 1 {
            return lines.join("\n");
        }

        let sep = widths
            .iter()
            .map(|w| "─".repeat(*w))
            .collect::<Vec<_>>()
            .join(&" ".repeat(self.gap));
        lines.push(
            Style::new()
                .fg(self.separator_fg)
                .render(&pad_or_truncate(&sep, width)),
        );

        if height == 2 {
            return lines.join("\n");
        }

        if self.rows.is_empty() {
            let msg = self.empty.as_deref().unwrap_or("no rows");
            lines.push(
                Style::new()
                    .fg(Color::BrightBlack)
                    .italic()
                    .render(&pad_or_truncate(msg, width)),
            );
            return lines.join("\n");
        }

        let body_height = height.saturating_sub(2);
        let start = self.scroll.min(
            self.rows
                .len()
                .saturating_sub(body_height.min(self.rows.len())),
        );
        for (idx, row) in self.rows.iter().enumerate().skip(start).take(body_height) {
            let selected = self.selected == Some(idx);
            let raw = self.row_line(row, &cols, &widths, selected);
            let line = pad_or_truncate(&raw, width);
            let mut style = Style::new();
            if selected {
                if let Some(fg) = row.selected_fg {
                    style = style.fg(fg);
                }
                style = style.bg(row.selected_bg.or(row.fg).unwrap_or(Color::Blue));
                style = style.bold();
            } else {
                if let Some(fg) = row.fg {
                    style = style.fg(fg);
                }
                if let Some(bg) = row.bg {
                    style = style.bg(bg);
                }
                if row.bold {
                    style = style.bold();
                }
            }
            lines.push(style.render(&line));
        }

        lines.join("\n")
    }

    fn row_line(&self, row: &DataRow, cols: &[usize], widths: &[usize], selected: bool) -> String {
        cols.iter()
            .zip(widths.iter())
            .map(|(col_idx, w)| {
                let column = &self.columns[*col_idx];
                let cell = row.cells.get(*col_idx).map_or("", String::as_str);
                let formatted = format_cell(cell, *w, column.align);
                if selected {
                    formatted
                } else if let Some(color) = row.cell_fg.get(*col_idx).copied().flatten() {
                    Style::new().fg(color).render(&formatted)
                } else {
                    formatted
                }
            })
            .collect::<Vec<_>>()
            .join(&" ".repeat(self.gap))
    }

    fn visible_columns(&self) -> Vec<usize> {
        self.columns
            .iter()
            .enumerate()
            .filter_map(|(idx, col)| col.visible.then_some(idx))
            .collect()
    }

    fn visible_columns_for_width(&self, width: usize) -> Vec<usize> {
        let mut cols = self.visible_columns();
        if cols.len() <= 1 || !cols.iter().any(|idx| self.columns[*idx].priority.is_some()) {
            return cols;
        }

        while cols.len() > 1 && self.requested_total_width(&cols) > width {
            let Some(lowest_priority) = cols
                .iter()
                .filter_map(|idx| self.columns[*idx].priority)
                .min()
            else {
                break;
            };
            let Some(remove_idx) = cols
                .iter()
                .rposition(|idx| self.columns[*idx].priority == Some(lowest_priority))
            else {
                break;
            };
            cols.remove(remove_idx);
        }

        cols
    }

    fn requested_total_width(&self, cols: &[usize]) -> usize {
        let gap_total = self.gap.saturating_mul(cols.len().saturating_sub(1));
        gap_total
            + cols
                .iter()
                .map(|idx| self.requested_column_width(*idx))
                .sum::<usize>()
    }

    fn requested_column_width(&self, idx: usize) -> usize {
        let col = &self.columns[idx];
        col.width
            .unwrap_or_else(|| {
                let mut w = visible_len(&col.header_label()).max(col.min_width);
                for row in &self.rows {
                    if let Some(cell) = row.cells.get(idx) {
                        w = w.max(visible_len(cell));
                    }
                }
                w
            })
            .max(col.min_width)
    }

    fn column_widths(&self, cols: &[usize], width: usize) -> Vec<usize> {
        let gap_total = self.gap.saturating_mul(cols.len().saturating_sub(1));
        let available = width.saturating_sub(gap_total).max(cols.len());
        let mut widths = cols
            .iter()
            .map(|idx| self.requested_column_width(*idx))
            .collect::<Vec<_>>();

        while widths.iter().sum::<usize>() > available {
            let Some((idx, _)) = widths
                .iter()
                .enumerate()
                .filter(|(i, w)| **w > self.columns[cols[*i]].min_width)
                .max_by_key(|(_, w)| **w)
            else {
                break;
            };
            widths[idx] -= 1;
        }

        while widths.iter().sum::<usize>() > available {
            let Some((idx, _)) = widths.iter().enumerate().max_by_key(|(_, w)| **w) else {
                break;
            };
            if widths[idx] <= 1 {
                break;
            }
            widths[idx] -= 1;
        }

        widths
    }
}

fn format_cell(value: &str, width: usize, align: CellAlign) -> String {
    let truncated = truncate_to_width(value, width);
    let len = visible_len(&truncated);
    if len >= width {
        return truncated;
    }
    let pad = width - len;
    match align {
        CellAlign::Left => format!("{truncated}{}", " ".repeat(pad)),
        CellAlign::Right => format!("{}{truncated}", " ".repeat(pad)),
        CellAlign::Center => {
            let left = pad / 2;
            format!(
                "{}{}{}",
                " ".repeat(left),
                truncated,
                " ".repeat(pad - left)
            )
        }
    }
}

fn pad_or_truncate(value: &str, width: usize) -> String {
    let truncated = truncate_to_width(value, width);
    let len = visible_len(&truncated);
    if len >= width {
        truncated
    } else {
        format!("{truncated}{}", " ".repeat(width - len))
    }
}

fn truncate_to_width(value: &str, width: usize) -> String {
    truncate_visible(value, width)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::strip_ansi;

    #[test]
    fn renders_header_separator_and_rows() {
        let table = DataTable::new(vec![DataColumn::new("Name"), DataColumn::new("CPU")])
            .row(DataRow::new(vec!["codex", "12.4"]))
            .row(DataRow::new(vec!["a3s", "1.0"]));

        let plain = strip_ansi(&table.view(40, 10));

        assert!(plain.contains("Name"));
        assert!(plain.contains("CPU"));
        assert!(plain.contains("codex"));
        assert!(plain.contains("a3s"));
    }

    #[test]
    fn right_aligns_numeric_cells() {
        let table = DataTable::new(vec![
            DataColumn::new("PID"),
            DataColumn::new("CPU").width(5).align(CellAlign::Right),
        ])
        .row(DataRow::new(vec!["1", "9.1"]));

        let plain = strip_ansi(&table.view(24, 4));

        assert!(plain.contains("  9.1"));
    }

    #[test]
    fn aligns_headers_with_cells() {
        let table = DataTable::new(vec![
            DataColumn::new("NAME").width(6),
            DataColumn::new("CPU").width(5).align(CellAlign::Right),
        ])
        .row(DataRow::new(vec!["box", "9.1"]));

        let plain = strip_ansi(&table.view(20, 4));
        let header = plain.lines().next().unwrap();

        assert!(header.contains("NAME      CPU"));
    }

    #[test]
    fn renders_header_suffixes() {
        let table = DataTable::new(vec![
            DataColumn::new("NAME"),
            DataColumn::new("CPU").header_suffix("↓"),
        ])
        .row(DataRow::new(vec!["box", "12.5"]));

        let plain = strip_ansi(&table.view(24, 4));
        let header = plain.lines().next().unwrap();

        assert!(header.contains("CPU↓"));
    }

    #[test]
    fn truncates_to_requested_width() {
        let table = DataTable::new(vec![DataColumn::new("Command").min_width(4)])
            .row(DataRow::new(vec!["a very long command line"]));

        for line in strip_ansi(&table.view(12, 4)).lines() {
            assert!(visible_len(line) <= 12, "{line:?}");
        }
    }

    #[test]
    fn scrolls_body_rows_but_keeps_header() {
        let table = DataTable::new(vec![DataColumn::new("Name")])
            .row(DataRow::new(vec!["one"]))
            .row(DataRow::new(vec!["two"]))
            .row(DataRow::new(vec!["three"]))
            .scroll(1);

        let plain = strip_ansi(&table.view(20, 4));

        assert!(plain.contains("Name"));
        assert!(!plain.contains("one"));
        assert!(plain.contains("two"));
    }

    #[test]
    fn hides_columns() {
        let table = DataTable::new(vec![DataColumn::new("A"), DataColumn::new("B").hidden()])
            .row(DataRow::new(vec!["visible", "secret"]));

        let plain = strip_ansi(&table.view(30, 4));

        assert!(plain.contains("visible"));
        assert!(!plain.contains("secret"));
    }

    #[test]
    fn hides_lower_priority_columns_when_width_is_tight() {
        let table = DataTable::new(vec![
            DataColumn::new("KEEP").width(6).priority(100),
            DataColumn::new("DROP").width(8).priority(10),
            DataColumn::new("ALSO").width(6).priority(90),
        ])
        .row(DataRow::new(vec!["alpha", "hidden", "omega"]));

        let plain = strip_ansi(&table.view(16, 4));

        assert!(plain.contains("KEEP"));
        assert!(plain.contains("ALSO"));
        assert!(plain.contains("alpha"));
        assert!(plain.contains("omega"));
        assert!(!plain.contains("DROP"));
        assert!(!plain.contains("hidden"));
    }

    #[test]
    fn leaves_unprioritized_columns_visible_for_legacy_tables() {
        let table = DataTable::new(vec![
            DataColumn::new("LEFT").width(10),
            DataColumn::new("RIGHT").width(10),
        ])
        .row(DataRow::new(vec!["left-value", "right-value"]));

        let plain = strip_ansi(&table.view(14, 4));

        assert!(plain.contains("LEFT"));
        assert!(plain.contains("RIG"));
    }

    #[test]
    fn selected_row_uses_row_color_as_default_background() {
        let table = DataTable::new(vec![DataColumn::new("Name")])
            .row(DataRow::new(vec!["agent"]).fg(Color::Green))
            .selected(Some(0));

        let rendered = table.view(20, 4);

        assert!(rendered.contains("\u{1b}["));
    }

    #[test]
    fn styles_individual_cells_without_changing_width() {
        let table = DataTable::new(vec![
            DataColumn::new("STATE").width(8),
            DataColumn::new("NAME").width(8),
        ])
        .row(DataRow::new(vec!["running", "api"]).cell_fg(0, Color::Green));

        let rendered = table.view(24, 4);
        let plain = strip_ansi(&rendered);
        let row = plain.lines().nth(2).unwrap();

        assert!(rendered.contains("\x1b[32m"));
        assert!(row.contains("running"));
        assert_eq!(visible_len(row), 24);
    }

    #[test]
    fn selected_rows_override_individual_cell_colors() {
        let table = DataTable::new(vec![DataColumn::new("STATE").width(8)])
            .row(DataRow::new(vec!["dead"]).cell_fg(0, Color::Red))
            .selected(Some(0));

        let rendered = table.view(16, 4);

        assert!(!rendered.contains("\x1b[31m"));
    }

    #[test]
    fn truncates_styled_cells_without_splitting_ansi() {
        let cell = Style::new().fg(Color::Cyan).render("abcdefghijabcdefghij");
        let table = DataTable::new(vec![
            DataColumn::new("Trend").width(20),
            DataColumn::new("Name").width(20),
        ])
        .row(DataRow::new(vec![cell, "long-process-name".to_string()]));

        for line in table.view(18, 4).lines() {
            assert!(!line.contains("\x1b[…"), "{line:?}");
            assert!(visible_len(line) <= 18, "{line:?}");
        }
    }
}
