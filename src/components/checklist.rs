use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, truncate_visible, visible_len, Color, Style};

/// Status for a checklist row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecklistStatus {
    Pending,
    Active,
    Done,
    Error,
}

impl ChecklistStatus {
    pub fn glyph(self) -> char {
        match self {
            ChecklistStatus::Pending => '◻',
            ChecklistStatus::Active => '◼',
            ChecklistStatus::Done => '✔',
            ChecklistStatus::Error => '✗',
        }
    }
}

/// One row in a [`Checklist`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChecklistItem {
    label: String,
    status: ChecklistStatus,
    glyph: Option<char>,
    color: Option<Color>,
}

impl ChecklistItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            status: ChecklistStatus::Pending,
            glyph: None,
            color: None,
        }
    }

    pub fn status(mut self, status: ChecklistStatus) -> Self {
        self.status = status;
        self
    }

    pub fn active(self) -> Self {
        self.status(ChecklistStatus::Active)
    }

    pub fn done(self) -> Self {
        self.status(ChecklistStatus::Done)
    }

    pub fn error(self) -> Self {
        self.status(ChecklistStatus::Error)
    }

    pub fn glyph(mut self, glyph: char) -> Self {
        self.glyph = Some(glyph);
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn status_value(&self) -> ChecklistStatus {
        self.status
    }
}

/// A compact task/checklist component for plans, TODOs, and workflows.
///
/// The text renderer is width-aware and preserves ANSI styling when truncating.
/// It is intentionally generic: callers choose the text and colors, while the
/// component handles consistent glyphs, status styling, connectors, and padding.
#[derive(Debug, Clone)]
pub struct Checklist {
    items: Vec<ChecklistItem>,
    indent: usize,
    connector: bool,
    pending_color: Color,
    active_color: Color,
    done_color: Color,
    error_color: Color,
    text_color: Color,
}

impl Checklist {
    pub fn new(items: Vec<ChecklistItem>) -> Self {
        Self {
            items,
            indent: 0,
            connector: false,
            pending_color: Color::BrightBlack,
            active_color: Color::Yellow,
            done_color: Color::BrightBlack,
            error_color: Color::Red,
            text_color: Color::White,
        }
    }

    pub fn empty() -> Self {
        Self::new(Vec::new())
    }

    pub fn item(mut self, item: ChecklistItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn add_item(&mut self, item: ChecklistItem) {
        self.items.push(item);
    }

    pub fn indent(mut self, indent: usize) -> Self {
        self.indent = indent;
        self
    }

    pub fn connector(mut self, enabled: bool) -> Self {
        self.connector = enabled;
        self
    }

    pub fn pending_color(mut self, color: Color) -> Self {
        self.pending_color = color;
        self
    }

    pub fn active_color(mut self, color: Color) -> Self {
        self.active_color = color;
        self
    }

    pub fn done_color(mut self, color: Color) -> Self {
        self.done_color = color;
        self
    }

    pub fn error_color(mut self, color: Color) -> Self {
        self.error_color = color;
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    pub fn items(&self) -> &[ChecklistItem] {
        &self.items
    }

    pub fn view(&self, width: u16, height: usize) -> String {
        let width = width as usize;
        if width == 0 || height == 0 {
            return String::new();
        }

        self.items
            .iter()
            .take(height)
            .enumerate()
            .map(|(index, item)| self.render_line(item, index, width))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        let children = self
            .items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let mut label = TextElement::new(item.label.clone()).fg(self.item_text_color(item));
                if item.status == ChecklistStatus::Done {
                    label = label.strikethrough();
                }

                Element::Box(
                    BoxElement::new()
                        .direction(FlexDirection::Row)
                        .child(Element::Text(TextElement::new(self.plain_prefix(index))))
                        .child(Element::Text(
                            TextElement::new(
                                item.glyph
                                    .unwrap_or_else(|| item.status.glyph())
                                    .to_string(),
                            )
                            .fg(self.glyph_color(item)),
                        ))
                        .child(Element::Text(TextElement::new(" ")))
                        .child(Element::Text(label)),
                )
            })
            .collect();

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(children),
        )
    }

    fn render_line(&self, item: &ChecklistItem, index: usize, width: usize) -> String {
        let prefix = self.render_prefix(item, index);
        let gap = " ";
        let label_width = width.saturating_sub(visible_len(&prefix) + visible_len(gap));
        let label = truncate_visible(&item.label, label_width);
        let mut style = Style::new().fg(self.item_text_color(item));
        if item.status == ChecklistStatus::Done {
            style = style.strikethrough();
        }
        fit_visible(&format!("{prefix}{gap}{}", style.render(&label)), width)
    }

    fn render_prefix(&self, item: &ChecklistItem, index: usize) -> String {
        let glyph = item.glyph.unwrap_or_else(|| item.status.glyph());
        format!(
            "{}{}{}",
            " ".repeat(self.indent),
            self.connector_prefix(index),
            Style::new()
                .fg(self.glyph_color(item))
                .render(&glyph.to_string())
        )
    }

    fn plain_prefix(&self, index: usize) -> String {
        format!(
            "{}{}",
            " ".repeat(self.indent),
            self.connector_prefix(index)
        )
    }

    fn connector_prefix(&self, index: usize) -> &'static str {
        if !self.connector {
            ""
        } else if index == 0 {
            "⎿  "
        } else {
            "   "
        }
    }

    fn glyph_color(&self, item: &ChecklistItem) -> Color {
        item.color.unwrap_or(match item.status {
            ChecklistStatus::Pending => self.pending_color,
            ChecklistStatus::Active => self.active_color,
            ChecklistStatus::Done => self.done_color,
            ChecklistStatus::Error => self.error_color,
        })
    }

    fn item_text_color(&self, item: &ChecklistItem) -> Color {
        item.color.unwrap_or(match item.status {
            ChecklistStatus::Pending => self.text_color,
            ChecklistStatus::Active => self.active_color,
            ChecklistStatus::Done => self.done_color,
            ChecklistStatus::Error => self.error_color,
        })
    }
}

