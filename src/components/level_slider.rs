use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::event::{MouseButton, MouseEvent, MouseEventKind};
use crate::interaction::Selectable;
use crate::style::{fit_visible, repeat_visible_char, truncate_visible, visible_len, Color, Style};
use crate::theme::{Theme, ThemeRole};

const MAX_LEVEL_SLIDER_MARGIN: usize = u16::MAX as usize;

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
    y_offset: u16,
    title_color: Color,
    selected_color: Color,
    track_color: Color,
    muted_color: Color,
}

/// Message returned by [`LevelSlider`] mouse handlers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LevelSliderMsg {
    Selected(usize),
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
            y_offset: 0,
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

    pub fn set_y_offset(&mut self, y: u16) {
        self.y_offset = y;
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
        if index < self.levels.len().saturating_sub(1) {
            self.separator_after = Some(index);
        }
        self
    }

    pub fn margin(mut self, margin: usize) -> Self {
        self.margin = margin.min(MAX_LEVEL_SLIDER_MARGIN);
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

    /// Apply semantic colors from a theme while preserving levels and layout.
    pub fn with_theme(mut self, theme: &Theme) -> Self {
        self.title_color = theme.color(ThemeRole::Primary);
        self.selected_color = theme.color(ThemeRole::Primary);
        self.track_color = theme.color(ThemeRole::Border);
        self.muted_color = theme.color(ThemeRole::Muted);
        self
    }

    pub fn levels_value(&self) -> &[SliderLevel] {
        &self.levels
    }

    pub fn selected_value(&self) -> usize {
        self.selected_index()
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
        let margin = self.margin_for_width(width);
        if let Some(title) = self.title.as_deref() {
            children.push(Element::Text(
                TextElement::new(fit_visible(
                    &format!("{}{}", " ".repeat(margin), title),
                    width,
                ))
                .fg(self.title_color)
                .bold(),
            ));
        }
        if self.left_label.is_some() || self.right_label.is_some() {
            children.push(Element::Text(
                TextElement::new(fit_visible(
                    &format!(
                        "{}{}",
                        " ".repeat(margin),
                        self.range_line(self.track_width(width))
                    ),
                    width,
                ))
                .fg(self.muted_color),
            ));
        }
        children.push(self.track_element(width));
        children.push(Element::Text(TextElement::new(fit_visible(
            &format!(
                "{}{}",
                " ".repeat(margin),
                self.labels_plain(self.track_width(width))
            ),
            width,
        ))));
        children.push(Element::Text(
            TextElement::new(fit_visible(
                &format!(
                    "{}{} {}",
                    " ".repeat(margin),
                    self.pointer,
                    self.selected_label()
                ),
                width,
            ))
            .fg(self.selected_level_color())
            .bold(),
        ));
        if let Some(description) = self.selected_description() {
            children.push(Element::Text(
                TextElement::new(fit_visible(
                    &format!("{}{}", " ".repeat(margin), description),
                    width,
                ))
                .fg(self.muted_color),
            ));
        }
        if let Some(hint) = self.hint.as_deref() {
            children.push(Element::Text(
                TextElement::new(fit_visible(
                    &format!("{}{}", " ".repeat(margin), hint),
                    width,
                ))
                .fg(self.muted_color),
            ));
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(children),
        )
    }

    pub fn handle_mouse(&mut self, mouse: &MouseEvent, width: u16) -> Option<LevelSliderMsg> {
        let width = width as usize;
        if width == 0 || self.levels.is_empty() {
            return None;
        }
        let local_row = super::relative_mouse_row(mouse.row, self.y_offset)?;
        if local_row >= self.row_count() {
            return None;
        }

        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.set_selected(self.selected_index().saturating_sub(1));
                None
            }
            MouseEventKind::ScrollDown => {
                self.set_selected(self.selected_index().saturating_add(1));
                None
            }
            MouseEventKind::Down(MouseButton::Left) => {
                if !self.is_clickable_row(local_row) {
                    return None;
                }
                let selected = self.index_for_column(mouse.column, width)?;
                self.set_selected(selected);
                Some(LevelSliderMsg::Selected(selected))
            }
            _ => None,
        }
    }

    fn render_lines(&self, width: usize) -> Vec<String> {
        let track_width = self.track_width(width);
        let margin = " ".repeat(self.margin_for_width(width));
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
        let mut children = Vec::new();
        let mut used = 0usize;
        Self::push_limited_text(
            &mut children,
            " ".repeat(self.margin_for_width(width)),
            None,
            false,
            &mut used,
            width,
        );

        for segment in self.track_segments(track_width) {
            Self::push_limited_text(
                &mut children,
                segment.text,
                Some(segment.color),
                segment.bold,
                &mut used,
                width,
            );
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Row)
                .children(children),
        )
    }

    fn push_limited_text<Msg>(
        children: &mut Vec<Element<Msg>>,
        text: String,
        fg: Option<Color>,
        bold: bool,
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
        let mut element = TextElement::new(clipped);
        if let Some(color) = fg {
            element = element.fg(color);
        }
        if bold {
            element = element.bold();
        }
        children.push(Element::Text(element));
    }

    fn track_line(&self, track_width: usize) -> String {
        self.track_segments(track_width)
            .into_iter()
            .map(|segment| {
                let mut style = Style::new().fg(segment.color);
                if segment.bold {
                    style = style.bold();
                }
                style.render(&segment.text)
            })
            .collect()
    }

    fn track_segments(&self, track_width: usize) -> Vec<TrackSegment> {
        let selected_pos = feature_position(
            self.position_for(self.selected_index(), track_width),
            self.marker,
            track_width,
        );
        let separator_pos = self
            .separator_position(track_width)
            .map(|position| feature_position(position, self.separator_char, track_width));
        let selected_width =
            glyph_slot_width(self.marker, track_width.saturating_sub(selected_pos));
        let mut features = Vec::new();

        if let Some(separator_pos) = separator_pos {
            let separator_width = glyph_slot_width(
                self.separator_char,
                track_width.saturating_sub(separator_pos),
            );
            let selected_end = selected_pos.saturating_add(selected_width);
            let separator_end = separator_pos.saturating_add(separator_width);
            let overlaps_selected = separator_pos < selected_end && selected_pos < separator_end;
            if !overlaps_selected {
                features.push(TrackFeature {
                    position: separator_pos,
                    ch: self.separator_char,
                    color: self.muted_color,
                    bold: false,
                });
            }
        }
        features.push(TrackFeature {
            position: selected_pos,
            ch: self.marker,
            color: self.selected_level_color(),
            bold: true,
        });
        features.sort_by_key(|feature| feature.position);

        let mut segments = Vec::new();
        let mut used = 0usize;
        for feature in features {
            if feature.position >= track_width {
                continue;
            }
            if feature.position < used {
                continue;
            }
            if used < feature.position {
                segments.push(TrackSegment {
                    text: repeat_visible_char(self.track_char, feature.position - used),
                    color: self.track_color,
                    bold: false,
                });
                used = feature.position;
            }

            let width = glyph_slot_width(feature.ch, track_width - used);
            segments.push(TrackSegment {
                text: repeat_visible_char(feature.ch, width),
                color: feature.color,
                bold: feature.bold,
            });
            used += width;
        }

        if used < track_width {
            segments.push(TrackSegment {
                text: repeat_visible_char(self.track_char, track_width - used),
                color: self.track_color,
                bold: false,
            });
        }

        segments
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
            if index == self.selected_index() {
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
            .get(self.selected_index())
            .and_then(|level| level.description.as_deref())
    }

    fn selected_label(&self) -> &str {
        self.levels
            .get(self.selected_index())
            .map(|level| level.label.as_str())
            .unwrap_or("")
    }

    fn selected_level_color(&self) -> Color {
        self.level_color(self.selected_index())
    }

    fn level_color(&self, index: usize) -> Color {
        let selected = self.selected_index();
        self.levels
            .get(index)
            .and_then(|level| level.color)
            .unwrap_or(if index == selected {
                self.selected_color
            } else {
                self.muted_color
            })
    }

    fn track_width(&self, width: usize) -> usize {
        width.saturating_sub(self.margin_for_width(width)).max(1)
    }

    fn position_for(&self, index: usize, track_width: usize) -> usize {
        if self.levels.len() <= 1 || track_width <= 1 {
            return 0;
        }
        index
            .min(self.levels.len() - 1)
            .saturating_mul(track_width - 1)
            / (self.levels.len() - 1)
    }

    fn separator_position(&self, track_width: usize) -> Option<usize> {
        let index = self.separator_after?;
        let next = index.checked_add(1)?;
        if next >= self.levels.len() {
            return None;
        }
        Some((self.position_for(index, track_width) + self.position_for(next, track_width)) / 2)
    }

    fn selected_index(&self) -> usize {
        self.selected.min(self.levels.len().saturating_sub(1))
    }

    fn margin_for_width(&self, width: usize) -> usize {
        self.margin.min(width).min(MAX_LEVEL_SLIDER_MARGIN)
    }

    fn row_count(&self) -> usize {
        let mut count = 3;
        if self.title.is_some() {
            count += 1;
        }
        if self.left_label.is_some() || self.right_label.is_some() {
            count += 1;
        }
        if self.selected_description().is_some() {
            count += 1;
        }
        if self.hint.is_some() {
            count += 1;
        }
        count
    }

    fn track_row_index(&self) -> usize {
        let title_row = if self.title.is_some() { 1 } else { 0 };
        let range_row = if self.left_label.is_some() || self.right_label.is_some() {
            1
        } else {
            0
        };
        title_row + range_row
    }

    fn is_clickable_row(&self, row: usize) -> bool {
        let track_row = self.track_row_index();
        row == track_row || row == track_row.saturating_add(1)
    }

    fn index_for_column(&self, column: u16, width: usize) -> Option<usize> {
        let margin = self.margin_for_width(width);
        let column = usize::from(column);
        let track_width = self.track_width(width);
        let local_column = column.checked_sub(margin)?;
        if local_column >= track_width {
            return None;
        }
        if self.levels.len() <= 1 {
            return Some(0);
        }

        (0..self.levels.len()).min_by_key(|index| {
            let position = self.position_for(*index, track_width);
            position.abs_diff(local_column)
        })
    }
}

