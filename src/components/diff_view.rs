use crate::element::{BoxElement, Element, FlexDirection, TextElement, TextWrap};
use crate::style::{
    fit_visible, next_display_cell_boundary, truncate_visible, visible_len, Color, Style,
};
use crate::theme::{Theme, ThemeRole};
use similar::{ChangeTag, TextDiff};

const MAX_DIFF_VIEW_CONTEXT_LINES: usize = u16::MAX as usize;
const MAX_DIFF_VIEW_LINES: usize = u16::MAX as usize;

/// Visual kind for one diff row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    Context,
    Insert,
    Delete,
    Hunk,
    Metadata,
    Separator,
}

/// One syntax-colored span inside a diff line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffSpan {
    content: String,
    color: Option<Color>,
}

impl DiffSpan {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            color: None,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn foreground(&self) -> Option<Color> {
        self.color
    }
}

impl DiffLineKind {
    fn marker(self) -> char {
        match self {
            DiffLineKind::Insert => '+',
            DiffLineKind::Delete => '-',
            DiffLineKind::Context => ' ',
            DiffLineKind::Hunk | DiffLineKind::Metadata | DiffLineKind::Separator => ' ',
        }
    }
}

/// One row in a [`DiffView`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    old_number: Option<usize>,
    new_number: Option<usize>,
    kind: DiffLineKind,
    content: String,
    spans: Vec<DiffSpan>,
}

impl DiffLine {
    pub fn new(kind: DiffLineKind, content: impl Into<String>) -> Self {
        Self {
            old_number: None,
            new_number: None,
            kind,
            content: content.into(),
            spans: Vec::new(),
        }
    }

    pub fn numbers(mut self, old_number: Option<usize>, new_number: Option<usize>) -> Self {
        self.old_number = old_number;
        self.new_number = new_number;
        self
    }

    pub fn kind(&self) -> DiffLineKind {
        self.kind
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn old_number(&self) -> Option<usize> {
        self.old_number
    }

    pub fn new_number(&self) -> Option<usize> {
        self.new_number
    }
}

/// Width-aware unified diff renderer for edits and tool output.
///
/// `DiffView` can render pre-existing unified diff lines or compute a compact
/// grouped diff from before/after text. It uses a stable line-number gutter,
/// clear insertion/deletion colors, optional full-row backgrounds for changed
/// lines, and CJK-safe truncation/wrapping.
#[derive(Debug, Clone)]
pub struct DiffView {
    path: Option<String>,
    action: String,
    lines: Vec<DiffLine>,
    added: usize,
    deleted: usize,
    max_lines: usize,
    context_lines: usize,
    wrap: bool,
    fill_height: bool,
    header_color: Color,
    header_bullet_color: Color,
    header_action_color: Color,
    header_detail_color: Color,
    meta_color: Color,
    hunk_color: Color,
    context_color: Color,
    context_gutter_color: Color,
    insert_fg: Color,
    insert_marker_color: Color,
    insert_gutter_color: Color,
    insert_bg: Option<Color>,
    delete_fg: Color,
    delete_marker_color: Color,
    delete_gutter_color: Color,
    delete_bg: Option<Color>,
    separator_color: Color,
}

impl DiffView {
    pub fn new(lines: Vec<DiffLine>) -> Self {
        let added = lines
            .iter()
            .filter(|line| line.kind == DiffLineKind::Insert)
            .count();
        let deleted = lines
            .iter()
            .filter(|line| line.kind == DiffLineKind::Delete)
            .count();
        Self {
            path: None,
            action: "Edited".to_string(),
            lines,
            added,
            deleted,
            max_lines: 200,
            context_lines: 3,
            wrap: true,
            fill_height: false,
            header_color: Color::Green,
            header_bullet_color: Color::Green,
            header_action_color: Color::Green,
            header_detail_color: Color::Green,
            meta_color: Color::BrightBlack,
            hunk_color: Color::Cyan,
            context_color: Color::BrightBlack,
            context_gutter_color: Color::BrightBlack,
            insert_fg: Color::Rgb(205, 255, 210),
            insert_marker_color: Color::Rgb(205, 255, 210),
            insert_gutter_color: Color::Rgb(205, 255, 210),
            insert_bg: Some(Color::Rgb(26, 60, 36)),
            delete_fg: Color::Rgb(255, 215, 215),
            delete_marker_color: Color::Rgb(255, 215, 215),
            delete_gutter_color: Color::Rgb(255, 215, 215),
            delete_bg: Some(Color::Rgb(82, 30, 34)),
            separator_color: Color::BrightBlack,
        }
    }

    pub fn from_texts(
        path: impl Into<String>,
        before: impl AsRef<str>,
        after: impl AsRef<str>,
    ) -> Self {
        Self::from_texts_with_context(path, before, after, 3)
    }

