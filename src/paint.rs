//! Paints an Element tree onto a cell grid using computed layout positions.

use crate::element::*;
use crate::grid::{Cell, CellStyle, Grid};
use crate::layout_engine::{LayoutNode, LayoutResult};
use crate::style::{
    next_display_cell_boundary, split_lines_preserving_trailing_blank, strip_ansi,
    truncate_visible, visible_len, wrap_words,
};

pub fn paint<Msg>(root: &Element<Msg>, layout: &LayoutResult, width: u16, height: u16) -> Grid {
    let mut grid = Grid::new(width, height);
    let mut idx = 0;
    paint_element(
        &mut grid,
        root,
        layout,
        &mut idx,
        width,
        ClipRect::from_grid(width, height),
    );
    grid
}

#[derive(Debug, Clone, Copy)]
struct ClipRect {
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
}

impl ClipRect {
    fn from_grid(width: u16, height: u16) -> Self {
        Self {
            left: 0,
            top: 0,
            right: width as usize,
            bottom: height as usize,
        }
    }

    fn is_empty(self) -> bool {
        self.left >= self.right || self.top >= self.bottom
    }

    fn intersect_node(self, node: &LayoutNode) -> Self {
        let node_left = node.x as usize;
        let node_top = node.y as usize;
        let node_right = node_left.saturating_add(node.width as usize);
        let node_bottom = node_top.saturating_add(node.height as usize);

        Self {
            left: self.left.max(node_left),
            top: self.top.max(node_top),
            right: self.right.min(node_right),
            bottom: self.bottom.min(node_bottom),
        }
    }

    fn clipped_rect(self, x: u16, y: u16, w: u16, h: u16) -> Option<(u16, u16, u16, u16)> {
        let left = self.left.max(x as usize);
        let top = self.top.max(y as usize);
        let right = self.right.min((x as usize).saturating_add(w as usize));
        let bottom = self.bottom.min((y as usize).saturating_add(h as usize));

        if left >= right || top >= bottom {
            return None;
        }

        Some((
            left as u16,
            top as u16,
            (right - left) as u16,
            (bottom - top) as u16,
        ))
    }

    fn contains(self, x: u16, y: u16) -> bool {
        let x = x as usize;
        let y = y as usize;
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }
}

