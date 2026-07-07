use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{pad_visible, visible_len, Color};

pub struct Table {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    col_widths: Vec<usize>,
}

impl Table {
    pub fn new(headers: Vec<impl Into<String>>) -> Self {
        let headers: Vec<String> = headers.into_iter().map(|h| h.into()).collect();
        let col_widths = headers.iter().map(|h| visible_len(h)).collect();
        Self {
            headers,
            rows: Vec::new(),
            col_widths,
        }
    }

    pub fn row(mut self, cells: Vec<impl Into<String>>) -> Self {
        let row: Vec<String> = cells.into_iter().map(|c| c.into()).collect();
        self.update_col_widths(&row);
        self.rows.push(row);
        self
    }

    pub fn add_row(&mut self, cells: Vec<impl Into<String>>) {
        let row: Vec<String> = cells.into_iter().map(|c| c.into()).collect();
        self.update_col_widths(&row);
        self.rows.push(row);
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        let mut lines: Vec<Element<Msg>> = Vec::new();

        let header_text = self.format_row(&self.headers);
        lines.push(Element::Text(
            TextElement::new(header_text).bold().fg(Color::BrightWhite),
        ));

        let separator = self
            .col_widths
            .iter()
            .map(|w| "─".repeat(w.saturating_add(2)))
            .collect::<Vec<_>>()
            .join("┼");
        lines.push(Element::Text(
            TextElement::new(separator).fg(Color::BrightBlack),
        ));

        for row in &self.rows {
            let row_text = self.format_row(row);
            lines.push(Element::Text(TextElement::new(row_text)));
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(lines),
        )
    }

    fn update_col_widths(&mut self, row: &[String]) {
        if row.len() > self.col_widths.len() {
            self.col_widths.resize(row.len(), 0);
        }
        for (i, cell) in row.iter().enumerate() {
            self.col_widths[i] = self.col_widths[i].max(visible_len(cell));
        }
    }

    fn format_row(&self, cells: &[String]) -> String {
        let column_count = self.col_widths.len().max(cells.len());
        (0..column_count)
            .map(|i| {
                let cell = cells.get(i).map(String::as_str).unwrap_or("");
                let width = self
                    .col_widths
                    .get(i)
                    .copied()
                    .unwrap_or_else(|| visible_len(cell));
                format!(" {} ", pad_visible(cell, width))
            })
            .collect::<Vec<_>>()
            .join("│")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::visible_len;

    #[test]
    fn empty_table() {
        let table = Table::new(vec!["A", "B"]);
        let el: Element<()> = table.element();
        match el {
            Element::Box(b) => {
                // header + separator = 2 lines
                assert_eq!(b.children.len(), 2);
            }
            _ => panic!("expected Box"),
        }
    }

    #[test]
    fn table_with_rows() {
        let table = Table::new(vec!["Name", "Age"])
            .row(vec!["Alice", "30"])
            .row(vec!["Bob", "25"]);
        let el: Element<()> = table.element();
        match el {
            Element::Box(b) => {
                // header + separator + 2 rows = 4
                assert_eq!(b.children.len(), 4);
            }
            _ => panic!("expected Box"),
        }
    }

    #[test]
    fn col_widths_expand() {
        let table = Table::new(vec!["X"]).row(vec!["longer text"]);
        assert_eq!(table.col_widths[0], 11);
    }

    #[test]
    fn columns_align_wide_cells_by_display_width() {
        let table = Table::new(vec!["文件", "State"]).row(vec!["中", "ok"]);
        let header = table.format_row(&table.headers);
        let row = table.format_row(&table.rows[0]);
        let header_first_col = header.split('│').next().unwrap();
        let row_first_col = row.split('│').next().unwrap();

        assert_eq!(visible_len(header_first_col), visible_len(row_first_col));
    }

    #[test]
    fn rows_with_missing_cells_preserve_columns() {
        let table = Table::new(vec!["A", "B"]).row(vec!["x"]);
        let header = table.format_row(&table.headers);
        let row = table.format_row(&table.rows[0]);

        assert_eq!(visible_len(&row), visible_len(&header));
        assert_eq!(row.split('│').count(), 2);
    }

    #[test]
    fn rows_with_extra_cells_expand_columns() {
        let table = Table::new(vec!["A"]).row(vec!["x", "extra"]);
        let Element::Box(box_el) = table.element::<()>() else {
            panic!("expected Box");
        };
        let Element::Text(header) = &box_el.children[0] else {
            panic!("expected header");
        };
        let Element::Text(separator) = &box_el.children[1] else {
            panic!("expected separator");
        };
        let Element::Text(row) = &box_el.children[2] else {
            panic!("expected row");
        };

        assert_eq!(table.col_widths, vec![1, 5]);
        assert_eq!(visible_len(&header.content), visible_len(&row.content));
        assert_eq!(visible_len(&separator.content), visible_len(&row.content));
        assert_eq!(row.content.split('│').count(), 2);
    }

    #[test]
    fn add_row_method() {
        let mut table = Table::new(vec!["Col"]);
        table.add_row(vec!["val"]);
        let el: Element<()> = table.element();
        match el {
            Element::Box(b) => assert_eq!(b.children.len(), 3),
            _ => panic!("expected Box"),
        }
    }
}
