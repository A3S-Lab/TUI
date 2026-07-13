use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::event::{MouseEvent, MouseEventKind};
use crate::style::{
    ansi_escape_sequence_end, next_display_cell_boundary, slice_visible_cols, strip_ansi,
    truncate_visible, visible_len, Style,
};

pub struct Viewport {
    content: String,
    lines: Vec<String>,
    partition: Option<ContentPartition>,
    offset: usize,
    width: u16,
    height: u16,
    auto_scroll: bool,
}

struct ContentPartition {
    prefix: String,
    suffix: String,
    prefix_line_count: usize,
    suffix_was_empty: bool,
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
            partition: None,
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
        self.partition = None;
        self.content.clear();
        self.content.push_str(content);
        self.rewrap_content();
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    pub fn append(&mut self, content: &str) {
        self.materialize_partition();
        self.content.push_str(content);
        self.rewrap_content();
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    /// Replace a mutable suffix while retaining the already-wrapped prefix.
    ///
    /// Streaming transcript callers use this to update only the active
    /// Markdown tail. When a newline-terminated `prefix` is unchanged, its
    /// wrapped rows stay in place and only `suffix` is wrapped again. Both raw
    /// parts are retained so a terminal resize can still perform a
    /// source-backed full reflow.
    pub fn set_content_parts(&mut self, prefix: &str, suffix: &str) {
        // A non-newline boundary can fall in the middle of a wrapped row or an
        // ANSI-styled span. Wrapping the parts independently would then differ
        // from wrapping their concatenation, so keep the combined source and
        // perform a full reflow. Newline-terminated prefixes use the safe
        // incremental path below.
        if !prefix.ends_with('\n') {
            self.partition = None;
            self.content.clear();
            self.content.push_str(prefix);
            self.content.push_str(suffix);
            self.rewrap_content();
            if self.auto_scroll {
                self.scroll_to_bottom();
            }
            return;
        }

        let suffix_is_empty = suffix.is_empty();
        let reuse_prefix = self.partition.as_ref().is_some_and(|partition| {
            partition.prefix == prefix && partition.suffix_was_empty == suffix_is_empty
        });
        let append_to_prefix = self.partition.as_ref().and_then(|partition| {
            (partition.suffix_was_empty == suffix_is_empty
                && !partition.prefix.is_empty()
                && partition.prefix.ends_with('\n')
                && prefix.len() > partition.prefix.len()
                && prefix.starts_with(&partition.prefix))
            .then_some((
                partition.prefix.len(),
                partition.prefix_line_count,
                partition.suffix_was_empty,
            ))
        });

        if reuse_prefix {
            let prefix_line_count = self
                .partition
                .as_ref()
                .map_or(0, |partition| partition.prefix_line_count);
            self.lines.truncate(prefix_line_count);
            self.lines.extend(wrap_content(suffix, self.width as usize));
            if let Some(partition) = &mut self.partition {
                partition.suffix.clear();
                partition.suffix.push_str(suffix);
            }
        } else if let Some((old_prefix_len, old_prefix_line_count, old_suffix_was_empty)) =
            append_to_prefix
        {
            // Stable streaming rows only append to a newline-terminated
            // prefix. Retain its wrapped rows, remove the synthetic trailing
            // blank row when the old suffix was empty, and wrap only the new
            // prefix fragment plus the mutable suffix.
            let mut retained = old_prefix_line_count;
            if old_suffix_was_empty
                && retained > 0
                && self.lines.get(retained - 1).is_some_and(String::is_empty)
            {
                retained -= 1;
            }
            self.lines.truncate(retained);
            let appended = &prefix[old_prefix_len..];
            let mut appended_lines = wrap_content(appended, self.width as usize);
            trim_partition_boundary(&mut appended_lines, appended, suffix);
            self.lines.extend(appended_lines);
            let prefix_line_count = self.lines.len();
            self.lines.extend(wrap_content(suffix, self.width as usize));
            if let Some(partition) = &mut self.partition {
                partition.prefix.clear();
                partition.prefix.push_str(prefix);
                partition.suffix.clear();
                partition.suffix.push_str(suffix);
                partition.prefix_line_count = prefix_line_count;
                partition.suffix_was_empty = suffix_is_empty;
            }
        } else {
            let mut prefix_lines = wrap_content(prefix, self.width as usize);
            trim_partition_boundary(&mut prefix_lines, prefix, suffix);
            let prefix_line_count = prefix_lines.len();
            prefix_lines.extend(wrap_content(suffix, self.width as usize));
            self.lines = prefix_lines;
            self.partition = Some(ContentPartition {
                prefix: prefix.to_string(),
                suffix: suffix.to_string(),
                prefix_line_count,
                suffix_was_empty: suffix_is_empty,
            });
            self.content.clear();
        }

        self.clamp_offset();
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.lines.clear();
        self.partition = None;
        self.offset = 0;
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        if let Some(partition) = &mut self.partition {
            let mut prefix_lines = wrap_content(&partition.prefix, width as usize);
            trim_partition_boundary(&mut prefix_lines, &partition.prefix, &partition.suffix);
            partition.prefix_line_count = prefix_lines.len();
            partition.suffix_was_empty = partition.suffix.is_empty();
            prefix_lines.extend(wrap_content(&partition.suffix, width as usize));
            self.lines = prefix_lines;
        } else {
            self.rewrap_content();
        }
        self.clamp_offset();
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

    /// Current top row in the fully wrapped content.
    pub fn scroll_offset(&self) -> usize {
        self.offset.min(self.max_offset())
    }

    /// Restore a previously captured top row, clamped to current content.
    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.offset = offset.min(self.max_offset());
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
        self.clamp_offset();
    }

    fn clamp_offset(&mut self) {
        self.offset = self.offset.min(self.max_offset());
    }

    fn materialize_partition(&mut self) {
        let Some(partition) = self.partition.take() else {
            return;
        };
        self.content.clear();
        self.content.push_str(&partition.prefix);
        self.content.push_str(&partition.suffix);
    }
}

fn trim_partition_boundary(lines: &mut Vec<String>, prefix: &str, suffix: &str) {
    if !suffix.is_empty() && prefix.ends_with('\n') && lines.last().is_some_and(String::is_empty) {
        lines.pop();
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
    // screen edge. When the row uses a symbolic gutter marker (`  ● text`),
    // continue under the text column rather than under the marker.
    let indent = continuation_indent(s, width);
    let pad = " ".repeat(indent);
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0;
    let mut index = 0usize;

    while index < s.len() {
        if let Some(end) = ansi_escape_sequence_end(s, index) {
            current.push_str(&s[index..end]);
            index = end;
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

fn continuation_indent(s: &str, width: usize) -> usize {
    // Always leave at least one display column for content.  The previous
    // eight-column reserve dropped the indent entirely in very narrow
    // viewports (for example an 8-column transcript row), so gutter-wrapped
    // continuation lines jumped back to column zero.
    let max_indent = width.saturating_sub(1);
    let plain = strip_ansi(s);
    let leading = plain.chars().take_while(|c| *c == ' ').count();

    let rest = &plain[leading..];
    let mut token_end = 0usize;
    for (index, ch) in rest.char_indices() {
        if ch.is_whitespace() {
            break;
        }
        token_end = index + ch.len_utf8();
    }
    if token_end == 0 {
        return leading.min(max_indent);
    }

    let token = &rest[..token_end];
    if token.chars().any(char::is_alphanumeric) || visible_len(token) > 4 {
        return leading.min(max_indent);
    }

    let gap_width = rest[token_end..]
        .chars()
        .take_while(|ch| ch.is_whitespace())
        .map(|ch| visible_len(&ch.to_string()))
        .sum::<usize>();
    if gap_width == 0 {
        return leading.min(max_indent);
    }

    leading
        .saturating_add(visible_len(token))
        .saturating_add(gap_width)
        .min(max_indent)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_content_parts_match_full(width: u16, prefix: &str, suffix: &str) {
        let mut partitioned = Viewport::new(width, 20).with_auto_scroll(false);
        partitioned.set_content_parts(prefix, suffix);

        let mut full = Viewport::new(width, 20).with_auto_scroll(false);
        full.set_content(&format!("{prefix}{suffix}"));

        assert_eq!(partitioned.lines, full.lines);
    }

    #[test]
    fn set_content_and_view() {
        let mut vp = Viewport::new(80, 3);
        vp.set_content("line1\nline2\nline3");
        let view = vp.view();
        assert!(view.contains("line1"));
        assert!(view.contains("line3"));
    }

    #[test]
    fn partitioned_content_matches_full_content_and_replaces_only_suffix_rows() {
        let prefix = "history one\nhistory two\n\n";
        let mut partitioned = Viewport::new(12, 20).with_auto_scroll(false);
        partitioned.set_content_parts(prefix, "live alpha\nlive beta");

        let mut full = Viewport::new(12, 20).with_auto_scroll(false);
        full.set_content(&format!("{prefix}live alpha\nlive beta"));
        assert_eq!(partitioned.lines, full.lines);

        let prefix_line_count = partitioned
            .partition
            .as_ref()
            .expect("partition should be retained")
            .prefix_line_count;
        let prefix_rows = partitioned.lines[..prefix_line_count].to_vec();
        partitioned.set_content_parts(prefix, "replacement tail that wraps");

        assert_eq!(
            &partitioned.lines[..prefix_line_count],
            prefix_rows.as_slice(),
            "stable prefix rows must remain in place"
        );
        let mut replaced_full = Viewport::new(12, 20).with_auto_scroll(false);
        replaced_full.set_content(&format!("{prefix}replacement tail that wraps"));
        assert_eq!(partitioned.lines, replaced_full.lines);
    }

    #[test]
    fn partitioned_content_joins_plain_text_across_non_newline_boundary() {
        assert_content_parts_match_full(5, "abc", "def");
    }

    #[test]
    fn partitioned_content_joins_ansi_text_across_non_newline_boundary() {
        assert_content_parts_match_full(4, "\x1b[31mab", "cd\x1b[0mef");
    }

    #[test]
    fn partitioned_content_joins_wide_text_across_non_newline_boundary() {
        assert_content_parts_match_full(4, "你", "好ab");
    }

    #[test]
    fn partitioned_content_reflows_both_regions_after_resize() {
        let prefix = "stable history line that wraps\n\n";
        let suffix = "mutable tail line that also wraps";
        let mut partitioned = Viewport::new(24, 20).with_auto_scroll(false);
        partitioned.set_content_parts(prefix, suffix);

        partitioned.resize(10, 20);

        let mut full = Viewport::new(10, 20).with_auto_scroll(false);
        full.set_content(&format!("{prefix}{suffix}"));
        assert_eq!(partitioned.lines, full.lines);
    }

    #[test]
    fn partitioned_content_appends_stable_prefix_before_replacing_tail() {
        let mut partitioned = Viewport::new(12, 20).with_auto_scroll(false);
        partitioned.set_content_parts("history one\n", "live tail");
        partitioned.set_content_parts(
            "history one\nstable answer row that wraps\n",
            "replacement tail",
        );

        let mut full = Viewport::new(12, 20).with_auto_scroll(false);
        full.set_content("history one\nstable answer row that wraps\nreplacement tail");
        assert_eq!(partitioned.lines, full.lines);
        assert_eq!(
            partitioned
                .partition
                .as_ref()
                .expect("partition")
                .prefix_line_count,
            4
        );
    }

    #[test]
    fn append_materializes_partition_without_losing_content() {
        let mut viewport = Viewport::new(80, 10).with_auto_scroll(false);
        viewport.set_content_parts("history\n", "tail");

        viewport.append("\nappended");

        assert!(viewport.partition.is_none());
        assert_eq!(viewport.lines, vec!["history", "tail", "appended"]);
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
    fn content_shrink_clamps_raw_offset_before_followup_scroll() {
        let mut vp = Viewport::new(80, 2).with_auto_scroll(false);
        vp.set_content("zero\none\ntwo\nthree\nfour\nfive\nsix\nseven");
        vp.update(ViewportMsg::ScrollDown(usize::MAX));
        assert_eq!(vp.offset, 6);

        vp.set_content("zero\none\ntwo\nthree");

        assert_eq!(vp.offset, 2, "raw offset should clamp during shrink");
        vp.update(ViewportMsg::ScrollUp(1));
        assert_eq!(vp.offset, 1);
        vp.update(ViewportMsg::ScrollDown(1));
        assert_eq!(vp.offset, 2);
    }

    #[test]
    fn partitioned_content_shrink_clamps_raw_offset_before_followup_scroll() {
        let mut vp = Viewport::new(80, 2).with_auto_scroll(false);
        vp.set_content_parts("zero\none\n", "two\nthree\nfour\nfive\nsix\nseven");
        vp.update(ViewportMsg::ScrollDown(usize::MAX));
        assert_eq!(vp.offset, 6);

        vp.set_content_parts("zero\none\n", "two\nthree");

        assert_eq!(vp.offset, 2, "raw offset should clamp during shrink");
        vp.update(ViewportMsg::ScrollUp(1));
        assert_eq!(vp.offset, 1);
        vp.update(ViewportMsg::ScrollDown(1));
        assert_eq!(vp.offset, 2);
    }

    #[test]
    fn captured_scroll_offset_can_be_restored_after_content_reflow() {
        let mut vp = Viewport::new(12, 3).with_auto_scroll(false);
        vp.set_content("zero\none\ntwo\nthree\nfour\nfive");
        vp.set_scroll_offset(2);
        assert_eq!(vp.scroll_offset(), 2);
        assert!(vp.view().starts_with("two"));

        vp.set_content("prefix\nzero\none\ntwo\nthree\nfour\nfive");
        vp.set_scroll_offset(3);
        assert_eq!(vp.scroll_offset(), 3);
        assert!(vp.view().starts_with("two"));
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
    fn wrapped_gutter_line_aligns_continuation_after_marker() {
        let mut vp = Viewport::new(15, 6);
        vp.set_content("  ● abcdefghijklmnopqrstuv");

        let view = vp.view();
        let wrapped: Vec<&str> = view.lines().filter(|l| !l.trim().is_empty()).collect();

        assert_eq!(wrapped[0], "  ● abcdefghijk");
        assert!(
            wrapped.iter().skip(1).all(|row| row.starts_with("    ")),
            "continuations should align under gutter text: {wrapped:?}"
        );
    }

    #[test]
    fn wrapped_gutter_line_keeps_alignment_at_minimum_transcript_width() {
        let mut vp = Viewport::new(8, 8);
        vp.set_content("  • abcdefghijkl");

        let view = vp.view();
        let wrapped: Vec<&str> = view
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect();

        assert_eq!(wrapped[0], "  • abcd");
        assert!(
            wrapped.iter().skip(1).all(|row| row.starts_with("    ")),
            "narrow continuations should retain the gutter text column: {wrapped:?}"
        );
        assert!(wrapped.iter().all(|row| visible_len(row) <= 8));
    }

    #[test]
    fn wrapped_full_bleed_gutter_line_aligns_after_marker() {
        let mut vp = Viewport::new(9, 8);
        vp.set_content("• abcdefghijklmnop");

        let wrapped = vp
            .view()
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();

        assert_eq!(wrapped[0], "• abcdefg");
        assert!(
            wrapped.iter().skip(1).all(|row| row.starts_with("  ")),
            "full-bleed continuations should retain the gutter text column: {wrapped:?}"
        );
        assert!(wrapped.iter().all(|row| visible_len(row) <= 9));
    }

    #[test]
    fn wrapped_osc8_link_counts_only_visible_label_columns() {
        let linked = "\x1b]8;;https://example.com\x1b\\abcdefghijkl\x1b]8;;\x1b\\";
        let mut vp = Viewport::new(6, 4);
        vp.set_content(linked);

        let rendered = vp.view();
        let rows = rendered
            .lines()
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>();

        assert_eq!(
            rows.iter()
                .map(|row| strip_ansi(row))
                .collect::<Vec<_>>()
                .join(""),
            "abcdefghijkl"
        );
        assert!(rows.iter().all(|row| visible_len(row) <= 6), "{rows:?}");
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