fn paint_element<Msg>(
    grid: &mut Grid,
    element: &Element<Msg>,
    layout: &LayoutResult,
    idx: &mut usize,
    grid_h: u16,
    clip: ClipRect,
) {
    let Some(node) = layout.nodes.get(*idx) else {
        return;
    };
    *idx += 1;

    if clip.is_empty() || node.x as usize >= clip.right || node.y as usize >= clip.bottom {
        skip_children(element, layout, idx);
        return;
    }

    match element {
        Element::Box(box_el) => {
            if let Some(bg) = box_el.style.bg {
                if let Some((x, y, w, h)) =
                    clip.clipped_rect(node.x, node.y, node.width, node.height)
                {
                    grid.fill_bg(x, y, w, h, bg);
                }
            }

            if let Some(border) = &box_el.style.border {
                draw_border(grid, node, *border, box_el.style.border_color, clip);
            }

            let child_clip = match box_el.style.overflow {
                Overflow::Visible => clip,
                Overflow::Hidden => clip.intersect_node(node),
            };
            if child_clip.is_empty() {
                skip_children(element, layout, idx);
                return;
            }

            for child in &box_el.children {
                paint_element(grid, child, layout, idx, grid_h, child_clip);
            }
        }
        Element::Text(text_el) => {
            let style = CellStyle {
                fg: text_el.style.fg,
                bg: text_el.style.bg,
                bold: text_el.style.bold,
                italic: text_el.style.italic,
                underline: text_el.style.underline,
                reverse: text_el.style.reverse,
                dim: text_el.style.dim,
                strikethrough: text_el.style.strikethrough,
            };

            let max_w = node.width as usize;
            let max_h = node.height as usize;
            let mut painted_rows = 0usize;

            for line in split_lines_preserving_trailing_blank(&text_el.content) {
                if painted_rows >= max_h {
                    break;
                }
                let stripped_line;
                let paint_line = if line.contains('\x1b') {
                    stripped_line = strip_ansi(line);
                    stripped_line.as_str()
                } else {
                    line
                };
                match text_el.wrap {
                    TextWrap::Wrap => {
                        if max_w == 0 {
                            if is_zero_width_text(paint_line) {
                                if !paint_text_row(
                                    grid,
                                    node,
                                    painted_rows,
                                    grid_h,
                                    clip,
                                    paint_line,
                                    &style,
                                ) {
                                    break;
                                }
                                painted_rows += 1;
                            }
                            continue;
                        }

                        if visible_len(paint_line) <= max_w {
                            if !paint_text_row(
                                grid,
                                node,
                                painted_rows,
                                grid_h,
                                clip,
                                paint_line,
                                &style,
                            ) {
                                break;
                            }
                            painted_rows += 1;
                            continue;
                        }

                        for display_line in wrap_words(paint_line, max_w) {
                            if painted_rows >= max_h {
                                break;
                            }
                            if !paint_text_row(
                                grid,
                                node,
                                painted_rows,
                                grid_h,
                                clip,
                                &display_line,
                                &style,
                            ) {
                                painted_rows = max_h;
                                break;
                            }
                            painted_rows += 1;
                        }
                    }
                    TextWrap::Truncate => {
                        let display_line;
                        let row_text = if max_w == 0 && is_zero_width_text(paint_line) {
                            paint_line
                        } else {
                            display_line = truncate_visible(paint_line, max_w);
                            display_line.as_str()
                        };
                        if !paint_text_row(grid, node, painted_rows, grid_h, clip, row_text, &style)
                        {
                            break;
                        }
                        painted_rows += 1;
                    }
                    TextWrap::NoWrap => {
                        if !paint_text_row(
                            grid,
                            node,
                            painted_rows,
                            grid_h,
                            clip,
                            paint_line,
                            &style,
                        ) {
                            break;
                        }
                        painted_rows += 1;
                    }
                }
            }
        }
        Element::Spacer => {}
        Element::_Phantom(_) => {}
    }
}

fn is_zero_width_text(text: &str) -> bool {
    !text.is_empty() && visible_len(text) == 0
}

fn paint_text_row(
    grid: &mut Grid,
    node: &LayoutNode,
    row_offset: usize,
    grid_h: u16,
    clip: ClipRect,
    text: &str,
    style: &CellStyle,
) -> bool {
    let Some(row_offset) = u16::try_from(row_offset).ok() else {
        return false;
    };
    let Some(row_y) = node.y.checked_add(row_offset) else {
        return false;
    };
    if row_y >= grid_h || row_y as usize >= clip.bottom {
        return false;
    }
    if (row_y as usize) < clip.top {
        return true;
    }

    if node.x as usize >= clip.right {
        return true;
    }

    let local_from = clip.left.saturating_sub(node.x as usize);
    let clip_width = clip.right.saturating_sub((node.x as usize).max(clip.left));
    if let Some((start_col, clipped)) = clip_visible_cols(text, local_from, clip_width) {
        let write_x = (node.x as usize).saturating_add(start_col);
        grid.write_str(write_x as u16, row_y, &clipped, style);
    }
    true
}

fn clip_visible_cols(text: &str, from: usize, width: usize) -> Option<(usize, String)> {
    if width == 0 {
        return None;
    }

    let end = from.saturating_add(width);
    let mut col = 0usize;
    let mut start_col = None;
    let mut out = String::new();

    let mut index = 0usize;
    while let Some((cell_end, cw)) = next_display_cell_boundary(text, index) {
        let cell = &text[index..cell_end];
        index = cell_end;
        if cw == 0 {
            if col >= from && col < end {
                start_col.get_or_insert(col);
                out.push_str(cell);
            }
            continue;
        }
        let next_col = col.saturating_add(cw);
        if next_col <= from {
            col = next_col;
            continue;
        }
        if col < from {
            col = next_col;
            continue;
        }
        if next_col > end {
            break;
        }

        start_col.get_or_insert(col);
        out.push_str(cell);
        col = next_col;
    }

    start_col.map(|start| (start, out))
}

