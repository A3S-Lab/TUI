//! Paints an Element tree onto a cell grid using computed layout positions.

use crate::element::*;
use crate::grid::{Cell, CellStyle, Grid};
use crate::layout_engine::LayoutResult;
use crate::style::{strip_ansi, truncate_visible, visible_len, wrap_words};

pub fn paint<Msg>(root: &Element<Msg>, layout: &LayoutResult, width: u16, height: u16) -> Grid {
    let mut grid = Grid::new(width, height);
    let mut idx = 0;
    paint_element(&mut grid, root, layout, &mut idx, width, height);
    grid
}

fn paint_element<Msg>(
    grid: &mut Grid,
    element: &Element<Msg>,
    layout: &LayoutResult,
    idx: &mut usize,
    grid_w: u16,
    grid_h: u16,
) {
    let Some(node) = layout.nodes.get(*idx) else {
        return;
    };
    *idx += 1;

    // Early culling: skip elements entirely outside the visible area
    if node.x >= grid_w || node.y >= grid_h {
        skip_children(element, layout, idx);
        return;
    }

    match element {
        Element::Box(box_el) => {
            if let Some(bg) = box_el.style.bg {
                grid.fill_bg(node.x, node.y, node.width, node.height, bg);
            }

            if let Some(border) = &box_el.style.border {
                draw_border(
                    grid,
                    node.x,
                    node.y,
                    node.width,
                    node.height,
                    *border,
                    box_el.style.border_color,
                );
            }

            for child in &box_el.children {
                paint_element(grid, child, layout, idx, grid_w, grid_h);
            }
        }
        Element::Text(text_el) => {
            let style = CellStyle {
                fg: text_el.style.fg,
                bg: text_el.style.bg,
                bold: text_el.style.bold,
                italic: text_el.style.italic,
                underline: text_el.style.underline,
                dim: text_el.style.dim,
                strikethrough: text_el.style.strikethrough,
            };

            let max_w = node.width as usize;
            let max_h = node.height as usize;
            let mut painted_rows = 0usize;

            for line in text_el.content.lines() {
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
                            continue;
                        }

                        if visible_len(paint_line) <= max_w {
                            if !paint_text_row(
                                grid,
                                node.x,
                                node.y,
                                painted_rows,
                                grid_h,
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
                                node.x,
                                node.y,
                                painted_rows,
                                grid_h,
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
                        let display_line = truncate_visible(paint_line, max_w);
                        if !paint_text_row(
                            grid,
                            node.x,
                            node.y,
                            painted_rows,
                            grid_h,
                            &display_line,
                            &style,
                        ) {
                            break;
                        }
                        painted_rows += 1;
                    }
                    TextWrap::NoWrap => {
                        if !paint_text_row(
                            grid,
                            node.x,
                            node.y,
                            painted_rows,
                            grid_h,
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

fn paint_text_row(
    grid: &mut Grid,
    x: u16,
    y: u16,
    row_offset: usize,
    grid_h: u16,
    text: &str,
    style: &CellStyle,
) -> bool {
    let Some(row_offset) = u16::try_from(row_offset).ok() else {
        return false;
    };
    let Some(row_y) = y.checked_add(row_offset) else {
        return false;
    };
    if row_y >= grid_h {
        return false;
    }

    grid.write_str(x, row_y, text, style);
    true
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
    x: u16,
    y: u16,
    w: u16,
    h: u16,
    border: BorderStyle,
    color: Option<crate::style::Color>,
) {
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

    grid.set(x, y, Cell::styled(tl, &style));
    grid.set(right, y, Cell::styled(tr, &style));
    grid.set(x, bottom, Cell::styled(bl, &style));
    grid.set(right, bottom, Cell::styled(br, &style));

    for col in (x + 1)..right {
        grid.set(col, y, Cell::styled(hz, &style));
        grid.set(col, bottom, Cell::styled(hz, &style));
    }

    for row in (y + 1)..bottom {
        grid.set(x, row, Cell::styled(vt, &style));
        grid.set(right, row, Cell::styled(vt, &style));
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
        let el: Element<()> = Element::Text(TextElement::new("X").bold().fg(Color::Red));
        let grid = render(&el, 10, 5);
        assert_eq!(grid.get(0, 0).ch, 'X');
        assert!(grid.get(0, 0).bold);
        assert_eq!(grid.get(0, 0).fg, Some(Color::Red));
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
