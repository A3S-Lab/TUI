use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, truncate_visible, visible_len, Color, Style};
use crate::theme::{Theme, ThemeRole};

const MAX_CHECKLIST_INDENT: usize = u16::MAX as usize;

/// Status for a checklist row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecklistStatus {
    Pending,
    Active,
    Done,
    Error,
    Skipped,
    Cancelled,
}

impl ChecklistStatus {
    pub fn glyph(self) -> char {
        match self {
            ChecklistStatus::Pending => '◻',
            ChecklistStatus::Active => '◼',
            ChecklistStatus::Done => '✔',
            ChecklistStatus::Error => '✗',
            ChecklistStatus::Skipped => '↷',
            ChecklistStatus::Cancelled => '⊘',
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
    glyph_color: Option<Color>,
    text_color: Option<Color>,
}

impl ChecklistItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            status: ChecklistStatus::Pending,
            glyph: None,
            color: None,
            glyph_color: None,
            text_color: None,
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

    pub fn skipped(self) -> Self {
        self.status(ChecklistStatus::Skipped)
    }

    pub fn cancelled(self) -> Self {
        self.status(ChecklistStatus::Cancelled)
    }

    pub fn glyph(mut self, glyph: char) -> Self {
        self.glyph = Some(glyph);
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn glyph_color(mut self, color: Color) -> Self {
        self.glyph_color = Some(color);
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
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
    skipped_color: Color,
    cancelled_color: Color,
    text_color: Color,
    strikethrough_done: bool,
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
            skipped_color: Color::BrightBlack,
            cancelled_color: Color::BrightBlack,
            text_color: Color::White,
            strikethrough_done: true,
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
        self.indent = indent.min(MAX_CHECKLIST_INDENT);
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

    pub fn skipped_color(mut self, color: Color) -> Self {
        self.skipped_color = color;
        self
    }

    pub fn cancelled_color(mut self, color: Color) -> Self {
        self.cancelled_color = color;
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    pub fn strikethrough_done(mut self, enabled: bool) -> Self {
        self.strikethrough_done = enabled;
        self
    }

    pub fn with_theme(mut self, theme: &Theme) -> Self {
        self.pending_color = theme.color(ThemeRole::Muted);
        self.active_color = theme.color(ThemeRole::Warning);
        self.done_color = theme.color(ThemeRole::Success);
        self.error_color = theme.color(ThemeRole::Error);
        self.skipped_color = theme.color(ThemeRole::Muted);
        self.cancelled_color = theme.color(ThemeRole::Muted);
        self.text_color = theme.color(ThemeRole::Foreground);
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
        self.element_with_height(self.items.len())
    }

    pub fn element_with_height<Msg>(&self, height: usize) -> Element<Msg> {
        let children = self
            .items
            .iter()
            .take(height)
            .enumerate()
            .map(|(index, item)| {
                let mut label = TextElement::new(item.label.clone()).fg(self.item_text_color(item));
                if self.strikethrough_done && item.status == ChecklistStatus::Done {
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
        let prefix = self.render_prefix(item, index, width);
        let gap = " ";
        let label_width =
            width.saturating_sub(visible_len(&prefix).saturating_add(visible_len(gap)));
        let label = truncate_visible(&item.label, label_width);
        let mut style = Style::new().fg(self.item_text_color(item));
        if self.strikethrough_done && item.status == ChecklistStatus::Done {
            style = style.strikethrough();
        }
        fit_visible(&format!("{prefix}{gap}{}", style.render(&label)), width)
    }

    fn render_prefix(&self, item: &ChecklistItem, index: usize, width: usize) -> String {
        let glyph = item.glyph.unwrap_or_else(|| item.status.glyph());
        let indent = self.indent_for_width(item, index, width);
        format!(
            "{}{}{}",
            " ".repeat(indent),
            self.connector_prefix(index),
            Style::new()
                .fg(self.glyph_color(item))
                .render(&glyph.to_string())
        )
    }

    fn plain_prefix(&self, index: usize) -> String {
        format!(
            "{}{}",
            " ".repeat(self.indent_for_element()),
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
        item.glyph_color
            .or(item.color)
            .unwrap_or(match item.status {
                ChecklistStatus::Pending => self.pending_color,
                ChecklistStatus::Active => self.active_color,
                ChecklistStatus::Done => self.done_color,
                ChecklistStatus::Error => self.error_color,
                ChecklistStatus::Skipped => self.skipped_color,
                ChecklistStatus::Cancelled => self.cancelled_color,
            })
    }

    fn item_text_color(&self, item: &ChecklistItem) -> Color {
        item.text_color.or(item.color).unwrap_or(match item.status {
            ChecklistStatus::Pending => self.text_color,
            ChecklistStatus::Active => self.active_color,
            ChecklistStatus::Done => self.done_color,
            ChecklistStatus::Error => self.error_color,
            ChecklistStatus::Skipped => self.skipped_color,
            ChecklistStatus::Cancelled => self.cancelled_color,
        })
    }

    fn indent_for_width(&self, item: &ChecklistItem, index: usize, width: usize) -> usize {
        let glyph_width = visible_len(
            &item
                .glyph
                .unwrap_or_else(|| item.status.glyph())
                .to_string(),
        );
        let fixed_width = visible_len(self.connector_prefix(index))
            .saturating_add(glyph_width)
            .saturating_add(1);
        self.indent
            .min(width.saturating_sub(fixed_width))
            .min(MAX_CHECKLIST_INDENT)
    }

    fn indent_for_element(&self) -> usize {
        self.indent.min(MAX_CHECKLIST_INDENT)
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
            ChecklistItem::new("optional").skipped(),
            ChecklistItem::new("obsolete").cancelled(),
        ]);

        let plain = strip_ansi(&checklist.view(40, 8));

        assert!(plain.contains("◻ collect evidence"));
        assert!(plain.contains("◼ implement"));
        assert!(plain.contains("✔ verify"));
        assert!(plain.contains("✗ ship"));
        assert!(plain.contains("↷ optional"));
        assert!(plain.contains("⊘ obsolete"));
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
    fn oversized_indent_is_clamped_to_render_width() {
        let checklist = Checklist::new(vec![ChecklistItem::new("one")])
            .indent(usize::MAX)
            .connector(true);
        let rendered = checklist.view(8, 1);
        let prefix = checklist.render_prefix(&checklist.items[0], 0, 8);

        assert_eq!(checklist.indent, MAX_CHECKLIST_INDENT);
        assert!(visible_len(&prefix) <= 7);
        assert!(rendered.lines().all(|line| visible_len(line) == 8));

        let Element::Box(column) = checklist.element::<()>() else {
            panic!("expected column element");
        };
        let Element::Box(row) = &column.children[0] else {
            panic!("expected row element");
        };
        let Element::Text(prefix) = &row.children[0] else {
            panic!("expected prefix text");
        };
        assert_eq!(
            visible_len(&prefix.content),
            MAX_CHECKLIST_INDENT + visible_len("⎿  ")
        );
    }

    #[test]
    fn done_rows_are_struck_through() {
        let checklist = Checklist::new(vec![ChecklistItem::new("complete").done()]);
        let rendered = checklist.view(30, 1);

        assert!(rendered.contains("\x1b[9;"));
    }

    #[test]
    fn row_color_still_tints_glyph_and_label() {
        let checklist = Checklist::new(vec![ChecklistItem::new("tinted").color(Color::Yellow)]);
        let rendered = checklist.view(30, 1);

        assert!(rendered.contains("\x1b[33m◻\x1b[0m"));
        assert!(rendered.contains("\x1b[33mtinted\x1b[0m"));
    }

    #[test]
    fn glyph_and_text_colors_can_differ_without_done_strike() {
        let checklist = Checklist::new(vec![ChecklistItem::new("agent result")
            .done()
            .glyph('✓')
            .glyph_color(Color::Green)
            .text_color(Color::BrightBlack)])
        .strikethrough_done(false);
        let rendered = checklist.view(40, 1);

        assert_eq!(strip_ansi(&rendered).trim_end(), "✓ agent result");
        assert!(rendered.contains("\x1b[32m✓\x1b[0m"));
        assert!(rendered.contains("\x1b[90magent result\x1b[0m"));
        assert!(!rendered.contains("\x1b[9;"));
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

    #[test]
    fn element_with_height_limits_rows() {
        let checklist = Checklist::empty()
            .item(ChecklistItem::new("one"))
            .item(ChecklistItem::new("two").active())
            .item(ChecklistItem::new("three").done());

        let Element::Box(column) = checklist.element_with_height::<()>(2) else {
            panic!("expected column element");
        };
        let text = column
            .children
            .iter()
            .flat_map(|row| match row {
                Element::Box(row) => row
                    .children
                    .iter()
                    .filter_map(Element::text_content)
                    .collect::<Vec<_>>(),
                _ => Vec::new(),
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert_eq!(column.children.len(), 2);
        assert!(text.contains("one"));
        assert!(text.contains("two"));
        assert!(!text.contains("three"));
    }

    #[test]
    fn with_theme_applies_semantic_colors() {
        let theme = Theme::tokyo_night();
        let checklist = Checklist::empty().with_theme(&theme);

        assert_eq!(checklist.pending_color, theme.color(ThemeRole::Muted));
        assert_eq!(checklist.active_color, theme.color(ThemeRole::Warning));
        assert_eq!(checklist.done_color, theme.color(ThemeRole::Success));
        assert_eq!(checklist.error_color, theme.color(ThemeRole::Error));
        assert_eq!(checklist.text_color, theme.color(ThemeRole::Foreground));
    }
}
