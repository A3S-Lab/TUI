use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, truncate_visible, visible_len, Color, Style};

/// Row data for a [`Timeline`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelineRow {
    Section(String),
    Item(TimelineItem),
}

impl TimelineRow {
    pub fn section(label: impl Into<String>) -> Self {
        Self::Section(label.into())
    }

    pub fn item(item: TimelineItem) -> Self {
        Self::Item(item)
    }
}

/// One node in a [`Timeline`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineItem {
    time: String,
    badge: String,
    preview: String,
    color: Option<Color>,
}

impl TimelineItem {
    pub fn new(
        time: impl Into<String>,
        badge: impl Into<String>,
        preview: impl Into<String>,
    ) -> Self {
        Self {
            time: time.into(),
            badge: badge.into(),
            preview: preview.into(),
            color: None,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn time_value(&self) -> &str {
        &self.time
    }

    pub fn badge_value(&self) -> &str {
        &self.badge
    }

    pub fn preview_value(&self) -> &str {
        &self.preview
    }

    pub fn color_value(&self) -> Option<Color> {
        self.color
    }
}

/// Scrollable timeline with section separators and colored item nodes.
///
/// This extracts the CLI memory timeline pattern: day/section buckets,
/// rail-style item nodes, relative time, compact badges, preview text, and a
/// highlighted selected node that is kept visible when possible.
#[derive(Debug, Clone)]
pub struct Timeline {
    rows: Vec<TimelineRow>,
    selected_item: Option<usize>,
    scroll: usize,
    margin: usize,
    marker: String,
    time_width: usize,
    badge_width: usize,
    fill_height: bool,
    item_color: Color,
    selected_fg: Color,
    section_color: Color,
    time_color: Color,
    preview_color: Color,
}

impl Timeline {
    pub fn new() -> Self {
        Self {
            rows: Vec::new(),
            selected_item: None,
            scroll: 0,
            margin: 2,
            marker: "●".to_string(),
            time_width: 4,
            badge_width: 4,
            fill_height: false,
            item_color: Color::Cyan,
            selected_fg: Color::Black,
            section_color: Color::BrightBlack,
            time_color: Color::BrightBlack,
            preview_color: Color::White,
        }
    }

    pub fn section(mut self, label: impl Into<String>) -> Self {
        self.rows.push(TimelineRow::section(label));
        self
    }

    pub fn item(mut self, item: TimelineItem) -> Self {
        self.rows.push(TimelineRow::item(item));
        self
    }

    pub fn row(mut self, row: TimelineRow) -> Self {
        self.rows.push(row);
        self
    }

    pub fn rows(mut self, rows: Vec<TimelineRow>) -> Self {
        self.rows = rows;
        self.clamp_selection();
        self
    }

    pub fn add_row(&mut self, row: TimelineRow) {
        self.rows.push(row);
        self.clamp_selection();
    }

    pub fn selected_item(mut self, selected_item: usize) -> Self {
        self.selected_item = self
            .item_count()
            .checked_sub(1)
            .map(|max| selected_item.min(max));
        self
    }

    pub fn scroll(mut self, scroll: usize) -> Self {
        self.scroll = scroll;
        self
    }

    pub fn margin(mut self, margin: usize) -> Self {
        self.margin = margin;
        self
    }

    pub fn marker(mut self, marker: impl Into<String>) -> Self {
        let marker = marker.into();
        if !marker.is_empty() {
            self.marker = marker;
        }
        self
    }

    pub fn time_width(mut self, width: usize) -> Self {
        self.time_width = width.max(1);
        self
    }

    pub fn badge_width(mut self, width: usize) -> Self {
        self.badge_width = width.max(1);
        self
    }

    pub fn fill_height(mut self, fill: bool) -> Self {
        self.fill_height = fill;
        self
    }

    pub fn item_color(mut self, color: Color) -> Self {
        self.item_color = color;
        self
    }

    pub fn selected_fg(mut self, color: Color) -> Self {
        self.selected_fg = color;
        self
    }

    pub fn section_color(mut self, color: Color) -> Self {
        self.section_color = color;
        self
    }

    pub fn time_color(mut self, color: Color) -> Self {
        self.time_color = color;
        self
    }

    pub fn preview_color(mut self, color: Color) -> Self {
        self.preview_color = color;
        self
    }

    pub fn rows_value(&self) -> &[TimelineRow] {
        &self.rows
    }

    pub fn selected_item_value(&self) -> Option<usize> {
        self.selected_item
    }

    pub fn view(&self, width: u16, height: usize) -> String {
        let width = width as usize;
        if width == 0 || height == 0 || self.rows.is_empty() {
            return String::new();
        }

        let mut lines = self
            .visible_rows(height)
            .into_iter()
            .map(|(row, selected)| self.render_row(row, selected, width))
            .collect::<Vec<_>>();
        if self.fill_height {
            while lines.len() < height {
                lines.push(String::new());
            }
        }

        lines
            .into_iter()
            .take(height)
            .map(|line| fit_visible(&line, width))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn element<Msg>(&self, width: u16, height: usize) -> Element<Msg> {
        let width = width as usize;
        if width == 0 || height == 0 || self.rows.is_empty() {
            return Element::Box(BoxElement::new().direction(FlexDirection::Column));
        }

        let mut children = self
            .visible_rows(height)
            .into_iter()
            .map(|(row, selected)| self.row_element(row, selected, width))
            .collect::<Vec<_>>();
        if self.fill_height {
            while children.len() < height {
                children.push(Element::Text(TextElement::new("")));
            }
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(children),
        )
    }

    fn render_row(&self, row: &TimelineRow, selected: bool, width: usize) -> String {
        match row {
            TimelineRow::Section(label) => self.render_section(label, width),
            TimelineRow::Item(item) => self.render_item(item, selected, width),
        }
    }

    fn render_section(&self, label: &str, width: usize) -> String {
        let head = format!("{}── {} ", " ".repeat(self.margin), label);
        let fill = "─".repeat(width.saturating_sub(visible_len(&head)));
        Style::new()
            .fg(self.section_color)
            .render(&fit_visible(&format!("{head}{fill}"), width))
    }

    fn render_item(&self, item: &TimelineItem, selected: bool, width: usize) -> String {
        let color = item.color.unwrap_or(self.item_color);
        let plain = self.item_plain(item, width);
        if selected {
            return Style::new()
                .fg(self.selected_fg)
                .bg(color)
                .render(&fit_visible(&plain, width));
        }

        let preview_width = self.preview_width(width);
        let time = self.fit_slot(&item.time, self.time_width, true);
        let badge = self.fit_slot(&item.badge, self.badge_width, false);
        let preview = truncate_visible(&item.preview, preview_width);
        format!(
            "{}{}{}{}{}",
            " ".repeat(self.margin),
            Style::new().fg(color).render(&format!(" {}", self.marker)),
            Style::new().fg(self.time_color).render(&format!(" {time}")),
            Style::new().fg(color).render(&format!("  {badge}")),
            Style::new()
                .fg(self.preview_color)
                .render(&format!("  {preview}"))
        )
    }

    fn row_element<Msg>(&self, row: &TimelineRow, selected: bool, width: usize) -> Element<Msg> {
        match row {
            TimelineRow::Section(label) => Element::Text(
                TextElement::new(crate::style::strip_ansi(&self.render_section(label, width)))
                    .fg(self.section_color),
            ),
            TimelineRow::Item(item) if selected => {
                let color = item.color.unwrap_or(self.item_color);
                Element::Text(
                    TextElement::new(self.item_plain(item, width))
                        .fg(self.selected_fg)
                        .bg(color),
                )
            }
            TimelineRow::Item(item) => {
                let color = item.color.unwrap_or(self.item_color);
                let preview_width = self.preview_width(width);
                let time = self.fit_slot(&item.time, self.time_width, true);
                let badge = self.fit_slot(&item.badge, self.badge_width, false);
                let preview = truncate_visible(&item.preview, preview_width);
                Element::Box(
                    BoxElement::new()
                        .direction(FlexDirection::Row)
                        .child(Element::Text(TextElement::new(" ".repeat(self.margin))))
                        .child(Element::Text(
                            TextElement::new(format!(" {}", self.marker)).fg(color),
                        ))
                        .child(Element::Text(
                            TextElement::new(format!(" {time}")).fg(self.time_color),
                        ))
                        .child(Element::Text(
                            TextElement::new(format!("  {badge}")).fg(color),
                        ))
                        .child(Element::Text(
                            TextElement::new(format!("  {preview}")).fg(self.preview_color),
                        )),
                )
            }
        }
    }

    fn item_plain(&self, item: &TimelineItem, width: usize) -> String {
        let time = self.fit_slot(&item.time, self.time_width, true);
        let badge = self.fit_slot(&item.badge, self.badge_width, false);
        let prefix = format!(
            "{} {} {time}  {badge}  ",
            " ".repeat(self.margin),
            self.marker
        );
        let preview = truncate_visible(&item.preview, width.saturating_sub(visible_len(&prefix)));
        format!("{prefix}{preview}")
    }

    fn visible_rows(&self, height: usize) -> Vec<(&TimelineRow, bool)> {
        let start = self.visible_start(height);
        let selected_row = self.selected_row_index();
        self.rows
            .iter()
            .enumerate()
            .skip(start)
            .take(height)
            .map(|(index, row)| (row, selected_row == Some(index)))
            .collect()
    }

    fn visible_start(&self, height: usize) -> usize {
        if height == 0 || self.rows.len() <= height {
            return 0;
        }
        let max_start = self.rows.len().saturating_sub(height);
        let scroll = self.scroll.min(max_start);
        let Some(selected_row) = self.selected_row_index() else {
            return scroll;
        };
        if selected_row < scroll {
            selected_row
        } else if selected_row >= scroll + height {
            selected_row
                .saturating_add(1)
                .saturating_sub(height)
                .min(max_start)
        } else {
            scroll
        }
    }

    fn selected_row_index(&self) -> Option<usize> {
        let selected = self.selected_item?;
        let mut item_index = 0usize;
        for (row_index, row) in self.rows.iter().enumerate() {
            if matches!(row, TimelineRow::Item(_)) {
                if item_index == selected {
                    return Some(row_index);
                }
                item_index += 1;
            }
        }
        None
    }

    fn clamp_selection(&mut self) {
        self.selected_item = self.selected_item.and_then(|selected| {
            self.item_count()
                .checked_sub(1)
                .map(|max| selected.min(max))
        });
    }

    fn item_count(&self) -> usize {
        self.rows
            .iter()
            .filter(|row| matches!(row, TimelineRow::Item(_)))
            .count()
    }

    fn preview_width(&self, width: usize) -> usize {
        let prefix_width = self.margin
            + 1
            + visible_len(&self.marker)
            + 1
            + self.time_width
            + 2
            + self.badge_width
            + 2;
        width.saturating_sub(prefix_width).max(1)
    }

    fn fit_slot(&self, value: &str, width: usize, align_right: bool) -> String {
        let value = truncate_visible(value, width);
        let len = visible_len(&value);
        if len >= width {
            return value;
        }
        let pad = " ".repeat(width - len);
        if align_right {
            format!("{pad}{value}")
        } else {
            format!("{value}{pad}")
        }
    }
}

impl Default for Timeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::strip_ansi;

    fn sample_timeline() -> Timeline {
        Timeline::new()
            .section("today")
            .item(TimelineItem::new("2m", "fact", "workspace uses a3s-tui").color(Color::Cyan))
            .item(
                TimelineItem::new("18m", "risk", "network test failed but recovered")
                    .color(Color::Yellow),
            )
            .section("yesterday")
            .item(TimelineItem::new("1d", "fix", "patched terminal layout").color(Color::Green))
    }

    #[test]
    fn renders_sections_items_and_selected_row_at_fixed_width() {
        let rendered = sample_timeline().selected_item(1).view(56, 5);
        let plain = strip_ansi(&rendered);
        let rows = plain.lines().collect::<Vec<_>>();

        assert_eq!(rows.len(), 5);
        assert!(rows[0].contains("today"));
        assert!(rows[1].contains("●   2m  fact"));
        assert!(rows[2].contains("risk"));
        assert!(rendered.lines().all(|line| visible_len(line) == 56));
        assert!(rendered.contains("\x1b[30;43m"));
    }

    #[test]
    fn scroll_keeps_selected_item_visible() {
        let rendered = sample_timeline().selected_item(2).scroll(0).view(40, 2);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("yesterday"));
        assert!(plain.contains("patched terminal"));
        assert!(!plain.contains("workspace uses"));
    }

    #[test]
    fn cjk_preview_fits_requested_width() {
        let rendered = Timeline::new()
            .item(TimelineItem::new("现在", "事实", "中文内容测试 with suffix").color(Color::Cyan))
            .view(24, 1);

        assert_eq!(visible_len(&rendered), 24);
        assert!(strip_ansi(&rendered).contains("中文"));
    }

    #[test]
    fn fill_height_adds_blank_rows() {
        let rendered = Timeline::new()
            .item(TimelineItem::new("1m", "ok", "done"))
            .fill_height(true)
            .view(24, 3);

        assert_eq!(rendered.lines().count(), 3);
    }

    #[test]
    fn element_styles_selected_item() {
        let element: Element<()> = sample_timeline().selected_item(0).element(48, 3);
        let Element::Box(column) = element else {
            panic!("expected column");
        };
        let Element::Text(selected) = &column.children[1] else {
            panic!("expected selected text");
        };

        assert_eq!(selected.style.fg, Some(Color::Black));
        assert_eq!(selected.style.bg, Some(Color::Cyan));
    }
}
