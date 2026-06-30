use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, truncate_visible, visible_len, Color, Style};

/// One selectable level in a [`LevelSlider`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliderLevel {
    label: String,
    description: Option<String>,
    color: Option<Color>,
}

impl SliderLevel {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: None,
            color: None,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        let description = description.into();
        if !description.is_empty() {
            self.description = Some(description);
        }
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn label_value(&self) -> &str {
        &self.label
    }

    pub fn description_value(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn color_value(&self) -> Option<Color> {
        self.color
    }
}

/// Discrete level slider with tick labels and an optional separator.
///
/// This extracts the CLI effort picker pattern: a wide track, selected marker,
/// level labels centered under their ticks, optional "faster/smarter" range
/// labels, selected-level summary, and a hint row.
#[derive(Debug, Clone)]
pub struct LevelSlider {
    levels: Vec<SliderLevel>,
    selected: usize,
    title: Option<String>,
    left_label: Option<String>,
    right_label: Option<String>,
    hint: Option<String>,
    separator_after: Option<usize>,
    margin: usize,
    marker: char,
    track_char: char,
    separator_char: char,
    pointer: String,
    title_color: Color,
    selected_color: Color,
    track_color: Color,
    muted_color: Color,
}

impl LevelSlider {
    pub fn new(levels: Vec<SliderLevel>) -> Self {
        Self {
            levels,
            selected: 0,
            title: None,
            left_label: None,
            right_label: None,
            hint: None,
            separator_after: None,
            margin: 2,
            marker: '▲',
            track_char: '─',
            separator_char: '┆',
            pointer: "▸".to_string(),
            title_color: Color::Cyan,
            selected_color: Color::Cyan,
            track_color: Color::White,
            muted_color: Color::BrightBlack,
        }
    }

    pub fn from_labels(labels: Vec<impl Into<String>>) -> Self {
        Self::new(labels.into_iter().map(SliderLevel::new).collect())
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        let title = title.into();
        if !title.is_empty() {
            self.title = Some(title);
        }
        self
    }

    pub fn selected(mut self, selected: usize) -> Self {
        self.selected = selected.min(self.levels.len().saturating_sub(1));
        self
    }

    pub fn set_selected(&mut self, selected: usize) {
        self.selected = selected.min(self.levels.len().saturating_sub(1));
    }

    pub fn range_labels(mut self, left: impl Into<String>, right: impl Into<String>) -> Self {
        let left = left.into();
        let right = right.into();
        if !left.is_empty() {
            self.left_label = Some(left);
        }
        if !right.is_empty() {
            self.right_label = Some(right);
        }
        self
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        let hint = hint.into();
        if !hint.is_empty() {
            self.hint = Some(hint);
        }
        self
    }

    pub fn separator_after(mut self, index: usize) -> Self {
        if !self.levels.is_empty() && index + 1 < self.levels.len() {
            self.separator_after = Some(index);
        }
        self
    }

    pub fn margin(mut self, margin: usize) -> Self {
        self.margin = margin;
        self
    }

    pub fn marker(mut self, marker: char) -> Self {
        self.marker = marker;
        self
    }

    pub fn track_char(mut self, track_char: char) -> Self {
        self.track_char = track_char;
        self
    }

    pub fn separator_char(mut self, separator_char: char) -> Self {
        self.separator_char = separator_char;
        self
    }

    pub fn pointer(mut self, pointer: impl Into<String>) -> Self {
        let pointer = pointer.into();
        if !pointer.is_empty() {
            self.pointer = pointer;
        }
        self
    }

    pub fn title_color(mut self, color: Color) -> Self {
        self.title_color = color;
        self
    }

    pub fn selected_color(mut self, color: Color) -> Self {
        self.selected_color = color;
        self
    }

    pub fn track_color(mut self, color: Color) -> Self {
        self.track_color = color;
        self
    }

    pub fn muted_color(mut self, color: Color) -> Self {
        self.muted_color = color;
        self
    }

    pub fn levels_value(&self) -> &[SliderLevel] {
        &self.levels
    }

    pub fn selected_value(&self) -> usize {
        self.selected
    }

    pub fn view(&self, width: u16) -> String {
        let width = width as usize;
        if width == 0 || self.levels.is_empty() {
            return String::new();
        }

        self.render_lines(width).join("\n")
    }