impl Default for Checklist {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::strip_ansi;

    #[test]
    fn renders_status_glyphs_and_labels() {
        let checklist = Checklist::new(vec![
            ChecklistItem::new("collect evidence"),
            ChecklistItem::new("implement").active(),
            ChecklistItem::new("verify").done(),
            ChecklistItem::new("ship").error(),
        ]);

        let plain = strip_ansi(&checklist.view(40, 8));

        assert!(plain.contains("◻ collect evidence"));
        assert!(plain.contains("◼ implement"));
        assert!(plain.contains("✔ verify"));
        assert!(plain.contains("✗ ship"));
    }

    #[test]
    fn truncates_and_pads_to_requested_width() {
        let checklist = Checklist::new(vec![ChecklistItem::new("中文测试内容 with long suffix")]);
        let rendered = checklist.view(12, 4);

        for line in rendered.lines() {
            assert_eq!(visible_len(line), 12, "{line:?}");
        }
    }

    #[test]
    fn limits_rows_by_height() {
        let checklist = Checklist::empty()
            .item(ChecklistItem::new("one"))
            .item(ChecklistItem::new("two"))
            .item(ChecklistItem::new("three"));

        assert_eq!(strip_ansi(&checklist.view(30, 2)).lines().count(), 2);
    }

    #[test]
    fn connector_aligns_child_rows() {
        let checklist = Checklist::new(vec![ChecklistItem::new("one"), ChecklistItem::new("two")])
            .indent(2)
            .connector(true);

        let plain = strip_ansi(&checklist.view(30, 3));
        let rows = plain.lines().collect::<Vec<_>>();

        assert!(rows[0].starts_with("  ⎿  ◻ one"));
        assert!(rows[1].starts_with("     ◻ two"));
    }

    #[test]
    fn done_rows_are_struck_through() {
        let checklist = Checklist::new(vec![ChecklistItem::new("complete").done()]);
        let rendered = checklist.view(30, 1);

        assert!(rendered.contains("\x1b[9;"));
    }

    #[test]
    fn element_produces_column_rows() {
        let checklist = Checklist::new(vec![ChecklistItem::new("one")]);
        let el: Element<()> = checklist.element();

        match el {
            Element::Box(column) => {
                assert_eq!(column.style.flex_direction, FlexDirection::Column);
                assert_eq!(column.children.len(), 1);
            }
            _ => panic!("expected Box"),
        }
    }
}
