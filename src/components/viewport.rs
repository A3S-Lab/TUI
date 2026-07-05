use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::event::{MouseEvent, MouseEventKind};
use crate::style::{
    next_display_cell_boundary, slice_visible_cols, strip_ansi, truncate_visible, visible_len,
    Style,
};

pub struct Viewport {
    content: String,
    lines: Vec<String>,
    offset: usize,
    width: u16,
    height: u16,
    auto_scroll: bool,
}

#[derive(Debug, Clone)]
pub enum ViewportMsg {
    ScrollUp(usize),
    ScrollDown(usize),
    PageUp,
    PageDown,
    Top,
    Bottom,
}

/// In-progress text selection in screen cells.
///
/// Coordinates are `(row, column)` within the rendered viewport. `anchor` is the
/// drag start and `head` is the current cursor position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSelection {
    anchor: (u16, u16),
    head: (u16, u16),
}

impl TextSelection {
    pub fn new(anchor: (u16, u16), head: (u16, u16)) -> Self {
        Self { anchor, head }
    }

    pub fn from_cells(anchor_row: u16, anchor_col: u16, head_row: u16, head_col: u16) -> Self {
        Self::new((anchor_row, anchor_col), (head_row, head_col))
    }

    pub fn anchor(&self) -> (u16, u16) {
        self.anchor
    }

    pub fn head(&self) -> (u16, u16) {
        self.head
    }

    pub fn set_head(&mut self, row: u16, col: u16) {
        self.head = (row, col);
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    pub fn ordered(&self) -> SelectionRange {
        SelectionRange::from_cells(
            self.anchor.0 as usize,
            self.anchor.1 as usize,
            self.head.0 as usize,
            self.head.1 as usize,
        )
    }
}

/// Ordered selection range over rendered viewport rows.
///
/// Rows are inclusive. Columns use the half-open range `[start_col, end_col)` on
/// the first/last selected rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionRange {
    pub start_row: usize,
    pub start_col: usize,
    pub end_row: usize,
    pub end_col: usize,
}

impl SelectionRange {
    pub fn from_cells(row_a: usize, col_a: usize, row_b: usize, col_b: usize) -> Self {
        if (row_a, col_a) <= (row_b, col_b) {
            Self {
                start_row: row_a,
                start_col: col_a,
                end_row: row_b,
                end_col: col_b,
            }
        } else {
            Self {
                start_row: row_b,
                start_col: col_b,
                end_row: row_a,
                end_col: col_a,
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.start_row == self.end_row && self.start_col == self.end_col
    }
}

impl Viewport {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            content: String::new(),
            lines: Vec::new(),
            offset: 0,
            width,
            height,
            auto_scroll: true,
        }
    }

    pub fn with_auto_scroll(mut self, auto: bool) -> Self {
        self.auto_scroll = auto;
        self
    }

    /// Pause/resume auto-follow (e.g. when the user scrolls up mid-stream).
    pub fn set_auto_scroll(&mut self, auto: bool) {
        self.auto_scroll = auto;
    }