    pub fn element<Msg>(&self, width: u16) -> Element<Msg> {
        let width = width as usize;
        if width == 0 || self.levels.is_empty() {
            return Element::Box(BoxElement::new().direction(FlexDirection::Column));
        }

        let mut children = Vec::new();
        if let Some(title) = self.title.as_deref() {
            children.push(Element::Text(
                TextElement::new(format!("{}{}", " ".repeat(self.margin), title))
                    .fg(self.title_color)
                    .bold(),
            ));
        }
        if self.left_label.is_some() || self.right_label.is_some() {
            children.push(Element::Text(
                TextElement::new(format!(
                    "{}{}",
                    " ".repeat(self.margin),
                    self.range_line(self.track_width(width))
                ))
                .fg(self.muted_color),
            ));
        }
        children.push(self.track_element(width));
        children.push(Element::Text(TextElement::new(format!(
            "{}{}",
            " ".repeat(self.margin),
            self.labels_plain(self.track_width(width))
        ))));
        children.push(Element::Text(
            TextElement::new(format!(
                "{}{} {}",
                " ".repeat(self.margin),
                self.pointer,
                self.selected_label()
            ))
            .fg(self.selected_level_color())
            .bold(),
        ));
        if let Some(description) = self.selected_description() {
            children.push(Element::Text(
                TextElement::new(format!("{}{}", " ".repeat(self.margin), description))
                    .fg(self.muted_color),
            ));
        }
        if let Some(hint) = self.hint.as_deref() {
            children.push(Element::Text(
                TextElement::new(format!("{}{}", " ".repeat(self.margin), hint))
                    .fg(self.muted_color),
            ));
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(children),
        )
    }

    fn render_lines(&self, width: usize) -> Vec<String> {
        let track_width = self.track_width(width);
        let margin = " ".repeat(self.margin);
        let mut lines = Vec::new();

        if let Some(title) = self.title.as_deref() {
            lines.push(fit_visible(
                &format!(
                    "{margin}{}",
                    Style::new().fg(self.title_color).bold().render(title)
                ),
                width,
            ));
        }
        if self.left_label.is_some() || self.right_label.is_some() {
            lines.push(fit_visible(
                &format!(
                    "{margin}{}",
                    Style::new()
                        .fg(self.muted_color)
                        .render(&self.range_line(track_width))
                ),
                width,
            ));
        }
        lines.push(fit_visible(
            &format!("{margin}{}", self.track_line(track_width)),
            width,
        ));
        lines.push(fit_visible(
            &format!("{margin}{}", self.labels_line(track_width)),
            width,
        ));
        lines.push(fit_visible(
            &format!(
                "{margin}{}",
                Style::new()
                    .fg(self.selected_level_color())
                    .bold()
                    .render(&format!("{} {}", self.pointer, self.selected_label()))
            ),
            width,
        ));
        if let Some(description) = self.selected_description() {
            lines.push(fit_visible(
                &format!(
                    "{margin}{}",
                    Style::new().fg(self.muted_color).render(description)
                ),
                width,
            ));
        }
        if let Some(hint) = self.hint.as_deref() {
            lines.push(fit_visible(
                &format!("{margin}{}", Style::new().fg(self.muted_color).render(hint)),
                width,
            ));
        }
        lines
    }

    fn track_element<Msg>(&self, width: usize) -> Element<Msg> {
        let track_width = self.track_width(width);
        let selected_pos = self.position_for(self.selected, track_width);
        let separator_pos = self.separator_position(track_width);
        let mut children = vec![Element::Text(TextElement::new(" ".repeat(self.margin)))];

        for index in 0..track_width {
            let (ch, color, bold) = if index == selected_pos {
                (self.marker, self.selected_level_color(), true)
            } else if Some(index) == separator_pos {
                (self.separator_char, self.muted_color, false)
            } else {
                (self.track_char, self.track_color, false)
            };
            let mut text = TextElement::new(ch.to_string()).fg(color);
            if bold {
                text = text.bold();
            }
            children.push(Element::Text(text));
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Row)
                .children(children),
        )
    }

    fn track_line(&self, track_width: usize) -> String {
        let selected_pos = self.position_for(self.selected, track_width);
        let separator_pos = self.separator_position(track_width);
        (0..track_width)
            .map(|index| {
                if index == selected_pos {
                    Style::new()
                        .fg(self.selected_level_color())
                        .bold()
                        .render(&self.marker.to_string())
                } else if Some(index) == separator_pos {
                    Style::new()
                        .fg(self.muted_color)
                        .render(&self.separator_char.to_string())
                } else {
                    Style::new()
                        .fg(self.track_color)
                        .render(&self.track_char.to_string())
                }
            })
            .collect()
    }

    fn labels_line(&self, track_width: usize) -> String {
        let mut out = String::new();
        let mut used = 0usize;

        for (index, level) in self.levels.iter().enumerate() {
            let label = truncate_visible(&level.label, track_width);
            let label_width = visible_len(&label);
            let start = self
                .position_for(index, track_width)
                .saturating_sub(label_width / 2);
            if used < start {
                out.push_str(&" ".repeat(start - used));
                used = start;
            } else if used > 0 {
                out.push(' ');
                used += 1;
            }
            let mut style = Style::new().fg(self.level_color(index));
            if index == self.selected {
                style = style.bold();
            }
            out.push_str(&style.render(&label));
            used += label_width;
        }

        fit_visible(&out, track_width)
    }