    pub fn from_texts_with_context(
        path: impl Into<String>,
        before: impl AsRef<str>,
        after: impl AsRef<str>,
        context_lines: usize,
    ) -> Self {
        let before = before.as_ref();
        let after = after.as_ref();
        let context_lines = context_lines.min(MAX_DIFF_VIEW_CONTEXT_LINES);
        let diff = TextDiff::from_lines(before, after);
        let mut lines = Vec::new();
        let mut added = 0usize;
        let mut deleted = 0usize;

        for (group_index, group) in diff.grouped_ops(context_lines).iter().enumerate() {
            if group_index > 0 {
                lines.push(DiffLine::new(DiffLineKind::Separator, "⋮"));
            }
            for op in group {
                for change in diff.iter_changes(op) {
                    let raw = trim_newline(change.value());
                    match change.tag() {
                        ChangeTag::Insert => {
                            added += 1;
                            lines.push(
                                DiffLine::new(DiffLineKind::Insert, raw)
                                    .numbers(None, change.new_index().map(|index| index + 1)),
                            );
                        }
                        ChangeTag::Delete => {
                            deleted += 1;
                            lines.push(
                                DiffLine::new(DiffLineKind::Delete, raw)
                                    .numbers(change.old_index().map(|index| index + 1), None),
                            );
                        }
                        ChangeTag::Equal => {
                            lines.push(DiffLine::new(DiffLineKind::Context, raw).numbers(
                                change.old_index().map(|index| index + 1),
                                change.new_index().map(|index| index + 1),
                            ));
                        }
                    }
                }
            }
        }

        Self::new(lines)
            .path(path)
            .context_lines(context_lines)
            .counts(added, deleted)
    }

    pub fn from_unified_lines(lines: Vec<impl Into<String>>) -> Self {
        let mut parsed = Vec::new();
        let mut old_line = None;
        let mut new_line = None;

        for line in lines.into_iter().map(Into::into) {
            if line.starts_with("@@") {
                if let Some((old_start, new_start)) = parse_hunk_starts(&line) {
                    old_line = Some(old_start);
                    new_line = Some(new_start);
                } else {
                    old_line = None;
                    new_line = None;
                }
                parsed.push(DiffLine::new(DiffLineKind::Hunk, line));
                continue;
            }

            if line.starts_with("diff ")
                || line.starts_with("index ")
                || line.starts_with("--- ")
                || line.starts_with("+++ ")
                || line.starts_with("\\ ")
            {
                parsed.push(DiffLine::new(DiffLineKind::Metadata, line));
                continue;
            }

            if let Some(content) = line.strip_prefix('+') {
                let number = new_line;
                if let Some(next) = &mut new_line {
                    *next += 1;
                }
                parsed.push(DiffLine::new(DiffLineKind::Insert, content).numbers(None, number));
                continue;
            }

            if let Some(content) = line.strip_prefix('-') {
                let number = old_line;
                if let Some(next) = &mut old_line {
                    *next += 1;
                }
                parsed.push(DiffLine::new(DiffLineKind::Delete, content).numbers(number, None));
                continue;
            }

            let old_number = old_line;
            let new_number = new_line;
            if let Some(next) = &mut old_line {
                *next += 1;
            }
            if let Some(next) = &mut new_line {
                *next += 1;
            }
            parsed.push(
                DiffLine::new(
                    DiffLineKind::Context,
                    line.strip_prefix(' ').unwrap_or(&line),
                )
                .numbers(old_number, new_number),
            );
        }
        Self::new(parsed)
    }

    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Set the completed file-change verb shown in the header. The action is
    /// included before width fitting so `Added`, `Edited`, and `Deleted` all
    /// preserve the same exact row-width contract.
    pub fn action(mut self, action: impl Into<String>) -> Self {
        let action = action.into();
        let action = action.trim();
        if !action.is_empty() && !action.contains(['\r', '\n']) {
            self.action = action.to_string();
        }
        self
    }

    pub fn counts(mut self, added: usize, deleted: usize) -> Self {
        self.added = added;
        self.deleted = deleted;
        self
    }

    pub fn max_lines(mut self, max_lines: usize) -> Self {
        self.max_lines = max_lines.clamp(1, MAX_DIFF_VIEW_LINES);
        self
    }

    pub fn context_lines(mut self, context_lines: usize) -> Self {
        self.context_lines = context_lines.min(MAX_DIFF_VIEW_CONTEXT_LINES);
        self
    }

    pub fn wrap(mut self, enabled: bool) -> Self {
        self.wrap = enabled;
        self
    }

    pub fn fill_height(mut self, enabled: bool) -> Self {
        self.fill_height = enabled;
        self
    }

    pub fn changed_backgrounds(mut self, insert: Option<Color>, delete: Option<Color>) -> Self {
        self.insert_bg = insert;
        self.delete_bg = delete;
        self
    }

    pub fn header_color(mut self, color: Color) -> Self {
        self.header_color = color;
        self.header_bullet_color = color;
        self.header_action_color = color;
        self.header_detail_color = color;
        self
    }

    pub fn header_colors(mut self, bullet: Color, action: Color, detail: Color) -> Self {
        self.header_color = detail;
        self.header_bullet_color = bullet;
        self.header_action_color = action;
        self.header_detail_color = detail;
        self
    }

    pub fn meta_color(mut self, color: Color) -> Self {
        self.meta_color = color;
        self
    }

    pub fn hunk_color(mut self, color: Color) -> Self {
        self.hunk_color = color;
        self
    }

    pub fn context_color(mut self, color: Color) -> Self {
        self.context_color = color;
        self
    }