impl Selectable for LevelSlider {
    fn item_count(&self) -> usize {
        self.levels.len()
    }

    fn selected_index(&self) -> Option<usize> {
        (!self.levels.is_empty()).then(|| LevelSlider::selected_index(self))
    }

    fn select_index(&mut self, index: usize) {
        self.set_selected(index);
    }
}

#[derive(Debug, Clone)]
struct TrackFeature {
    position: usize,
    ch: char,
    color: Color,
    bold: bool,
}

#[derive(Debug, Clone)]
struct TrackSegment {
    text: String,
    color: Color,
    bold: bool,
}

fn glyph_slot_width(ch: char, remaining: usize) -> usize {
    if remaining == 0 {
        return 0;
    }

    glyph_display_width(ch).min(remaining)
}

fn glyph_display_width(ch: char) -> usize {
    visible_len(&ch.to_string()).max(1)
}

fn feature_position(position: usize, ch: char, track_width: usize) -> usize {
    if track_width == 0 {
        return 0;
    }

    let width = glyph_display_width(ch).min(track_width);
    position.min(track_width - width)
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
    fn with_theme_applies_semantic_colors() {
        let theme = Theme::tokyo_night();
        let slider = sample_slider().with_theme(&theme);

        assert_eq!(slider.title_color, theme.color(ThemeRole::Primary));
        assert_eq!(slider.selected_color, theme.color(ThemeRole::Primary));
        assert_eq!(slider.track_color, theme.color(ThemeRole::Border));
        assert_eq!(slider.muted_color, theme.color(ThemeRole::Muted));
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
    fn stale_selected_index_is_clamped_during_rendering() {
        let mut slider = LevelSlider::from_labels(vec!["a", "b"]);
        slider.selected = usize::MAX;

        let rendered = slider.view(24);

        assert_eq!(slider.selected_value(), 1);
        assert!(strip_ansi(&rendered).contains("▸ b"));
    }

    #[test]
    fn oversized_separator_index_is_ignored() {
        let mut slider = LevelSlider::from_labels(vec!["a", "b", "c"]).separator_after(usize::MAX);
        assert_eq!(slider.separator_after, None);

        slider.separator_after = Some(usize::MAX);
        let rendered = slider.view(16);

        assert!(!strip_ansi(&rendered).contains('┆'));
    }

    #[test]
    fn oversized_margin_is_clamped_to_render_width() {
        let slider = LevelSlider::from_labels(vec!["a", "b"]).margin(usize::MAX);
        let rendered = slider.view(8);

        assert_eq!(slider.margin, MAX_LEVEL_SLIDER_MARGIN);
        for row in strip_ansi(&rendered).lines() {
            assert_eq!(visible_len(row), 8, "{row:?}");
        }

        let Element::Box(column) = slider.element::<()>(8) else {
            panic!("expected column");
        };
        assert!(!column.children.is_empty());
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
    fn custom_wide_track_glyphs_fit_requested_width() {
        let rendered = LevelSlider::from_labels(vec!["low", "mid", "high"])
            .track_char('界')
            .separator_char('中')
            .marker('好')
            .separator_after(0)
            .selected(1)
            .view(18);
        let plain = strip_ansi(&rendered);
        let track = plain.lines().next().unwrap();

        assert_eq!(visible_len(track), 18);
        assert!(track.contains('界'));
        assert!(track.contains('中'));
        assert!(track.contains('好'));
        for row in plain.lines() {
            assert_eq!(visible_len(row), 18, "{row:?}");
        }
    }

    #[test]
    fn element_track_glyphs_use_display_width() {
        let element: Element<()> = LevelSlider::from_labels(vec!["a", "b", "c"])
            .track_char('界')
            .separator_char('\u{301}')
            .marker('好')
            .separator_after(0)
            .selected(2)
            .element(18);

        let Element::Box(column) = element else {
            panic!("expected column");
        };
        let Element::Box(track) = &column.children[0] else {
            panic!("expected track row");
        };
        let row_width = track
            .children
            .iter()
            .map(|child| match child {
                Element::Text(text) => visible_len(&text.content),
                _ => 0,
            })
            .sum::<usize>();
        let row_text = track
            .children
            .iter()
            .filter_map(|child| match child {
                Element::Text(text) => Some(text.content.as_str()),
                _ => None,
            })
            .collect::<String>();

        assert_eq!(row_width, 18);
        assert!(row_text.contains('好'));
    }

    #[test]
    fn element_rows_fit_requested_width() {
        let element: Element<()> = LevelSlider::new(vec![
            SliderLevel::new("低").description("非常长的说明文字"),
            SliderLevel::new("高级模式"),
        ])
        .title("非常长的标题")
        .range_labels("左侧很长", "右侧也很长")
        .hint("提示文字也很长")
        .margin(usize::MAX)
        .selected(0)
        .element(8);
        let Element::Box(column) = element else {
            panic!("expected column");
        };

        for row in &column.children {
            assert!(element_visible_width(row) <= 8);
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

    #[test]
    fn mouse_wheel_updates_selected_level() {
        let mut slider = sample_slider();

        let msg = slider.handle_mouse(
            &MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column: 0,
                row: 2,
                modifiers: crate::KeyModifiers::NONE,
            },
            48,
        );

        assert_eq!(msg, None);
        assert_eq!(slider.selected_value(), 3);

        slider.handle_mouse(
            &MouseEvent {
                kind: MouseEventKind::ScrollUp,
                column: 0,
                row: 2,
                modifiers: crate::KeyModifiers::NONE,
            },
            48,
        );

        assert_eq!(slider.selected_value(), 2);
    }

    #[test]
    fn mouse_click_on_track_selects_nearest_level() {
        let mut slider = sample_slider().selected(0);
        let width = 48;
        let track_width = slider.track_width(width);
        let column = slider.margin_for_width(width) + slider.position_for(2, track_width);

        let msg = slider.handle_mouse(
            &MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: column as u16,
                row: slider.track_row_index() as u16,
                modifiers: crate::KeyModifiers::NONE,
            },
            width as u16,
        );

        assert_eq!(msg, Some(LevelSliderMsg::Selected(2)));
        assert_eq!(slider.selected_value(), 2);
    }

    #[test]
    fn mouse_click_above_offset_is_ignored() {
        let mut slider = sample_slider().selected(1);
        slider.set_y_offset(4);

        let msg = slider.handle_mouse(
            &MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 4,
                row: 3,
                modifiers: crate::KeyModifiers::NONE,
            },
            48,
        );

        assert_eq!(msg, None);
        assert_eq!(slider.selected_value(), 1);
    }
}
