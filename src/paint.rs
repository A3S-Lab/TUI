//! Paints an Element tree onto a cell grid using computed layout positions.

use crate::element::*;
use crate::grid::{Cell, CellStyle, Grid};
use crate::layout_engine::LayoutResult;

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
    let node = &layout.nodes[*idx];
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

            for (line_idx, line) in text_el.content.lines().enumerate() {
                if line_idx >= max_h {
                    break;
                }
                let row_y = node.y + line_idx as u16;
                if row_y >= grid_h {
                    break;
                }
                let display_line = match text_el.wrap {
                    TextWrap::Truncate => truncate_str(line, max_w),
                    _ => line.to_string(),
                };
                grid.write_str(node.x, row_y, &display_line, &style);
            }
        }
        Element::Spacer => {}
        Element::_Phantom(_) => {}
    }
}

/// Skip over child nodes in the layout index without rendering.
fn skip_children<Msg>(element: &Element<Msg>, layout: &LayoutResult, idx: &mut usize) {
    if let Element::Box(box_el) = element {
        for child in &box_el.children {
            let _ = &layout.nodes[*idx];
            *idx += 1;
            skip_children(child, layout, idx);
        }
    }
}

fn truncate_str(s: &str, max_width: usize) -> String {
    let mut out = String::new();
    let mut w = 0;
    for ch in s.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if w + cw > max_width {
            break;
        }
        out.push(ch);
        w += cw;
    }
    out
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
    grid.set(x + w - 1, y, Cell::styled(tr, &style));
    grid.set(x, y + h - 1, Cell::styled(bl, &style));
    grid.set(x + w - 1, y + h - 1, Cell::styled(br, &style));

    for col in (x + 1)..(x + w - 1) {
        grid.set(col, y, Cell::styled(hz, &style));
        grid.set(col, y + h - 1, Cell::styled(hz, &style));
    }

    for row in (y + 1)..(y + h - 1) {
        grid.set(x, row, Cell::styled(vt, &style));
        grid.set(x + w - 1, row, Cell::styled(vt, &style));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::{BoxElement, Element, FlexDirection, TextElement};
    use crate::layout_engine::LayoutEngine;
    use crate::style::Color;

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
    fn paint_truncate_str() {
        assert_eq!(truncate_str("hello world", 5), "hello");
        assert_eq!(truncate_str("hi", 10), "hi");
        assert_eq!(truncate_str("", 5), "");
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
}