    pub fn gutter_colors(mut self, context: Color, insert: Color, delete: Color) -> Self {
        self.context_gutter_color = context;
        self.insert_gutter_color = insert;
        self.delete_gutter_color = delete;
        self
    }

    pub fn separator_color(mut self, color: Color) -> Self {
        self.separator_color = color;
        self
    }

    pub fn insert_color(mut self, color: Color) -> Self {
        self.insert_fg = color;
        self.insert_marker_color = color;
        self
    }

    pub fn delete_color(mut self, color: Color) -> Self {
        self.delete_fg = color;
        self.delete_marker_color = color;
        self
    }

    pub fn changed_content_colors(mut self, insert: Color, delete: Color) -> Self {
        self.insert_fg = insert;
        self.delete_fg = delete;
        self
    }

    pub fn marker_colors(mut self, insert: Color, delete: Color) -> Self {
        self.insert_marker_color = insert;
        self.delete_marker_color = delete;
        self
    }

    /// Eagerly attach syntax-colored spans to each diff row.
    ///
    /// The callback must preserve the row's text. Invalid span output is
    /// ignored so the diff remains the authoritative source for layout.
    pub fn highlight_content<F>(mut self, mut highlighter: F) -> Self
    where
        F: FnMut(DiffLineKind, &str) -> Vec<DiffSpan>,
    {
        for line in &mut self.lines {
            let spans = highlighter(line.kind, &line.content);
            let content = spans.iter().map(DiffSpan::content).collect::<String>();
            if content == line.content {
                line.spans = spans;
            }
        }
        self
    }

    pub fn with_theme(mut self, theme: &Theme) -> Self {
        self.header_color = theme.color(ThemeRole::Primary);
        self.header_bullet_color = theme.color(ThemeRole::Primary);
        self.header_action_color = theme.color(ThemeRole::Primary);
        self.header_detail_color = theme.color(ThemeRole::Primary);
        self.meta_color = theme.color(ThemeRole::Muted);
        self.hunk_color = theme.color(ThemeRole::Info);
        self.context_color = theme.color(ThemeRole::Foreground);
        self.context_gutter_color = theme.color(ThemeRole::Foreground);
        self.insert_fg = theme.color(ThemeRole::Success);
        self.insert_marker_color = theme.color(ThemeRole::Success);
        self.insert_gutter_color = theme.color(ThemeRole::Success);
        self.insert_bg = Some(theme.color(ThemeRole::Surface));
        self.delete_fg = theme.color(ThemeRole::Error);
        self.delete_marker_color = theme.color(ThemeRole::Error);
        self.delete_gutter_color = theme.color(ThemeRole::Error);
        self.delete_bg = Some(theme.color(ThemeRole::Surface));
        self.separator_color = theme.color(ThemeRole::Border);
        self
    }

    pub fn lines(&self) -> &[DiffLine] {
        &self.lines
    }

    pub fn added(&self) -> usize {
        self.added
    }

    pub fn deleted(&self) -> usize {
        self.deleted
    }

    pub fn view(&self, width: u16, height: usize) -> String {
        let width = width as usize;
        if width == 0 || height == 0 {
            return String::new();
        }

        let mut lines = self.render_lines(width, height);
        lines.truncate(height);
        if self.fill_height {
            while lines.len() < height {
                lines.push(String::new());
            }
        }

        lines
            .into_iter()
            .map(|line| fit_visible(&line, width))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        let mut children = Vec::new();
        if let Some(header) = self.header_element() {
            children.push(header);
        }
        for line in self.lines.iter().take(self.max_lines) {
            children.push(Element::Text(self.line_element(line)));
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(children),
        )
    }

    pub fn element_with_height<Msg>(&self, height: usize) -> Element<Msg> {
        let mut children = Vec::new();
        if height == 0 {
            return Element::Box(BoxElement::new().direction(FlexDirection::Column));
        }

        if let Some(header) = self.header_element() {
            children.push(header);
        }

        let available = height.saturating_sub(children.len());
        let limit = self.max_lines.min(available).min(self.lines.len());
        for line in self.lines.iter().take(limit) {
            children.push(Element::Text(self.line_element(line)));
        }

        if self.lines.len() > limit && children.len() < height {
            children.push(Element::Text(self.truncation_notice_element()));
        }
        children.truncate(height);

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(children),
        )
    }

    fn render_lines(&self, width: usize, height: usize) -> Vec<String> {
        let mut lines = Vec::new();
        if let Some(header) = self.styled_header_for_width(width) {
            lines.push(header);
        }

        let number_width = self.number_width();
        let code_column = 4 + number_width + 3;
        let code_width = width.saturating_sub(code_column).max(8);
        let mut rendered = 0usize;
        let available = height.saturating_sub(lines.len());

        for line in &self.lines {
            if rendered >= self.max_lines || rendered >= available {
                break;
            }
            for segment in self.render_line_segments(line, number_width, code_width, width) {
                if rendered >= self.max_lines || rendered >= available {
                    break;
                }
                lines.push(segment);
                rendered += 1;
            }
        }

        let truncated_by_height = self.rendered_line_count(code_width) > rendered;
        if (self.lines.len() > self.max_lines || truncated_by_height) && lines.len() < height {
            lines.push(
                Style::new()
                    .fg(self.separator_color)
                    .render(&fit_visible("    … (diff truncated)", width)),
            );
        }

        lines
    }

