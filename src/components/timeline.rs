use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, right_visible, truncate_visible, visible_len, Color, Style};

const MAX_TIMELINE_BADGE_WIDTH: usize = u16::MAX as usize;
const MAX_TIMELINE_MARGIN: usize = u16::MAX as usize;
const MAX_TIMELINE_TIME_WIDTH: usize = u16::MAX as usize;

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
        self.selected_item = Some(selected_item);
        self.clamp_selection();
        self
    }

    pub fn scroll(mut self, scroll: usize) -> Self {
        self.scroll = scroll;
        self
    }

    pub fn margin(mut self, margin: usize) -> Self {
        self.margin = margin.min(MAX_TIMELINE_MARGIN);
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
        self.time_width = width.clamp(1, MAX_TIMELINE_TIME_WIDTH);
        self
    }

    pub fn badge_width(mut self, width: usize) -> Self {
        self.badge_width = width.clamp(1, MAX_TIMELINE_BADGE_WIDTH);
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
        self.normalized_selected_item()
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
        let head = format!("{}── {} ", " ".repeat(self.margin_for_width(width)), label);
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
        let time = self.fit_slot(&item.time, self.time_width_for_width(width), true);
        let badge = self.fit_slot(&item.badge, self.badge_width_for_width(width), false);
        let preview = truncate_visible(&item.preview, preview_width);
        format!(
            "{}{}{}{}{}",
            " ".repeat(self.margin_for_width(width)),
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
                    TextElement::new(fit_visible(&self.item_plain(item, width), width))
                        .fg(self.selected_fg)
                        .bg(color),
                )
            }
            TimelineRow::Item(item) => {
                let color = item.color.unwrap_or(self.item_color);
                let preview_width = self.preview_width(width);
                let time = self.fit_slot(&item.time, self.time_width_for_width(width), true);
                let badge = self.fit_slot(&item.badge, self.badge_width_for_width(width), false);
                let preview = truncate_visible(&item.preview, preview_width);
                let mut children = Vec::new();
                let mut used = 0usize;
                Self::push_limited_text(
                    &mut children,
                    " ".repeat(self.margin_for_width(width)),
                    None,
                    &mut used,
                    width,
                );
                Self::push_limited_text(
                    &mut children,
                    format!(" {}", self.marker),
                    Some(color),
                    &mut used,
                    width,
                );
                Self::push_limited_text(
                    &mut children,
                    format!(" {time}"),
                    Some(self.time_color),
                    &mut used,
                    width,
                );
                Self::push_limited_text(
                    &mut children,
                    format!("  {badge}"),
                    Some(color),
                    &mut used,
                    width,
                );
                Self::push_limited_text(
                    &mut children,
                    format!("  {preview}"),
                    Some(self.preview_color),
                    &mut used,
                    width,
                );

                Element::Box(
                    BoxElement::new()
                        .direction(FlexDirection::Row)
                        .children(children),
                )
            }
        }
    }

    fn push_limited_text<Msg>(
        children: &mut Vec<Element<Msg>>,
        text: String,
        fg: Option<Color>,
        used: &mut usize,
        width: usize,
    ) {
        let remaining = width.saturating_sub(*used);
        if remaining == 0 {
            return;
        }

        let clipped = truncate_visible(&text, remaining);
        if clipped.is_empty() {
            return;
        }

        *used = (*used).saturating_add(visible_len(&clipped));
        let mut text = TextElement::new(clipped);
        if let Some(color) = fg {
            text = text.fg(color);
        }
        children.push(Element::Text(text));
    }

    fn item_plain(&self, item: &TimelineItem, width: usize) -> String {
        let time = self.fit_slot(&item.time, self.time_width_for_width(width), true);
        let badge = self.fit_slot(&item.badge, self.badge_width_for_width(width), false);
        let prefix = format!(
            "{} {} {time}  {badge}  ",
            " ".repeat(self.margin_for_width(width)),
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
        let selected = self.normalized_selected_item()?;
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
        self.selected_item = self.normalized_selected_item();
    }

    fn normalized_selected_item(&self) -> Option<usize> {
        self.selected_item.and_then(|selected| {
            self.item_count()
                .checked_sub(1)
                .map(|max| selected.min(max))
        })
    }

    fn item_count(&self) -> usize {
        self.rows
            .iter()
            .filter(|row| matches!(row, TimelineRow::Item(_)))
            .count()
    }

    fn preview_width(&self, width: usize) -> usize {
        let prefix_width = [
            self.margin_for_width(width),
            1,
            visible_len(&self.marker),
            1,
            self.time_width_for_width(width),
            2,
            self.badge_width_for_width(width),
            2,
        ]
        .into_iter()
        .fold(0usize, usize::saturating_add);
        width.saturating_sub(prefix_width).max(1)
    }

    fn margin_for_width(&self, width: usize) -> usize {
        self.margin.min(width).min(MAX_TIMELINE_MARGIN)
    }

    fn time_width_for_width(&self, width: usize) -> usize {
        self.time_width.min(width).clamp(1, MAX_TIMELINE_TIME_WIDTH)
    }

    fn badge_width_for_width(&self, width: usize) -> usize {
        self.badge_width
            .min(width)
            .clamp(1, MAX_TIMELINE_BADGE_WIDTH)
    }

    fn fit_slot(&self, value: &str, width: usize, align_right: bool) -> String {
        if align_right {
            right_visible(value, width)
        } else {
            fit_visible(value, width)
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

    fn element_visible_width(element: &Element<()>) -> usize {
        match element {
            Element::Box(box_element) => {
                box_element.children.iter().map(element_visible_width).sum()
            }
            Element::Text(text) => visible_len(&text.content),
            _ => 0,
        }
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
    fn stale_selected_item_is_normalized_for_rendering_and_scroll() {
        let mut timeline = sample_timeline().scroll(0);
        timeline.selected_item = Some(usize::MAX);

        assert_eq!(timeline.selected_item_value(), Some(2));

        let rendered = timeline.view(40, 2);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("yesterday"), "{plain:?}");
        assert!(plain.contains("patched terminal"), "{plain:?}");
        assert!(!plain.contains("workspace uses"), "{plain:?}");

        let Element::Box(column) = timeline.element::<()>(40, 2) else {
            panic!("expected column element");
        };
        let Element::Text(selected) = &column.children[1] else {
            panic!("expected selected item");
        };
        assert_eq!(selected.style.fg, Some(Color::Black));
        assert_eq!(selected.style.bg, Some(Color::Green));
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
    fn oversized_spacing_is_clamped_to_render_width() {
        let timeline = Timeline::new()
            .margin(usize::MAX)
            .time_width(usize::MAX)
            .badge_width(usize::MAX)
            .section("today")
            .item(TimelineItem::new("2m", "fact", "workspace uses a3s-tui"));
        let rendered = timeline.view(8, 2);

        assert_eq!(timeline.margin, MAX_TIMELINE_MARGIN);
        assert_eq!(timeline.time_width, MAX_TIMELINE_TIME_WIDTH);
        assert_eq!(timeline.badge_width, MAX_TIMELINE_BADGE_WIDTH);
        assert!(rendered.lines().all(|line| visible_len(line) == 8));

        let Element::Box(column) = timeline.element::<()>(8, 2) else {
            panic!("expected column element");
        };
        let Element::Box(item) = &column.children[1] else {
            panic!("expected item row");
        };
        let Element::Text(margin) = &item.children[0] else {
            panic!("expected margin text");
        };
        assert_eq!(margin.content.len(), 8);
    }

    #[test]
    fn element_item_rows_fit_requested_width() {
        let element: Element<()> = Timeline::new()
            .marker("好")
            .item(TimelineItem::new("现在", "事实", "中文内容").color(Color::Cyan))
            .item(TimelineItem::new("稍后", "完成", "更多内容").color(Color::Green))
            .selected_item(0)
            .element(6, 2);
        let Element::Box(column) = element else {
            panic!("expected column");
        };

        assert_eq!(column.children.len(), 2);
        for row in &column.children {
            assert!(element_visible_width(row) <= 6);
        }
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