/// Skip over child nodes in the layout index without rendering.
fn skip_children<Msg>(element: &Element<Msg>, layout: &LayoutResult, idx: &mut usize) {
    if let Element::Box(box_el) = element {
        for child in &box_el.children {
            if *idx >= layout.nodes.len() {
                return;
            }
            *idx += 1;
            skip_children(child, layout, idx);
        }
    }
}

fn draw_border(
    grid: &mut Grid,
    node: &LayoutNode,
    border: BorderStyle,
    color: Option<crate::style::Color>,
    clip: ClipRect,
) {
    let x = node.x;
    let y = node.y;
    let w = node.width;
    let h = node.height;

    if w < 2 || h < 2 {
        return;
    }
    let Some(right) = x.checked_add(w - 1) else {
        return;
    };
    let Some(bottom) = y.checked_add(h - 1) else {
        return;
    };

    let (tl, tr, bl, br, hz, vt) = match border {
        BorderStyle::Single => ('┌', '┐', '└', '┘', '─', '│'),
        BorderStyle::Double => ('╔', '╗', '╚', '╝', '═', '║'),
        BorderStyle::Rounded => ('╭', '╮', '╰', '╯', '─', '│'),
        BorderStyle::Thick => ('┏', '┓', '┗', '┛', '━', '┃'),
    };

    let style = CellStyle {
        fg: color,
        ..Default::default()
    };

    set_clipped(grid, x, y, Cell::styled(tl, &style), clip);
    set_clipped(grid, right, y, Cell::styled(tr, &style), clip);
    set_clipped(grid, x, bottom, Cell::styled(bl, &style), clip);
    set_clipped(grid, right, bottom, Cell::styled(br, &style), clip);

    for col in (x + 1)..right {
        set_clipped(grid, col, y, Cell::styled(hz, &style), clip);
        set_clipped(grid, col, bottom, Cell::styled(hz, &style), clip);
    }

    for row in (y + 1)..bottom {
        set_clipped(grid, x, row, Cell::styled(vt, &style), clip);
        set_clipped(grid, right, row, Cell::styled(vt, &style), clip);
    }
}