    fn labels_plain(&self, track_width: usize) -> String {
        crate::style::strip_ansi(&self.labels_line(track_width))
    }

    fn range_line(&self, track_width: usize) -> String {
        let left = self.left_label.as_deref().unwrap_or_default();
        let right = self.right_label.as_deref().unwrap_or_default();
        let left_width = visible_len(left);
        let right_width = visible_len(right);
        if left_width + right_width >= track_width {
            return fit_visible(&format!("{left} {right}"), track_width);
        }
        format!(
            "{left}{}{right}",
            " ".repeat(track_width - left_width - right_width)
        )
    }

    fn selected_description(&self) -> Option<&str> {
        self.levels
            .get(self.selected)
            .and_then(|level| level.description.as_deref())
    }

    fn selected_label(&self) -> &str {
        self.levels
            .get(self.selected)
            .map(|level| level.label.as_str())
            .unwrap_or("")
    }

    fn selected_level_color(&self) -> Color {
        self.level_color(self.selected)
    }

    fn level_color(&self, index: usize) -> Color {
        self.levels
            .get(index)
            .and_then(|level| level.color)
            .unwrap_or(if index == self.selected {
                self.selected_color
            } else {
                self.muted_color
            })
    }

    fn track_width(&self, width: usize) -> usize {
        width.saturating_sub(self.margin).max(1)
    }

    fn position_for(&self, index: usize, track_width: usize) -> usize {
        if self.levels.len() <= 1 || track_width <= 1 {
            return 0;
        }
        index.min(self.levels.len() - 1) * (track_width - 1) / (self.levels.len() - 1)
    }

    fn separator_position(&self, track_width: usize) -> Option<usize> {
        let index = self.separator_after?;
        if index + 1 >= self.levels.len() {
            return None;
        }
        Some(
            (self.position_for(index, track_width) + self.position_for(index + 1, track_width)) / 2,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::strip_ansi;

    fn sample_slider() -> LevelSlider {
        LevelSlider::new(vec![
            SliderLevel::new("low").color(Color::Green),
            SliderLevel::new("medium").color(Color::Cyan),
            SliderLevel::new("high")
                .description("higher effort = slower, deeper")
                .color(Color::Yellow),
            SliderLevel::new("ultra").color(Color::Magenta),
        ])
        .title("Effort")
        .range_labels("Faster", "Smarter")
        .selected(2)
        .separator_after(2)
        .hint("←/→ adjust · Enter confirm")
    }

    #[test]
    fn renders_discrete_track_labels_and_selected_level() {
        let rendered = sample_slider().view(72);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("Effort"));
        assert!(plain.contains("Faster"));
        assert!(plain.contains("Smarter"));
        assert!(plain.contains('▲'));
        assert!(plain.contains('┆'));
        assert!(plain.contains("▸ high"));
        assert!(plain.contains("higher effort"));
        for row in plain.lines() {
            assert_eq!(visible_len(row), 72, "{row:?}");
        }
    }

    #[test]
    fn selected_index_is_clamped() {
        let slider = LevelSlider::from_labels(vec!["a", "b"]).selected(99);

        assert_eq!(slider.selected_value(), 1);
    }

    #[test]
    fn empty_slider_renders_no_rows() {
        let slider = LevelSlider::new(Vec::new());

        assert_eq!(slider.view(80), "");
        assert!(matches!(slider.element::<()>(80), Element::Box(_)));
    }

    #[test]
    fn cjk_labels_fit_requested_width() {
        let rendered = LevelSlider::new(vec![
            SliderLevel::new("低"),
            SliderLevel::new("中"),
            SliderLevel::new("高级模式"),
        ])
        .selected(2)
        .view(24);

        for row in strip_ansi(&rendered).lines() {
            assert_eq!(visible_len(row), 24, "{row:?}");
        }
    }

    #[test]
    fn element_tracks_selected_marker_style() {
        let element: Element<()> = sample_slider().element(40);
        let Element::Box(column) = element else {
            panic!("expected column");
        };
        let Element::Box(track) = &column.children[2] else {
            panic!("expected track row");
        };

        let marker = track.children.iter().find_map(|child| match child {
            Element::Text(text) if text.content == "▲" => Some(text),
            _ => None,
        });
        let marker = marker.expect("selected marker should be a text segment");
        assert_eq!(marker.style.fg, Some(Color::Yellow));
        assert!(marker.style.bold);
    }
}
