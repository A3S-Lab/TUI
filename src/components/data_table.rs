use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::event::{MouseButton, MouseEvent, MouseEventKind};
use crate::interaction::{Scrollable, Selectable};
use crate::style::{center_visible, fit_visible, right_visible, visible_len, Color, Style};
use crate::theme::{Theme, ThemeRole};

const MAX_DATA_COLUMN_WIDTH: usize = u16::MAX as usize;
const MAX_DATA_ROW_CELL_STYLES: usize = u16::MAX as usize;
const MAX_DATA_TABLE_GAP: usize = u16::MAX as usize;

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
        self.width = Some(width.clamp(1, MAX_DATA_COLUMN_WIDTH));
        self
    }

    pub fn min_width(mut self, width: usize) -> Self {
        self.min_width = width.clamp(1, MAX_DATA_COLUMN_WIDTH);
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
        let index = index.min(MAX_DATA_ROW_CELL_STYLES.saturating_sub(1));
        if index >= self.cell_fg.len() {
            self.cell_fg.resize(index.saturating_add(1), None);
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
    y_offset: u16,
    gap: usize,
    header_fg: Color,
    separator_fg: Color,
    empty: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataTableMsg {
    Selected(usize),
}

impl DataTable {
    pub fn new(columns: Vec<DataColumn>) -> Self {
        Self {
            columns,
            rows: Vec::new(),
            selected: None,
            scroll: 0,
            y_offset: 0,
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
        self.gap = gap.min(MAX_DATA_TABLE_GAP);
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

    /// Apply semantic colors from a theme while preserving rows and layout.
    pub fn with_theme(mut self, theme: &Theme) -> Self {
        self.header_fg = theme.color(ThemeRole::Foreground);
        self.separator_fg = theme.color(ThemeRole::Border);
        self
    }

    pub fn rows(&self) -> &[DataRow] {
        &self.rows
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.normalized_selected()
    }

    pub fn scroll_offset(&self) -> usize {
        self.scroll
    }

    pub fn set_y_offset(&mut self, y_offset: u16) {
        self.y_offset = y_offset;
    }

    pub fn handle_mouse(&mut self, mouse: &MouseEvent, height: usize) -> Option<DataTableMsg> {
        let local_row = super::relative_mouse_row(mouse.row, self.y_offset)?;
        if height == 0 || local_row >= height || self.rows.is_empty() {
            return None;
        }

        let body_height = height.saturating_sub(2);
        if body_height == 0 {
            return None;
        }

        match mouse.kind {
            MouseEventKind::ScrollUp => {
                let selected = self.normalized_selected().unwrap_or(0);
                self.selected = Some(selected.saturating_sub(1));
                self.keep_selected_visible(body_height);
                None
            }
            MouseEventKind::ScrollDown => {
                let selected = self.normalized_selected().unwrap_or(0);
                self.selected = Some(
                    selected
                        .saturating_add(1)
                        .min(self.rows.len().saturating_sub(1)),
                );
                self.keep_selected_visible(body_height);
                None
            }
            MouseEventKind::Down(MouseButton::Left) => {
                let body_row = local_row.checked_sub(2)?;
                if body_row >= body_height {
                    return None;
                }
                let index = self
                    .visible_body_start(body_height)
                    .saturating_add(body_row);
                if index < self.rows.len() {
                    self.selected = Some(index);
                    self.keep_selected_visible(body_height);
                    Some(DataTableMsg::Selected(index))
                } else {
                    None
                }
            }
            _ => None,
        }
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
        let gap = self.gap_for_width(width, cols.len());
        let gap_text = " ".repeat(gap);
        let mut lines = Vec::new();
        let header = cols
            .iter()
            .zip(widths.iter())
            .map(|(idx, w)| {
                let column = &self.columns[*idx];
                format_cell(&column.header_label(), *w, column.align)
            })
            .collect::<Vec<_>>()
            .join(&gap_text);
        lines.push(
            Style::new()
                .fg(self.header_fg)
                .bold()
                .render(&fit_visible(&header, width)),
        );

        if height == 1 {
            return lines.join("\n");
        }

        let sep = widths
            .iter()
            .map(|w| "─".repeat(*w))
            .collect::<Vec<_>>()
            .join(&gap_text);
        lines.push(
            Style::new()
                .fg(self.separator_fg)
                .render(&fit_visible(&sep, width)),
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
                    .render(&fit_visible(msg, width)),
            );
            return lines.join("\n");
        }

        let body_height = height.saturating_sub(2);
        let start = self.visible_body_start(body_height);
        let selected_row = self.normalized_selected();
        for (idx, row) in self.rows.iter().enumerate().skip(start).take(body_height) {
            let selected = selected_row == Some(idx);
            let raw = self.row_line(row, &cols, &widths, selected, &gap_text);
            let line = fit_visible(&raw, width);
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

    pub fn element<Msg>(&self, width: u16, height: usize) -> Element<Msg> {
        let width = width as usize;
        let mut children = Vec::new();
        if width == 0 || height == 0 {
            return data_table_column(children);
        }

        let cols = self.visible_columns_for_width(width);
        if cols.is_empty() {
            return data_table_column(children);
        }

        let widths = self.column_widths(&cols, width);
        let gap = self.gap_for_width(width, cols.len());
        let gap_text = " ".repeat(gap);
        let header = cols
            .iter()
            .zip(widths.iter())
            .map(|(idx, w)| {
                let column = &self.columns[*idx];
                format_cell(&column.header_label(), *w, column.align)
            })
            .collect::<Vec<_>>()
            .join(&gap_text);
        children.push(Element::Text(
            TextElement::new(fit_visible(&header, width))
                .fg(self.header_fg)
                .bold(),
        ));

        if height == 1 {
            return data_table_column(children);
        }

        let sep = widths
            .iter()
            .map(|w| "─".repeat(*w))
            .collect::<Vec<_>>()
            .join(&gap_text);
        children.push(Element::Text(
            TextElement::new(fit_visible(&sep, width)).fg(self.separator_fg),
        ));

        if height == 2 {
            return data_table_column(children);
        }

        if self.rows.is_empty() {
            let msg = self.empty.as_deref().unwrap_or("no rows");
            children.push(Element::Text(
                TextElement::new(fit_visible(msg, width))
                    .fg(Color::BrightBlack)
                    .italic(),
            ));
            return data_table_column(children);
        }

        let body_height = height.saturating_sub(2);
        let start = self.visible_body_start(body_height);
        let selected_row = self.normalized_selected();
        for (idx, row) in self.rows.iter().enumerate().skip(start).take(body_height) {
            let selected = selected_row == Some(idx);
            let raw = self.plain_row_line(row, &cols, &widths, &gap_text);
            let mut text = TextElement::new(fit_visible(&raw, width));
            if selected {
                if let Some(fg) = row.selected_fg {
                    text = text.fg(fg);
                }
                text = text.bg(row.selected_bg.or(row.fg).unwrap_or(Color::Blue));
                text = text.bold();
            } else {
                if let Some(fg) = row.fg {
                    text = text.fg(fg);
                }
                if let Some(bg) = row.bg {
                    text = text.bg(bg);
                }
                if row.bold {
                    text = text.bold();
                }
            }
            children.push(Element::Text(text));
        }

        data_table_column(children)
    }

    fn row_line(
        &self,
        row: &DataRow,
        cols: &[usize],
        widths: &[usize],
        selected: bool,
        gap: &str,
    ) -> String {
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
            .join(gap)
    }

    fn plain_row_line(&self, row: &DataRow, cols: &[usize], widths: &[usize], gap: &str) -> String {
        cols.iter()
            .zip(widths.iter())
            .map(|(col_idx, w)| {
                let column = &self.columns[*col_idx];
                let cell = row.cells.get(*col_idx).map_or("", String::as_str);
                format_cell(cell, *w, column.align)
            })
            .collect::<Vec<_>>()
            .join(gap)
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

        while cols.len() > 1 && self.requested_total_width(&cols, width) > width {
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

    fn requested_total_width(&self, cols: &[usize], width: usize) -> usize {
        cols.iter()
            .map(|idx| self.requested_column_width(*idx))
            .fold(
                self.gap_total_for_width(width, cols.len()),
                usize::saturating_add,
            )
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
        let gap_total = self.gap_total_for_width(width, cols.len());
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

    fn gap_for_width(&self, width: usize, column_count: usize) -> usize {
        let separators = column_count.saturating_sub(1);
        if separators == 0 {
            return 0;
        }

        let max_gap = width.saturating_sub(column_count) / separators;
        self.gap.min(max_gap).min(MAX_DATA_TABLE_GAP)
    }

    fn gap_total_for_width(&self, width: usize, column_count: usize) -> usize {
        self.gap_for_width(width, column_count)
            .saturating_mul(column_count.saturating_sub(1))
    }

    fn visible_body_start(&self, body_height: usize) -> usize {
        if body_height == 0 || self.rows.is_empty() {
            return 0;
        }

        let max_start = self
            .rows
            .len()
            .saturating_sub(body_height.min(self.rows.len()));
        let mut start = self.scroll.min(max_start);
        if let Some(selected) = self.normalized_selected() {
            if selected < start {
                start = selected;
            } else if selected >= start.saturating_add(body_height) {
                start = selected.saturating_add(1).saturating_sub(body_height);
            }
        }

        start.min(max_start)
    }

    fn keep_selected_visible(&mut self, body_height: usize) {
        self.scroll = self.visible_body_start(body_height);
    }

    fn normalized_selected(&self) -> Option<usize> {
        self.selected
            .map(|selected| selected.min(self.rows.len().saturating_sub(1)))
            .filter(|_| !self.rows.is_empty())
    }
}

impl Selectable for DataTable {
    fn item_count(&self) -> usize {
        self.rows.len()
    }

    fn selected_index(&self) -> Option<usize> {
        self.normalized_selected()
    }

    fn select_index(&mut self, index: usize) {
        self.selected = (!self.rows.is_empty()).then(|| index.min(self.rows.len() - 1));
    }
}

impl Scrollable for DataTable {
    fn scroll_offset(&self) -> usize {
        self.scroll
    }

    fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll = offset.min(self.rows.len().saturating_sub(1));
    }
}

fn data_table_column<Msg>(children: Vec<Element<Msg>>) -> Element<Msg> {
    Element::Box(
        BoxElement::new()
            .direction(FlexDirection::Column)
            .children(children),
    )
}

fn format_cell(value: &str, width: usize, align: CellAlign) -> String {
    match align {
        CellAlign::Left => fit_visible(value, width),
        CellAlign::Right => right_visible(value, width),
        CellAlign::Center => center_visible(value, width),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::strip_ansi;

    #[test]
    fn with_theme_applies_semantic_colors() {
        let theme = Theme::tokyo_night();
        let table = DataTable::new(vec![DataColumn::new("Name")]).with_theme(&theme);

        assert_eq!(table.header_fg, theme.color(ThemeRole::Foreground));
        assert_eq!(table.separator_fg, theme.color(ThemeRole::Border));
    }

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
    fn element_zero_size_returns_empty_column() {
        let table = DataTable::new(vec![DataColumn::new("Name")]).row(DataRow::new(vec!["codex"]));

        let Element::Box(column) = table.element::<()>(20, 0) else {
            panic!("expected column");
        };
        assert_eq!(column.style.flex_direction, FlexDirection::Column);
        assert!(column.children.is_empty());

        let Element::Box(column) = table.element::<()>(0, 4) else {
            panic!("expected column");
        };
        assert!(column.children.is_empty());
    }

    #[test]
    fn element_renders_header_separator_and_bounded_rows() {
        let table = DataTable::new(vec![DataColumn::new("Name"), DataColumn::new("CPU")])
            .row(DataRow::new(vec!["codex", "12.4"]))
            .row(DataRow::new(vec!["a3s", "1.0"]));

        let Element::Box(column) = table.element::<()>(40, 3) else {
            panic!("expected column");
        };
        assert_eq!(column.style.flex_direction, FlexDirection::Column);
        assert_eq!(column.children.len(), 3);
        assert!(column.children[0]
            .text_content()
            .is_some_and(|text| text.contains("Name")));
        assert!(column.children[1]
            .text_content()
            .is_some_and(|text| text.contains("─")));
        assert!(column.children[2]
            .text_content()
            .is_some_and(|text| text.contains("codex")));
        assert!(!column.children.iter().any(|child| child
            .text_content()
            .is_some_and(|text| text.contains("a3s"))));
    }

    #[test]
    fn element_keeps_selected_row_visible_with_style() {
        let table = DataTable::new(vec![DataColumn::new("Name").width(6)])
            .row(DataRow::new(vec!["one"]))
            .row(DataRow::new(vec!["two"]))
            .row(DataRow::new(vec!["three"]).selected(Color::White, Color::Red))
            .selected(Some(usize::MAX))
            .scroll(0);

        let Element::Box(column) = table.element::<()>(12, 3) else {
            panic!("expected column");
        };
        assert_eq!(column.children.len(), 3);
        let Element::Text(row) = &column.children[2] else {
            panic!("expected row text");
        };
        assert!(row.content.contains("three"));
        assert_eq!(row.style.fg, Some(Color::White));
        assert_eq!(row.style.bg, Some(Color::Red));
        assert!(row.style.bold);
    }

    #[test]
    fn element_renders_empty_message() {
        let table = DataTable::new(vec![DataColumn::new("Name")]).empty("nothing here");

        let Element::Box(column) = table.element::<()>(24, 4) else {
            panic!("expected column");
        };
        assert_eq!(column.children.len(), 3);
        let Element::Text(empty) = &column.children[2] else {
            panic!("expected empty text");
        };
        assert!(empty.content.contains("nothing here"));
        assert_eq!(empty.style.fg, Some(Color::BrightBlack));
        assert!(empty.style.italic);
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
    fn oversized_gap_is_clamped_to_render_width() {
        let table = DataTable::new(vec![
            DataColumn::new("A").width(1),
            DataColumn::new("B").width(1),
            DataColumn::new("C").width(1),
        ])
        .gap(usize::MAX)
        .row(DataRow::new(vec!["1", "2", "3"]));

        assert_eq!(table.gap, MAX_DATA_TABLE_GAP);
        assert_eq!(table.gap_for_width(9, 3), 3);
        assert_eq!(table.gap_total_for_width(9, 3), 6);

        let plain = strip_ansi(&table.view(9, 4));
        let header = plain.lines().next().unwrap();
        assert_eq!(visible_len(header), 9);
        assert!(header.contains("A   B   C"));
        assert!(plain.lines().all(|line| visible_len(line) == 9));
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
    fn mouse_wheel_moves_selection_at_y_offset() {
        use crate::event::MouseEventKind;

        let mut table = DataTable::new(vec![DataColumn::new("Name")])
            .row(DataRow::new(vec!["one"]))
            .row(DataRow::new(vec!["two"]))
            .row(DataRow::new(vec!["three"]))
            .selected(Some(0));
        table.set_y_offset(3);

        let msg = table.handle_mouse(
            &MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column: 0,
                row: 5,
                modifiers: crate::KeyModifiers::NONE,
            },
            4,
        );

        assert_eq!(msg, None);
        assert_eq!(table.selected_index(), Some(1));
        assert_eq!(table.scroll_offset(), 0);
    }

    #[test]
    fn mouse_click_selects_visible_body_row_at_y_offset() {
        use crate::event::{MouseButton, MouseEventKind};

        let mut table = DataTable::new(vec![DataColumn::new("Name")])
            .row(DataRow::new(vec!["one"]))
            .row(DataRow::new(vec!["two"]))
            .row(DataRow::new(vec!["three"]))
            .row(DataRow::new(vec!["four"]))
            .selected(Some(2))
            .scroll(2);
        table.set_y_offset(4);

        let msg = table.handle_mouse(
            &MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 0,
                row: 7,
                modifiers: crate::KeyModifiers::NONE,
            },
            4,
        );

        assert_eq!(msg, Some(DataTableMsg::Selected(3)));
        assert_eq!(table.selected_index(), Some(3));
        assert_eq!(table.scroll_offset(), 2);
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
    fn oversized_column_widths_are_clamped() {
        let column = DataColumn::new("Huge")
            .width(usize::MAX)
            .min_width(usize::MAX);

        assert_eq!(column.width, Some(MAX_DATA_COLUMN_WIDTH));
        assert_eq!(column.min_width, MAX_DATA_COLUMN_WIDTH);

        let table = DataTable::new(vec![column]).row(DataRow::new(vec!["value"]));
        let plain = strip_ansi(&table.view(12, 4));

        assert!(plain.lines().all(|line| visible_len(line) == 12));
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
    fn stale_selected_row_is_clamped_during_rendering() {
        let table = DataTable::new(vec![DataColumn::new("Name").width(6)])
            .row(DataRow::new(vec!["one"]))
            .row(DataRow::new(vec!["two"]))
            .row(DataRow::new(vec!["three"]).selected(Color::White, Color::Red))
            .selected(Some(usize::MAX))
            .scroll(0);

        let rendered = table.view(12, 4);
        let plain = strip_ansi(&rendered);

        assert!(rendered.contains("\x1b[1;37;41m"));
        assert!(!plain.contains("one"));
        assert!(plain.contains("three"));
    }

    #[test]
    fn selected_row_before_scroll_is_kept_visible() {
        let table = DataTable::new(vec![DataColumn::new("Name").width(6)])
            .row(DataRow::new(vec!["one"]).selected(Color::White, Color::Blue))
            .row(DataRow::new(vec!["two"]))
            .row(DataRow::new(vec!["three"]))
            .selected(Some(0))
            .scroll(usize::MAX);

        let rendered = table.view(12, 4);
        let plain = strip_ansi(&rendered);

        assert!(rendered.contains("\x1b[1;37;44m"));
        assert!(plain.contains("one"));
        assert!(!plain.contains("three"));
    }

    #[test]
    fn oversized_cell_style_index_is_clamped() {
        let row = DataRow::new(vec!["dead"]).cell_fg(usize::MAX, Color::Red);

        assert_eq!(row.cell_fg.len(), MAX_DATA_ROW_CELL_STYLES);
        assert_eq!(row.cell_fg[MAX_DATA_ROW_CELL_STYLES - 1], Some(Color::Red));
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