fn set_clipped(grid: &mut Grid, x: u16, y: u16, cell: Cell, clip: ClipRect) {
    if clip.contains(x, y) {
        grid.set(x, y, cell);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::{BoxElement, Element, FlexDirection, TextElement, TextWrap};
    use crate::layout_engine::LayoutEngine;
    use crate::style::{Color, Style};

    fn render(el: &Element<()>, w: u16, h: u16) -> Grid {
        let mut engine = LayoutEngine::new();
        let layout = engine.compute(el, w, h);
        paint(el, &layout, w, h)
    }

    #[test]
    fn paint_text_at_origin() {
        let el: Element<()> = Element::Text(TextElement::new("Hi"));
        let grid = render(&el, 10, 5);
        assert_eq!(grid.get(0, 0).ch, 'H');
        assert_eq!(grid.get(1, 0).ch, 'i');
        assert_eq!(grid.get(2, 0).ch, ' ');
    }

    #[test]
    fn paint_text_with_style() {
        let el: Element<()> = Element::Text(TextElement::new("X").bold().reverse().fg(Color::Red));
        let grid = render(&el, 10, 5);
        assert_eq!(grid.get(0, 0).ch, 'X');
        assert!(grid.get(0, 0).bold);
        assert!(grid.get(0, 0).reverse);
        assert_eq!(grid.get(0, 0).fg, Some(Color::Red));
    }

    #[test]
    fn paint_adjacent_zero_width_text_segment_attaches_to_previous_cell() {
        let el: Element<()> = Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Row)
                .child(Element::Text(TextElement::new("e").wrap(TextWrap::NoWrap)))
                .child(Element::Text(
                    TextElement::new("\u{0301}").wrap(TextWrap::NoWrap),
                )),
        );

        let grid = render(&el, 2, 1);

        assert_eq!(grid.get(0, 0).ch, 'e');
        assert_eq!(grid.get(0, 0).combining, "\u{0301}");
        assert_eq!(grid.render_to_string(), "e\u{0301} ");
    }

    #[test]
    fn paint_truncated_zero_width_text_segment_attaches_to_previous_cell() {
        let el: Element<()> = Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Row)
                .child(Element::Text(TextElement::new("e").wrap(TextWrap::NoWrap)))
                .child(Element::Text(
                    TextElement::new("\u{0301}").wrap(TextWrap::Truncate),
                )),
        );

        let grid = render(&el, 2, 1);

        assert_eq!(grid.get(0, 0).ch, 'e');
        assert_eq!(grid.get(0, 0).combining, "\u{0301}");
        assert_eq!(grid.render_to_string(), "e\u{0301} ");
    }

    #[test]
    fn paint_text_strips_ansi_content_before_writing_cells() {
        let styled = Style::new().fg(Color::Red).render("Hi");
        let el: Element<()> = Element::Text(TextElement::new(styled));

        let grid = render(&el, 4, 1);

        assert_eq!(grid.render_to_string(), "Hi  ");
    }

    #[test]
    fn paint_text_wraps_lines_to_node_width() {
        let el: Element<()> = Element::Text(TextElement::new("alpha beta").wrap(TextWrap::Wrap));

        let grid = render(&el, 6, 2);

        assert_eq!(grid.render_to_string(), "alpha \nbeta  ");
    }

    #[test]
    fn paint_text_trailing_blank_line_offsets_following_sibling() {
        let el: Element<()> = Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .child(Element::Text(TextElement::new("A\n")))
                .child(Element::Text(TextElement::new("B"))),
        );

        let grid = render(&el, 4, 3);

        assert_eq!(grid.get(0, 0).ch, 'A');
        assert_eq!(grid.get(0, 1).ch, ' ');
        assert_eq!(grid.get(0, 2).ch, 'B');
    }

    #[test]
    fn paint_box_background() {
        let el: Element<()> = Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .bg(Color::Blue)
                .width(Dimension::Points(5.0))
                .height(Dimension::Points(3.0)),
        );
        let grid = render(&el, 10, 5);
        assert_eq!(grid.get(0, 0).bg, Some(Color::Blue));
        assert_eq!(grid.get(4, 2).bg, Some(Color::Blue));
    }

    #[test]
    fn paint_border_corners() {
        let el: Element<()> = Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .border(BorderStyle::Rounded)
                .width(Dimension::Points(5.0))
                .height(Dimension::Points(3.0)),
        );
        let grid = render(&el, 10, 5);
        assert_eq!(grid.get(0, 0).ch, '╭');
        assert_eq!(grid.get(4, 0).ch, '╮');
        assert_eq!(grid.get(0, 2).ch, '╰');
        assert_eq!(grid.get(4, 2).ch, '╯');
    }

    #[test]
    fn paint_column_layout() {
        let el: Element<()> = Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .child(Element::Text(TextElement::new("A")))
                .child(Element::Text(TextElement::new("B"))),
        );
        let grid = render(&el, 10, 5);
        assert_eq!(grid.get(0, 0).ch, 'A');
        assert_eq!(grid.get(0, 1).ch, 'B');
    }

    #[test]
    fn paint_row_layout() {
        let el: Element<()> = Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Row)
                .child(Element::Text(TextElement::new("X")))
                .child(Element::Text(TextElement::new("Y"))),
        );
        let grid = render(&el, 10, 5);
        assert_eq!(grid.get(0, 0).ch, 'X');
        assert_eq!(grid.get(1, 0).ch, 'Y');
    }

    #[test]
    fn paint_visible_overflow_allows_child_text_past_box() {
        let el: Element<()> = Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .width(Dimension::Points(4.0))
                .height(Dimension::Points(1.0))
                .child(Element::Text(
                    TextElement::new("abcdef").wrap(TextWrap::NoWrap),
                )),
        );

        let grid = render(&el, 8, 1);

        assert_eq!(grid.render_to_string(), "abcdef  ");
    }

    #[test]
    fn paint_visible_overflow_keeps_zero_width_box_children() {
        let el: Element<()> = Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .width(Dimension::Points(0.0))
                .height(Dimension::Points(1.0))
                .child(Element::Text(
                    TextElement::new("abc").wrap(TextWrap::NoWrap),
                )),
        );

        let grid = render(&el, 4, 1);

        assert_eq!(grid.render_to_string(), "abc ");
    }

    #[test]
    fn paint_hidden_overflow_clips_child_text_width() {
        let el: Element<()> = Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .width(Dimension::Points(4.0))
                .height(Dimension::Points(1.0))
                .overflow(Overflow::Hidden)
                .child(Element::Text(
                    TextElement::new("abcdef").wrap(TextWrap::NoWrap),
                )),
        );

        let grid = render(&el, 8, 1);

        assert_eq!(grid.render_to_string(), "abcd    ");
    }

    #[test]
    fn paint_hidden_overflow_drops_wide_glyph_crossing_clip_edge() {
        let el: Element<()> = Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .width(Dimension::Points(3.0))
                .height(Dimension::Points(1.0))
                .overflow(Overflow::Hidden)
                .child(Element::Text(
                    TextElement::new("ab界").wrap(TextWrap::NoWrap),
                )),
        );

        let grid = render(&el, 5, 1);

        assert_eq!(grid.render_to_string(), "ab   ");
    }

    #[test]
    fn paint_hidden_overflow_keeps_zero_width_marks_with_base_glyph() {
        let el: Element<()> = Element::Box(
            BoxElement::new()
                .width(Dimension::Points(1.0))
                .height(Dimension::Points(1.0))
                .overflow(Overflow::Hidden)
                .child(Element::Text(
                    TextElement::new("e\u{0301}x").wrap(TextWrap::NoWrap),
                )),
        );

        let grid = render(&el, 2, 1);

        assert_eq!(grid.render_to_string(), "e\u{0301} ");
    }

    #[test]
    fn paint_hidden_overflow_preserves_columns_after_left_split_wide_glyph() {
        let el: Element<()> = Element::Box(BoxElement::new().overflow(Overflow::Hidden).child(
            Element::Text(TextElement::new("界a").wrap(TextWrap::NoWrap)),
        ));
        let layout = LayoutResult {
            nodes: vec![
                crate::layout_engine::LayoutNode {
                    x: 1,
                    y: 0,
                    width: 3,
                    height: 1,
                },
                crate::layout_engine::LayoutNode {
                    x: 0,
                    y: 0,
                    width: 3,
                    height: 1,
                },
            ],
        };

        let grid = paint(&el, &layout, 4, 1);

        assert_eq!(grid.render_to_string(), "  a ");
    }

    #[test]
    fn clip_visible_cols_drops_wide_glyphs_split_by_clip_edges() {
        assert_eq!(clip_visible_cols("ab界cd", 0, 4), Some((0, "ab界".into())));
        assert_eq!(clip_visible_cols("ab界cd", 0, 3), Some((0, "ab".into())));
        assert_eq!(clip_visible_cols("ab界cd", 3, 3), Some((4, "cd".into())));
    }

    #[test]
    fn clip_visible_cols_keeps_zero_width_marks_with_base_glyph() {
        assert_eq!(
            clip_visible_cols("e\u{0301}x", 0, 1),
            Some((0, "e\u{0301}".into()))
        );
        assert_eq!(
            clip_visible_cols("xe\u{0301}", 1, 1),
            Some((1, "e\u{0301}".into()))
        );
        assert_eq!(clip_visible_cols("e\u{0301}x", 1, 1), Some((1, "x".into())));
    }

    #[test]
    fn clip_visible_cols_preserves_leading_zero_width_marks() {
        assert_eq!(
            clip_visible_cols("\u{0301}x", 0, 1),
            Some((0, "\u{0301}x".into()))
        );
        assert_eq!(
            clip_visible_cols("\u{0301}", 0, 1),
            Some((0, "\u{0301}".into()))
        );
        assert_eq!(clip_visible_cols("\u{0301}x", 1, 1), None);
    }

    #[test]
    fn paint_hidden_overflow_clips_child_rows() {
        let el: Element<()> = Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .width(Dimension::Points(4.0))
                .height(Dimension::Points(1.0))
                .overflow(Overflow::Hidden)
                .child(Element::Text(TextElement::new("top")))
                .child(Element::Text(TextElement::new("bottom"))),
        );

        let grid = render(&el, 8, 3);

        assert_eq!(grid.render_to_string(), "top     \n        \n        ");
    }

    #[test]
    fn paint_hidden_overflow_clips_child_background() {
        let el: Element<()> = Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .width(Dimension::Points(4.0))
                .height(Dimension::Points(1.0))
                .overflow(Overflow::Hidden)
                .child(Element::Box(
                    BoxElement::new()
                        .width(Dimension::Points(6.0))
                        .height(Dimension::Points(1.0))
                        .bg(Color::Blue),
                )),
        );

        let grid = render(&el, 8, 1);

        for x in 0..4 {
            assert_eq!(grid.get(x, 0).bg, Some(Color::Blue));
        }
        for x in 4..8 {
            assert_eq!(grid.get(x, 0).bg, None);
        }
    }

    #[test]
    fn paint_text_truncates_with_ellipsis() {
        let el: Element<()> = Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .width(Dimension::Points(5.0))
                .child(Element::Text(
                    TextElement::new("hello world").wrap(TextWrap::Truncate),
                )),
        );

        let grid = render(&el, 5, 1);

        assert_eq!(grid.render_to_string(), "hell…");
    }

    #[test]
    fn paint_offscreen_culled() {
        let el: Element<()> = Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .child(Element::Text(TextElement::new("visible")))
                .child(Element::Text(TextElement::new("also visible")))
                .child(Element::Text(TextElement::new("offscreen"))),
        );
        // Only 2 rows tall — third child should be culled
        let grid = render(&el, 20, 2);
        assert_eq!(grid.get(0, 0).ch, 'v');
        assert_eq!(grid.get(0, 1).ch, 'a');
    }

    #[test]
    fn paint_missing_layout_nodes_is_blank_and_does_not_panic() {
        let el: Element<()> = Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .child(Element::Text(TextElement::new("hidden"))),
        );
        let layout = LayoutResult { nodes: Vec::new() };

        let grid = paint(&el, &layout, 8, 2);

        assert_eq!(grid.render_to_string(), "        \n        ");
    }

    #[test]
    fn paint_short_layout_nodes_renders_available_prefix() {
        let el: Element<()> = Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .child(Element::Text(TextElement::new("visible")))
                .child(Element::Text(TextElement::new("missing"))),
        );
        let layout = LayoutResult {
            nodes: vec![
                crate::layout_engine::LayoutNode {
                    x: 0,
                    y: 0,
                    width: 8,
                    height: 2,
                },
                crate::layout_engine::LayoutNode {
                    x: 0,
                    y: 0,
                    width: 8,
                    height: 1,
                },
            ],
        };

        let grid = paint(&el, &layout, 8, 2);

        assert!(grid.render_to_string().starts_with("visible "));
    }

    #[test]
    fn paint_overflowing_border_layout_does_not_panic() {
        let el: Element<()> = Element::Box(
            BoxElement::new()
                .border(BorderStyle::Rounded)
                .width(Dimension::Points(4.0))
                .height(Dimension::Points(2.0)),
        );
        let layout = LayoutResult {
            nodes: vec![crate::layout_engine::LayoutNode {
                x: u16::MAX - 1,
                y: u16::MAX - 1,
                width: 4,
                height: 2,
            }],
        };

        let grid = paint(&el, &layout, 8, 2);

        assert_eq!(grid.render_to_string(), "        \n        ");
    }

    #[test]
    fn paint_overflowing_text_row_layout_does_not_panic() {
        let el: Element<()> = Element::Text(TextElement::new("line1\nline2"));
        let layout = LayoutResult {
            nodes: vec![crate::layout_engine::LayoutNode {
                x: 0,
                y: u16::MAX,
                width: 8,
                height: 2,
            }],
        };

        let grid = paint(&el, &layout, 8, 2);

        assert_eq!(grid.render_to_string(), "        \n        ");
    }
}