    fn render_line_segments(
        &self,
        line: &DiffLine,
        number_width: usize,
        code_width: usize,
        row_width: usize,
    ) -> Vec<String> {
        let spans = if line.spans.is_empty() {
            vec![DiffSpan::new(&line.content)]
        } else {
            line.spans.clone()
        };
        let segments = if self.wrap {
            wrap_diff_spans(&spans, code_width)
        } else {
            vec![truncate_diff_spans(&spans, code_width)]
        };

        segments
            .into_iter()
            .enumerate()
            .map(|(segment_index, segment)| {
                self.style_segment(line, number_width, segment_index, &segment, row_width)
            })
            .collect()
    }

    fn style_segment(
        &self,
        line: &DiffLine,
        number_width: usize,
        segment_index: usize,
        spans: &[DiffSpan],
        row_width: usize,
    ) -> String {
        let bg = self.line_background(line.kind);
        let mut rendered = style_piece("    ", None, bg);

        if segment_index == 0 {
            rendered.push_str(&style_piece(
                &self.line_number(line, number_width),
                Some(self.gutter_color(line.kind)),
                bg,
            ));
            rendered.push_str(&style_piece(" ", None, bg));
            rendered.push_str(&style_piece(
                &line.kind.marker().to_string(),
                Some(self.marker_color(line.kind)),
                bg,
            ));
            rendered.push_str(&style_piece(" ", None, bg));
        } else {
            rendered.push_str(&style_piece(&" ".repeat(number_width + 3), None, bg));
        }

        for span in spans {
            let color = span.color.unwrap_or_else(|| self.content_color(line.kind));
            rendered.push_str(&style_piece(&span.content, Some(color), bg));
        }

        let padding = row_width.saturating_sub(visible_len(&rendered));
        rendered.push_str(&style_piece(&" ".repeat(padding), None, bg));
        rendered
    }

    fn line_background(&self, kind: DiffLineKind) -> Option<Color> {
        match kind {
            DiffLineKind::Insert => self.insert_bg,
            DiffLineKind::Delete => self.delete_bg,
            _ => None,
        }
    }

    fn gutter_color(&self, kind: DiffLineKind) -> Color {
        match kind {
            DiffLineKind::Insert => self.insert_gutter_color,
            DiffLineKind::Delete => self.delete_gutter_color,
            _ => self.context_gutter_color,
        }
    }

    fn marker_color(&self, kind: DiffLineKind) -> Color {
        match kind {
            DiffLineKind::Insert => self.insert_marker_color,
            DiffLineKind::Delete => self.delete_marker_color,
            DiffLineKind::Hunk => self.hunk_color,
            DiffLineKind::Metadata => self.meta_color,
            DiffLineKind::Separator => self.separator_color,
            DiffLineKind::Context => self.context_gutter_color,
        }
    }

    fn content_color(&self, kind: DiffLineKind) -> Color {
        match kind {
            DiffLineKind::Insert => self.insert_fg,
            DiffLineKind::Delete => self.delete_fg,
            DiffLineKind::Hunk => self.hunk_color,
            DiffLineKind::Metadata => self.meta_color,
            DiffLineKind::Separator => self.separator_color,
            DiffLineKind::Context => self.context_color,
        }
    }

    fn line_element(&self, line: &DiffLine) -> TextElement {
        let mut text = TextElement::new(self.plain_line(line));
        match line.kind {
            DiffLineKind::Insert => {
                text = text.fg(self.insert_fg);
                if let Some(bg) = self.insert_bg {
                    text = text.bg(bg);
                }
            }
            DiffLineKind::Delete => {
                text = text.fg(self.delete_fg);
                if let Some(bg) = self.delete_bg {
                    text = text.bg(bg);
                }
            }
            DiffLineKind::Hunk => text = text.fg(self.hunk_color),
            DiffLineKind::Metadata | DiffLineKind::Separator => text = text.fg(self.meta_color),
            DiffLineKind::Context => text = text.fg(self.context_color),
        }
        text
    }

    fn truncation_notice_element(&self) -> TextElement {
        TextElement::new("    … (diff truncated)").fg(self.separator_color)
    }

    fn header_element<Msg>(&self) -> Option<Element<Msg>> {
        let path = self.path.as_deref()?;
        let detail = |text| {
            Element::Text(
                TextElement::new(text)
                    .fg(self.header_detail_color)
                    .bold()
                    .wrap(TextWrap::NoWrap),
            )
        };
        let count = |text, color| {
            Element::Text(
                TextElement::new(text)
                    .fg(color)
                    .bold()
                    .wrap(TextWrap::NoWrap),
            )
        };
        let counts = Element::Box(BoxElement::row().children(vec![
            detail("("),
            count(format!("+{}", self.added), self.insert_marker_color),
            detail(" "),
            count(format!("-{}", self.deleted), self.delete_marker_color),
            detail(")"),
        ]));

        Some(Element::Box(BoxElement::row().children(vec![
            Element::Text(
                TextElement::new("  • ")
                    .fg(self.header_bullet_color)
                    .bold()
                    .wrap(TextWrap::NoWrap),
            ),
            Element::Text(
                TextElement::new(format!("{} ", self.action))
                    .fg(self.header_action_color)
                    .bold()
                    .wrap(TextWrap::NoWrap),
            ),
            Element::Text(
                TextElement::new(format!("{path} "))
                    .fg(self.header_detail_color)
                    .bold()
                    .wrap(TextWrap::Truncate),
            ),
            counts,
        ])))
    }