    pub fn set_content(&mut self, content: &str) {
        self.content.clear();
        self.content.push_str(content);
        self.rewrap_content();
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    pub fn append(&mut self, content: &str) {
        self.content.push_str(content);
        self.rewrap_content();
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.lines.clear();
        self.offset = 0;
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        self.rewrap_content();
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    pub fn update(&mut self, msg: ViewportMsg) {
        match msg {
            ViewportMsg::ScrollUp(n) => {
                self.offset = self.offset.saturating_sub(n);
            }
            ViewportMsg::ScrollDown(n) => {
                self.offset = self.offset.saturating_add(n).min(self.max_offset());
            }
            ViewportMsg::PageUp => {
                self.offset = self.offset.saturating_sub(self.height as usize);
            }
            ViewportMsg::PageDown => {
                self.offset = self
                    .offset
                    .saturating_add(self.height as usize)
                    .min(self.max_offset());
            }
            ViewportMsg::Top => {
                self.offset = 0;
            }
            ViewportMsg::Bottom => {
                self.scroll_to_bottom();
            }
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        self.offset = self.max_offset();
    }

    pub fn total_lines(&self) -> usize {
        self.lines.len()
    }

    pub fn scroll_percent(&self) -> u8 {
        if self.lines.len() <= self.height as usize {
            return 100;
        }
        let max = self.max_offset();
        if max == 0 {
            return 100;
        }
        let offset = self.offset.min(max);
        ((offset as u128 * 100) / max as u128) as u8
    }

    pub fn at_bottom(&self) -> bool {
        self.offset >= self.max_offset()
    }

    /// Handle mouse scroll events.
    pub fn handle_mouse(&mut self, mouse: &MouseEvent) {
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.update(ViewportMsg::ScrollUp(3));
            }
            MouseEventKind::ScrollDown => {
                self.update(ViewportMsg::ScrollDown(3));
            }
            _ => {}
        }
    }

    pub fn view(&self) -> String {
        let h = self.height as usize;
        let (offset, end) = self.visible_range();
        let mut visible: Vec<&str> = self.lines[offset..end].iter().map(|s| s.as_str()).collect();

        while visible.len() < h {
            visible.push("");
        }

        visible.join("\n")
    }

    /// Return plain text selected from the currently visible rows.
    pub fn selected_text(&self, selection: TextSelection) -> String {
        selected_text(&self.view(), selection)
    }

    /// Return the currently visible rows with a text selection highlighted.
    pub fn highlighted_view(&self, selection: TextSelection, style: &Style) -> String {
        highlight_selection(&self.view(), selection, style)
    }

    /// Render visible lines as an Element tree.
    pub fn element<Msg>(&self) -> Element<Msg> {
        let h = self.height as usize;
        let (offset, end) = self.visible_range();

        let mut children: Vec<Element<Msg>> = self.lines[offset..end]
            .iter()
            .map(|line| Element::Text(TextElement::new(line.as_str())))
            .collect();

        let visible_count = end - offset;
        for _ in visible_count..h {
            children.push(Element::Text(TextElement::new("")));
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(children),
        )
    }

    fn max_offset(&self) -> usize {
        self.lines.len().saturating_sub(self.height as usize)
    }

    fn visible_range(&self) -> (usize, usize) {
        let h = self.height as usize;
        // Clamp the offset: after a resize re-wraps to fewer lines, a stale
        // offset must not slice past the end and blank or panic in either
        // string or Element rendering.
        let max_off = self.lines.len().saturating_sub(h);
        let offset = self.offset.min(max_off);
        let end = offset.saturating_add(h).min(self.lines.len());
        (offset, end)
    }

    fn rewrap_content(&mut self) {
        self.lines = wrap_content(&self.content, self.width as usize);
    }
}

fn wrap_content(content: &str, width: usize) -> Vec<String> {
    let mut result = Vec::new();

    for line in content.lines() {
        if width == 0 {
            result.push(line.to_string());
            continue;
        }
        let vis = visible_len(line);
        if vis <= width {
            result.push(line.to_string());
        } else {
            let wrapped = wrap_line(line, width);
            result.extend(wrapped);
        }
    }

    if content.ends_with('\n') && !content.is_empty() {
        result.push(String::new());
    }

    result
}

/// Return plain text selected from a rendered viewport view.
///
/// Selected rows are ANSI-stripped and trailing padding is trimmed, making the
/// result suitable for clipboard copy.
pub fn selected_text(view: &str, selection: TextSelection) -> String {
    selected_text_range(view, selection.ordered())
}

pub fn selected_text_range(view: &str, range: SelectionRange) -> String {
    if range.is_empty() {
        return String::new();
    }

    let rows: Vec<&str> = view.split('\n').collect();
    let mut out = Vec::new();

    for row_idx in range.start_row..=range.end_row {
        let Some(row) = rows.get(row_idx) else {
            break;
        };
        let plain = strip_ansi(row);
        let from = if row_idx == range.start_row {
            range.start_col
        } else {
            0
        };
        let to = if row_idx == range.end_row {
            range.end_col
        } else {
            usize::MAX
        };
        out.push(slice_visible_cols(&plain, from, to).trim_end().to_string());
    }

    out.join("\n")
}

/// Highlight a selection in a rendered viewport view.
///
/// Unselected rows are left unchanged. Selected rows are ANSI-stripped before
/// highlighting so applications can transiently overlay selection color without
/// leaking syntax or log styling through the selected span.
pub fn highlight_selection(view: &str, selection: TextSelection, style: &Style) -> String {
    highlight_selection_range(view, selection.ordered(), style)
}

pub fn highlight_selection_range(view: &str, range: SelectionRange, style: &Style) -> String {
    if range.is_empty() {
        return view.to_string();
    }

    view.split('\n')
        .enumerate()
        .map(|(row_idx, row)| {
            if row_idx < range.start_row || row_idx > range.end_row {
                return row.to_string();
            }

            let plain = strip_ansi(row);
            let from = if row_idx == range.start_row {
                range.start_col
            } else {
                0
            };
            let to = if row_idx == range.end_row {
                range.end_col
            } else {
                usize::MAX
            };

            let before = slice_visible_cols(&plain, 0, from);
            let selected = slice_visible_cols(&plain, from, to);
            let after = slice_visible_cols(&plain, to, usize::MAX);

            if selected.is_empty() {
                plain
            } else {
                format!("{before}{}{after}", style.render(&selected))
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn wrap_line(s: &str, width: usize) -> Vec<String> {
    // The gutter prefixes content with leading spaces (the left margin); keep
    // that indent on wrapped continuation lines so they don't fall back to the
    // screen edge.
    let indent = s
        .chars()
        .take_while(|c| *c == ' ')
        .count()
        .min(width.saturating_sub(8));
    let pad = " ".repeat(indent);
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0;
    let mut in_escape = false;
    let mut index = 0usize;

    while index < s.len() {
        let c = s[index..].chars().next().unwrap_or_default();
        if c == '\x1b' {
            in_escape = true;
            current.push(c);
            index += c.len_utf8();
            continue;
        }
        if in_escape {
            current.push(c);
            index += c.len_utf8();
            if c.is_ascii_alphabetic() {
                in_escape = false;
            }
            continue;
        }

        let Some((end, cw)) = next_display_cell_boundary(s, index) else {
            break;
        };
        let cell = &s[index..end];
        index = end;

        if cw > width {
            if current_width > 0 {
                lines.push(current);
                current = pad.clone();
                current_width = indent;
            }
            let clipped = truncate_visible(cell, width.saturating_sub(current_width));
            if !clipped.is_empty() {
                current.push_str(&clipped);
                current_width += visible_len(&clipped);
            }
            continue;
        }
        if current_width + cw > width && current_width > 0 {
            lines.push(current);
            current = pad.clone();
            current_width = indent;
        }
        current.push_str(cell);
        current_width += cw;
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_content_and_view() {
        let mut vp = Viewport::new(80, 3);
        vp.set_content("line1\nline2\nline3");
        let view = vp.view();
        assert!(view.contains("line1"));
        assert!(view.contains("line3"));
    }

    #[test]
    fn empty_view_has_exact_height_rows() {
        let vp = Viewport::new(80, 3);
        let view = vp.view();

        assert_eq!(view.split('\n').collect::<Vec<_>>(), vec!["", "", ""]);
    }

    #[test]
    fn stale_offset_after_shrink_does_not_blank_or_panic() {
        // Regression: scroll up, then resize to far fewer lines. A stale offset
        // must clamp instead of slicing past the end (which blanked the screen).
        let mut vp = Viewport::new(80, 5).with_auto_scroll(false);
        vp.set_content(
            &(1..=50)
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join("\n"),
        );
        vp.update(ViewportMsg::ScrollDown(40)); // offset deep in the content
        vp.set_content("only\ntwo"); // now far fewer lines
        let view = vp.view();
        assert!(view.contains("only"), "clamped offset still shows content");

        let Element::Box(box_el) = vp.element::<()>() else {
            panic!("expected viewport element box");
        };
        let Element::Text(first) = &box_el.children[0] else {
            panic!("expected first rendered line");
        };
        assert_eq!(first.content, "only");
    }

    #[test]
    fn scroll_percent_clamps_stale_offset_after_shrink() {
        let mut vp = Viewport::new(80, 5).with_auto_scroll(false);
        vp.set_content(
            &(1..=50)
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join("\n"),
        );
        vp.update(ViewportMsg::ScrollDown(40));
        vp.set_content("only\ntwo\nthree\nfour\nfive\nsix");

        assert_eq!(vp.scroll_percent(), 100);
    }

    #[test]
    fn wrapped_line_keeps_leading_indent() {
        // A gutter-indented long line wraps; continuations keep the indent
        // instead of falling back to column 0. (Width must exceed indent+8, or
        // the indent is capped to keep usable wrap width.)
        let mut vp = Viewport::new(40, 5);
        vp.set_content(&format!("    {}", "x".repeat(60)));
        let view = vp.view();
        let wrapped: Vec<&str> = view.lines().filter(|l| !l.trim().is_empty()).collect();
        assert!(wrapped.len() >= 2, "long line wrapped");
        assert!(wrapped[1].starts_with("    "), "continuation keeps indent");
    }

    #[test]
    fn wraps_wide_glyphs_to_one_column_width() {
        let mut vp = Viewport::new(1, 3);
        vp.set_content("中文");

        let view = vp.view();
        let rows: Vec<&str> = view.split('\n').collect();

        assert!(rows.iter().all(|row| visible_len(row) <= 1));
        assert_eq!(rows[0], "…");
        assert_eq!(rows[1], "…");
    }

    #[test]
    fn wrap_line_keeps_zero_width_marks_with_base_glyph() {
        let lines = wrap_line("e\u{301}e\u{301}e", 1);

        assert!(lines.iter().all(|line| visible_len(line) <= 1));
        assert_eq!(lines, vec!["e\u{301}", "e\u{301}", "e"]);
    }

    #[test]
    fn wrap_line_packs_zero_width_marks_by_display_width() {
        let lines = wrap_line("e\u{301}e\u{301}e", 2);

        assert!(lines.iter().all(|line| visible_len(line) <= 2));
        assert_eq!(lines, vec!["e\u{301}e\u{301}", "e"]);
    }

    #[test]
    fn scroll_down() {
        let mut vp = Viewport::new(80, 2).with_auto_scroll(false);
        vp.set_content("a\nb\nc\nd\ne");
        assert_eq!(vp.total_lines(), 5);
        vp.update(ViewportMsg::ScrollDown(2));
        let view = vp.view();
        assert!(view.contains("c"));
    }

    #[test]
    fn huge_scroll_down_saturates_at_bottom() {
        let mut vp = Viewport::new(80, 2).with_auto_scroll(false);
        vp.set_content("a\nb\nc\nd\ne");

        vp.update(ViewportMsg::ScrollDown(usize::MAX));

        assert_eq!(vp.offset, vp.max_offset());
        assert!(vp.at_bottom());
    }

    #[test]
    fn scroll_up() {
        let mut vp = Viewport::new(80, 2).with_auto_scroll(false);
        vp.set_content("a\nb\nc\nd");
        vp.update(ViewportMsg::ScrollDown(3));
        vp.update(ViewportMsg::ScrollUp(1));
        let view = vp.view();
        assert!(view.contains("c"));
    }

    #[test]
    fn auto_scroll_to_bottom() {
        let mut vp = Viewport::new(80, 2);
        vp.set_content("a\nb\nc\nd\ne");
        assert!(vp.at_bottom());
    }

    #[test]
    fn page_up_down() {
        let mut vp = Viewport::new(80, 2).with_auto_scroll(false);
        vp.set_content("1\n2\n3\n4\n5\n6\n7\n8");
        vp.update(ViewportMsg::PageDown);
        vp.update(ViewportMsg::PageDown);
        vp.update(ViewportMsg::PageUp);
        let view = vp.view();
        assert!(view.contains("3"));
    }

    #[test]
    fn top_and_bottom() {
        let mut vp = Viewport::new(80, 2).with_auto_scroll(false);
        vp.set_content("a\nb\nc\nd\ne");
        vp.update(ViewportMsg::Bottom);
        assert!(vp.at_bottom());
        vp.update(ViewportMsg::Top);
        assert_eq!(vp.scroll_percent(), 0);
    }

    #[test]
    fn append_content() {
        let mut vp = Viewport::new(80, 3);
        vp.set_content("first");
        vp.append("\nsecond");
        assert!(vp.total_lines() >= 2);
        let view = vp.view();
        assert!(view.contains("second"));
    }

    #[test]
    fn append_content_without_newline_extends_current_line() {
        let mut vp = Viewport::new(80, 2);
        vp.set_content("first");
        vp.append(" second");

        let view = vp.view();
        let rows: Vec<&str> = view.split('\n').collect();

        assert_eq!(rows[0], "first second");
    }

    #[test]
    fn resize_rewraps_from_raw_content() {
        let mut vp = Viewport::new(5, 3);
        vp.set_content("abcdef");
        assert_eq!(vp.total_lines(), 2);

        vp.resize(20, 3);

        let view = vp.view();
        let rows: Vec<&str> = view.split('\n').collect();
        assert_eq!(rows[0], "abcdef");
        assert_eq!(vp.total_lines(), 1);
    }

    #[test]
    fn clear_resets() {
        let mut vp = Viewport::new(80, 3);
        vp.set_content("hello\nworld");
        vp.clear();
        assert_eq!(vp.total_lines(), 0);
    }

    #[test]
    fn text_selection_orders_anchor_and_head() {
        let selection = TextSelection::from_cells(3, 8, 1, 2);
        assert_eq!(
            selection.ordered(),
            SelectionRange {
                start_row: 1,
                start_col: 2,
                end_row: 3,
                end_col: 8,
            }
        );
        assert!(!selection.is_empty());
    }

    #[test]
    fn selected_text_extracts_span_across_rows() {
        let view = "  hello world\n  second line\n  third";
        let selection = TextSelection::from_cells(0, 2, 1, 8);

        assert_eq!(selected_text(view, selection), "hello world\n  second");
    }

    #[test]
    fn selected_text_accepts_reversed_selection() {
        let view = "alpha beta\nsecond row\nthird row";
        let selection = TextSelection::from_cells(1, 6, 0, 6);

        assert_eq!(selected_text(view, selection), "beta\nsecond");
    }

    #[test]
    fn selected_text_strips_ansi_and_uses_display_columns() {
        let styled = Style::new().fg(crate::style::Color::Red).render("ab你好cd");
        let view = format!("{styled}\nplain");
        let selection = TextSelection::from_cells(0, 2, 0, 6);

        assert_eq!(selected_text(&view, selection), "你好");
    }

    #[test]
    fn selected_text_keeps_zero_width_marks_with_base_glyph() {
        let view = "e\u{301}x";
        let selection = TextSelection::from_cells(0, 0, 0, 1);

        assert_eq!(selected_text(view, selection), "e\u{301}");
    }

    #[test]
    fn highlight_selection_keeps_zero_width_marks_with_base_glyph() {
        let view = "e\u{301}x";
        let selection = TextSelection::from_cells(0, 0, 0, 1);
        let style = Style::new().bg(crate::style::Color::Blue);

        let out = highlight_selection(view, selection, &style);

        assert_eq!(strip_ansi(&out), "e\u{301}x");
        assert!(out.contains("\x1b[44me\u{301}\x1b[0mx"));
    }

    #[test]
    fn empty_selection_copies_nothing_and_keeps_view_unchanged() {
        let view = "alpha\nbeta";
        let selection = TextSelection::from_cells(0, 2, 0, 2);
        let style = Style::new().bg(crate::style::Color::Blue);

        assert_eq!(selected_text(view, selection), "");
        assert_eq!(highlight_selection(view, selection, &style), view);
    }

    #[test]
    fn highlight_selection_touches_only_selected_rows() {
        let view = "row zero\nrow one\nrow two";
        let selection = TextSelection::from_cells(1, 0, 1, 7);
        let style = Style::new()
            .bg(crate::style::Color::Rgb(58, 64, 88))
            .fg(crate::style::Color::White);

        let out = highlight_selection(view, selection, &style);
        let lines: Vec<&str> = out.split('\n').collect();
        assert_eq!(lines[0], "row zero");
        assert_eq!(lines[2], "row two");
        assert!(lines[1].contains("row one"));
        assert!(lines[1].contains('\u{1b}'));
    }

    #[test]
    fn highlight_selection_strips_ansi_on_selected_rows_only() {
        let selected = Style::new()
            .fg(crate::style::Color::Red)
            .render("selected row");
        let unselected = Style::new()
            .fg(crate::style::Color::Green)
            .render("unselected row");
        let view = format!("{selected}\n{unselected}");
        let selection = TextSelection::from_cells(0, 0, 0, 8);
        let style = Style::new().bg(crate::style::Color::Blue);

        let out = highlight_selection(&view, selection, &style);
        let lines: Vec<&str> = out.split('\n').collect();
        assert!(lines[0].contains("selected"));
        assert!(!lines[0].contains("\x1b[31m"));
        assert!(lines[0].contains("\x1b[44m"));
        assert!(lines[1].contains("\x1b[32m"));
    }

    #[test]
    fn viewport_selection_helpers_use_current_view() {
        let mut vp = Viewport::new(80, 2).with_auto_scroll(false);
        vp.set_content("first\nsecond row\nthird");
        vp.update(ViewportMsg::ScrollDown(1));
        let selection = TextSelection::from_cells(0, 0, 0, 6);

        assert_eq!(vp.selected_text(selection), "second");
        assert!(vp
            .highlighted_view(selection, &Style::new().bg(crate::style::Color::Blue))
            .contains("\x1b[44msecond\x1b[0m"));
    }
}
