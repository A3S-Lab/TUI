use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::Color;

pub struct Table {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    col_widths: Vec<usize>,
}

impl Table {
    pub fn new(headers: Vec<impl Into<String>>) -> Self {
        let headers: Vec<String> = headers.into_iter().map(|h| h.into()).collect();
        let col_widths = headers.iter().map(|h| h.len()).collect();
        Self {
            headers,
            rows: Vec::new(),
            col_widths,
        }
    }

    pub fn row(mut self, cells: Vec<impl Into<String>>) -> Self {
        let row: Vec<String> = cells.into_iter().map(|c| c.into()).collect();
        for (i, cell) in row.iter().enumerate() {
            if i < self.col_widths.len() {
                self.col_widths[i] = self.col_widths[i].max(cell.len());
            }
        }
        self.rows.push(row);
        self
    }

    pub fn add_row(&mut self, cells: Vec<impl Into<String>>) {
        let row: Vec<String> = cells.into_iter().map(|c| c.into()).collect();
        for (i, cell) in row.iter().enumerate() {
            if i < self.col_widths.len() {
                self.col_widths[i] = self.col_widths[i].max(cell.len());
            }
        }
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
            .map(|w| "─".repeat(*w + 2))
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

    fn format_row(&self, cells: &[String]) -> String {
        cells
            .iter()
            .enumerate()
            .map(|(i, cell)| {
                let width = self.col_widths.get(i).copied().unwrap_or(cell.len());
                format!(" {:width$} ", cell, width = width)
            })
            .collect::<Vec<_>>()
            .join("│")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