    fn header_for_width(&self, width: usize) -> Option<String> {
        let path = self.path.as_deref()?;
        let prefix = format!("  • {} ", self.action);
        let counts = format!("(+{} -{})", self.added, self.deleted);
        let header = format!("{prefix}{path} {counts}");
        if visible_len(&header) <= width {
            return Some(fit_visible(&header, width));
        }

        let reserved = visible_len(&prefix) + 1 + visible_len(&counts);
        if reserved >= width {
            return Some(fit_visible(&header, width));
        }

        let path_width = width - reserved;
        let path = truncate_visible(path, path_width);
        Some(fit_visible(&format!("{prefix}{path} {counts}"), width))
    }

    fn styled_header_for_width(&self, width: usize) -> Option<String> {
        let header = self.header_for_width(width)?;
        let counts = format!("(+{} -{})", self.added, self.deleted);
        let Some(counts_start) = header.rfind(&counts) else {
            return Some(Style::new().fg(self.header_color).bold().render(&header));
        };

        let added = format!("+{}", self.added);
        let deleted = format!("-{}", self.deleted);
        let prefix = format!("  • {} ", self.action);
        let added_start = counts_start + 1;
        let added_end = added_start + added.len();
        let deleted_start = added_end + 1;
        let deleted_end = deleted_start + deleted.len();
        let bullet_end = "  •".len();
        let action_start = "  • ".len();
        let action_end = action_start + self.action.len();
        let detail = Style::new().fg(self.header_detail_color).bold();

        if !header.starts_with(&prefix) {
            return Some(detail.render(&header));
        }

        Some(format!(
            "{}{}{}{}{}{}{}{}{}",
            detail.render(&header[..2]),
            Style::new()
                .fg(self.header_bullet_color)
                .bold()
                .render(&header[2..bullet_end]),
            detail.render(&header[bullet_end..action_start]),
            Style::new()
                .fg(self.header_action_color)
                .bold()
                .render(&header[action_start..action_end]),
            detail.render(&header[action_end..added_start]),
            Style::new()
                .fg(self.insert_marker_color)
                .bold()
                .render(&added),
            detail.render(&header[added_end..deleted_start]),
            Style::new()
                .fg(self.delete_marker_color)
                .bold()
                .render(&deleted),
            detail.render(&header[deleted_end..]),
        ))
    }

    fn plain_line(&self, line: &DiffLine) -> String {
        format!(
            "    {} {} {}",
            self.line_number(line, self.number_width()),
            line.kind.marker(),
            line.content
        )
    }

    fn line_number(&self, line: &DiffLine, width: usize) -> String {
        let number = line
            .new_number
            .or(line.old_number)
            .map(|number| number.to_string())
            .unwrap_or_default();
        format!("{number:>width$}")
    }

    fn number_width(&self) -> usize {
        self.lines
            .iter()
            .flat_map(|line| [line.old_number, line.new_number])
            .flatten()
            .max()
            .unwrap_or(1)
            .to_string()
            .len()
            .max(3)
    }

    fn rendered_line_count(&self, code_width: usize) -> usize {
        self.lines
            .iter()
            .map(|line| {
                if self.wrap {
                    wrap_hard(&line.content, code_width).len().max(1)
                } else {
                    1
                }
            })
            .sum::<usize>()
            .min(self.max_lines)
    }
}

impl Default for DiffView {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

fn trim_newline(value: &str) -> String {
    let mut out = value;
    if let Some(stripped) = out.strip_suffix('\n') {
        out = stripped;
    }
    if let Some(stripped) = out.strip_suffix('\r') {
        out = stripped;
    }
    out.to_string()
}

fn parse_hunk_starts(header: &str) -> Option<(usize, usize)> {
    let mut old_start = None;
    let mut new_start = None;

    for token in header.split_whitespace() {
        if old_start.is_none() && token.starts_with('-') {
            old_start = parse_range_start(token, '-');
        } else if new_start.is_none() && token.starts_with('+') {
            new_start = parse_range_start(token, '+');
        }

        if old_start.is_some() && new_start.is_some() {
            break;
        }
    }

    Some((old_start?, new_start?))
}

fn parse_range_start(token: &str, prefix: char) -> Option<usize> {
    token.strip_prefix(prefix)?.split(',').next()?.parse().ok()
}

fn style_piece(content: &str, foreground: Option<Color>, background: Option<Color>) -> String {
    if content.is_empty() {
        return String::new();
    }
    let mut style = Style::new();
    if let Some(color) = foreground {
        style = style.fg(color);
    }
    if let Some(color) = background {
        style = style.bg(color);
    }
    style.render(content)
}

fn wrap_diff_spans(spans: &[DiffSpan], width: usize) -> Vec<Vec<DiffSpan>> {
    if width == 0 {
        return vec![spans.to_vec()];
    }

    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut used = 0usize;

    for span in spans {
        let mut index = 0usize;
        while let Some((end, cell_width)) = next_display_cell_boundary(&span.content, index) {
            let cell = &span.content[index..end];
            index = end;

            if cell_width > 0 && used > 0 && used + cell_width > width {
                rows.push(std::mem::take(&mut row));
                used = 0;
            }

            push_diff_span(&mut row, cell, span.color);
            used += cell_width;
            if cell_width > 0 && used >= width {
                rows.push(std::mem::take(&mut row));
                used = 0;
            }
        }
    }

    if !row.is_empty() || rows.is_empty() {
        rows.push(row);
    }
    rows
}

fn truncate_diff_spans(spans: &[DiffSpan], width: usize) -> Vec<DiffSpan> {
    let total = spans
        .iter()
        .map(|span| visible_len(&span.content))
        .sum::<usize>();
    if total <= width {
        return spans.to_vec();
    }
    if width == 0 {
        return Vec::new();
    }
    if width == 1 {
        return vec![DiffSpan::new("…")];
    }

    let target = width - 1;
    let mut out = Vec::new();
    let mut used = 0usize;
    for span in spans {
        let mut index = 0usize;
        while let Some((end, cell_width)) = next_display_cell_boundary(&span.content, index) {
            if cell_width > 0 && used + cell_width > target {
                push_diff_span(&mut out, "…", span.color);
                return out;
            }
            push_diff_span(&mut out, &span.content[index..end], span.color);
            used += cell_width;
            index = end;
        }
    }
    push_diff_span(&mut out, "…", None);
    out
}

fn push_diff_span(spans: &mut Vec<DiffSpan>, content: &str, color: Option<Color>) {
    if let Some(last) = spans.last_mut().filter(|span| span.color == color) {
        last.content.push_str(content);
    } else {
        spans.push(DiffSpan {
            content: content.to_string(),
            color,
        });
    }
}

fn wrap_hard(value: &str, width: usize) -> Vec<String> {
    if width == 0 || value.is_empty() {
        return vec![value.to_string()];
    }

    let mut out = Vec::new();
    let mut line = String::new();
    let mut used = 0usize;
    let mut index = 0usize;
    while let Some((end, cw)) = next_display_cell_boundary(value, index) {
        let cell = &value[index..end];
        index = end;
        if cw == 0 {
            line.push_str(cell);
            continue;
        }

        if used > 0 && used + cw > width {
            out.push(std::mem::take(&mut line));
            used = 0;
        }
        line.push_str(cell);
        used += cw;
        if used >= width {
            out.push(std::mem::take(&mut line));
            used = 0;
        }
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, visible_len};

    #[test]
    fn computes_and_renders_grouped_text_diff() {
        let diff = DiffView::from_texts(
            "src/main.rs",
            "fn main() {\n    old();\n}\n",
            "fn main() {\n    new();\n}\n",
        );
        let rendered = diff.view(56, 8);
        let plain = strip_ansi(&rendered);

        assert_eq!(diff.added(), 1);
        assert_eq!(diff.deleted(), 1);
        assert!(plain.contains("Edited src/main.rs (+1 -1)"));
        assert!(plain.contains("-     old();"));
        assert!(plain.contains("+     new();"));
        assert!(rendered.contains("\x1b["));
        for line in rendered.lines() {
            assert_eq!(visible_len(line), 56, "{line:?}");
        }
    }

    #[test]
    fn header_uses_conventional_addition_and_deletion_colors() {
        let insert = Color::Rgb(46, 160, 67);
        let delete = Color::Rgb(248, 81, 73);
        let rendered = DiffView::from_texts("src/main.rs", "old\n", "new\n")
            .header_color(Color::Blue)
            .insert_color(insert)
            .delete_color(delete)
            .view(48, 3);
        let header = rendered.lines().next().expect("diff header");

        assert!(
            header.contains(&Style::new().fg(insert).bold().render("+1")),
            "addition count should be green: {header:?}"
        );
        assert!(
            header.contains(&Style::new().fg(delete).bold().render("-1")),
            "deletion count should be red: {header:?}"
        );
        assert_eq!(
            strip_ansi(header).trim_end(),
            "  • Edited src/main.rs (+1 -1)"
        );
    }

    #[test]
    fn header_action_is_applied_before_width_fitting() {
        for action in ["Added", "Edited", "Deleted"] {
            let rendered = DiffView::from_texts(
                "src/a-very-long-file-name-that-needs-truncation.rs",
                "old\n",
                "new\n",
            )
            .action(action)
            .view(36, 3);
            let header = rendered.lines().next().expect("diff header");

            assert_eq!(visible_len(header), 36, "{action}: {header:?}");
            assert!(strip_ansi(header).contains(action), "{action}: {header:?}");
            assert!(
                strip_ansi(header).contains("(+1 -1)"),
                "{action}: {header:?}"
            );
        }
    }

    #[test]
    fn header_keeps_counts_when_path_is_truncated() {
        let diff = DiffView::from_texts(
            "src/tui/a/very/long/path/that/should/not/drop-counts.rs",
            "old();\n",
            "new();\n",
        );
        let rendered = diff.view(40, 4);
        let plain = strip_ansi(&rendered);
        let header = plain.lines().next().expect("diff header");

        assert!(header.contains("Edited src/tui"), "{header}");
        assert!(header.contains("(+1 -1)"), "{header}");
        assert_eq!(visible_len(header), 40, "{header}");
    }

    #[test]
    fn parses_unified_lines() {
        let diff = DiffView::from_unified_lines(vec![
            "diff --git a/a b/a",
            "@@ -1 +1 @@",
            "-old",
            "+new",
            " context",
        ]);

        assert_eq!(diff.lines()[0].kind(), DiffLineKind::Metadata);
        assert_eq!(diff.lines()[1].kind(), DiffLineKind::Hunk);
        assert_eq!(diff.lines()[2].kind(), DiffLineKind::Delete);
        assert_eq!(diff.lines()[3].kind(), DiffLineKind::Insert);
        assert_eq!(diff.added(), 1);
        assert_eq!(diff.deleted(), 1);
    }

    #[test]
    fn parses_unified_line_numbers_from_hunks() {
        let diff = DiffView::from_unified_lines(vec![
            "@@ -10,3 +20,4 @@",
            " context",
            "-old",
            "+new",
            " trailing",
        ]);

        assert_eq!(diff.lines()[1].old_number(), Some(10));
        assert_eq!(diff.lines()[1].new_number(), Some(20));
        assert_eq!(diff.lines()[2].old_number(), Some(11));
        assert_eq!(diff.lines()[2].new_number(), None);
        assert_eq!(diff.lines()[3].old_number(), None);
        assert_eq!(diff.lines()[3].new_number(), Some(21));
        assert_eq!(diff.lines()[4].old_number(), Some(12));
        assert_eq!(diff.lines()[4].new_number(), Some(22));
    }

    #[test]
    fn wraps_cjk_to_visible_width() {
        let diff = DiffView::new(vec![DiffLine::new(
            DiffLineKind::Insert,
            "中文测试内容 with a long suffix",
        )])
        .changed_backgrounds(None, None);
        let rendered = diff.view(24, 4);

        assert!(rendered.lines().count() > 1);
        for line in rendered.lines() {
            assert_eq!(visible_len(line), 24, "{line:?}");
        }
    }

    #[test]
    fn styled_content_keeps_gutter_marker_token_and_background_layers() {
        let gutter = Color::Rgb(122, 139, 131);
        let marker = Color::Rgb(0, 194, 0);
        let keyword = Color::Rgb(210, 164, 253);
        let content = Color::Rgb(203, 214, 247);
        let background = Color::Rgb(24, 59, 42);
        let rendered = DiffView::new(vec![
            DiffLine::new(DiffLineKind::Insert, "let value").numbers(None, Some(7))
        ])
        .gutter_colors(Color::BrightBlack, gutter, Color::BrightBlack)
        .marker_colors(marker, Color::Red)
        .changed_content_colors(content, Color::Red)
        .changed_backgrounds(Some(background), None)
        .highlight_content(|_, _| {
            vec![
                DiffSpan::new("let").color(keyword),
                DiffSpan::new(" value").color(content),
            ]
        })
        .view(32, 1);

        assert!(
            rendered.contains(&Style::new().fg(gutter).bg(background).render("  7")),
            "line number should use the insert gutter color: {rendered:?}"
        );
        assert!(
            rendered.contains(&Style::new().fg(marker).bg(background).render("+")),
            "marker should use its own color: {rendered:?}"
        );
        assert!(
            rendered.contains(&Style::new().fg(keyword).bg(background).render("let")),
            "syntax token should retain its color: {rendered:?}"
        );
        assert!(
            rendered.contains(&Style::new().bg(background).render("    ")),
            "the background should cover the leading row padding: {rendered:?}"
        );
        assert_eq!(visible_len(&rendered), 32);
    }

    #[test]
    fn highlighted_cjk_spans_wrap_without_losing_text() {
        let content = "中文测试abcdef";
        let rendered = DiffView::new(vec![DiffLine::new(DiffLineKind::Context, content)])
            .highlight_content(|_, _| {
                vec![
                    DiffSpan::new("中文").color(Color::Cyan),
                    DiffSpan::new("测试").color(Color::Yellow),
                    DiffSpan::new("abcdef").color(Color::Blue),
                ]
            })
            .view(18, 8);
        let plain = strip_ansi(&rendered);
        let rebuilt = plain
            .lines()
            .map(|line| line.chars().skip(10).collect::<String>())
            .map(|line| line.trim_end().to_string())
            .collect::<String>();

        assert_eq!(rebuilt, content);
        assert!(rendered.lines().all(|line| visible_len(line) == 18));
    }

    #[test]
    fn invalid_highlight_spans_fall_back_to_authoritative_content() {
        let invalid = Color::Rgb(1, 2, 3);
        let fallback = Color::Rgb(4, 5, 6);
        let rendered = DiffView::new(vec![DiffLine::new(DiffLineKind::Insert, "actual")])
            .insert_color(fallback)
            .changed_backgrounds(None, None)
            .highlight_content(|_, _| vec![DiffSpan::new("different").color(invalid)])
            .view(24, 1);

        assert_eq!(strip_ansi(&rendered).trim_end(), "        + actual");
        assert!(rendered.contains(&fallback.fg_ansi()));
        assert!(!rendered.contains(&invalid.fg_ansi()));
    }

    #[test]
    fn wrap_hard_keeps_zero_width_marks_with_base_glyph() {
        let lines = wrap_hard("e\u{301}e\u{301}e", 1);

        assert!(lines.iter().all(|line| visible_len(line) <= 1));
        assert_eq!(lines, vec!["e\u{301}", "e\u{301}", "e"]);
    }

    #[test]
    fn wrap_hard_packs_zero_width_marks_by_display_width() {
        let lines = wrap_hard("e\u{301}e\u{301}e", 2);

        assert!(lines.iter().all(|line| visible_len(line) <= 2));
        assert_eq!(lines, vec!["e\u{301}e\u{301}", "e"]);
    }

    #[test]
    fn max_lines_adds_truncation_notice() {
        let diff = DiffView::from_texts("x", "", "a\nb\nc\nd\n").max_lines(2);
        let plain = strip_ansi(&diff.view(40, 6));

        assert!(plain.contains("diff truncated"));
    }

    #[test]
    fn oversized_limits_are_clamped() {
        let diff =
            DiffView::from_texts_with_context("x", "", "a\nb\n", usize::MAX).max_lines(usize::MAX);
        let rendered = diff.view(24, 4);

        assert_eq!(diff.context_lines, MAX_DIFF_VIEW_CONTEXT_LINES);
        assert_eq!(diff.max_lines, MAX_DIFF_VIEW_LINES);
        assert!(rendered.lines().all(|line| visible_len(line) == 24));
    }

    #[test]
    fn element_produces_column() {
        let el: Element<()> = DiffView::from_texts("x", "a\n", "b\n").element();

        match el {
            Element::Box(column) => {
                assert_eq!(column.style.flex_direction, FlexDirection::Column);
                assert!(!column.children.is_empty());
            }
            _ => panic!("expected Box"),
        }
    }

    #[test]
    fn element_with_height_zero_returns_empty_column() {
        let el: Element<()> = DiffView::from_texts("x", "a\n", "b\n").element_with_height(0);

        let Element::Box(column) = el else {
            panic!("expected Box");
        };
        assert_eq!(column.style.flex_direction, FlexDirection::Column);
        assert!(column.children.is_empty());
    }

    #[test]
    fn element_with_height_limits_rows_after_header() {
        let el: Element<()> = DiffView::new(vec![
            DiffLine::new(DiffLineKind::Context, "one"),
            DiffLine::new(DiffLineKind::Context, "two"),
            DiffLine::new(DiffLineKind::Context, "three"),
        ])
        .path("src/lib.rs")
        .element_with_height(2);

        let Element::Box(column) = el else {
            panic!("expected Box");
        };
        assert_eq!(column.children.len(), 2);
        let Element::Box(header) = &column.children[0] else {
            panic!("expected styled header row");
        };
        assert_eq!(header.style.flex_direction, FlexDirection::Row);
        assert!(header.children.iter().any(|child| child
            .text_content()
            .is_some_and(|text| text.contains("Edited"))));
        assert!(header.children.iter().any(|child| child
            .text_content()
            .is_some_and(|text| text.contains("src/lib.rs"))));
        assert!(column.children[1]
            .text_content()
            .is_some_and(|text| text.contains("one")));
        assert!(!column.children.iter().any(|child| child
            .text_content()
            .is_some_and(|text| text.contains("two"))));
    }

    #[test]
    fn element_with_height_keeps_truncation_notice_when_space_remains() {
        let el: Element<()> = DiffView::new(vec![
            DiffLine::new(DiffLineKind::Insert, "one"),
            DiffLine::new(DiffLineKind::Insert, "two"),
            DiffLine::new(DiffLineKind::Insert, "three"),
        ])
        .max_lines(1)
        .element_with_height(3);

        let Element::Box(column) = el else {
            panic!("expected Box");
        };
        assert_eq!(column.children.len(), 2);
        assert!(column.children[0]
            .text_content()
            .is_some_and(|text| text.contains("one")));
        assert!(column.children[1]
            .text_content()
            .is_some_and(|text| text.contains("diff truncated")));
    }

    #[test]
    fn with_theme_applies_semantic_colors() {
        let theme = Theme::tokyo_night();
        let diff = DiffView::default().with_theme(&theme);

        assert_eq!(diff.header_color, theme.color(ThemeRole::Primary));
        assert_eq!(diff.meta_color, theme.color(ThemeRole::Muted));
        assert_eq!(diff.hunk_color, theme.color(ThemeRole::Info));
        assert_eq!(diff.context_color, theme.color(ThemeRole::Foreground));
        assert_eq!(diff.insert_fg, theme.color(ThemeRole::Success));
        assert_eq!(diff.insert_bg, Some(theme.color(ThemeRole::Surface)));
        assert_eq!(diff.delete_fg, theme.color(ThemeRole::Error));
        assert_eq!(diff.delete_bg, Some(theme.color(ThemeRole::Surface)));
        assert_eq!(diff.separator_color, theme.color(ThemeRole::Border));
    }
}
