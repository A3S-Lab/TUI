use crate::element::*;
use crate::grid::{Cell, CellStyle, Grid};
use crate::layout_engine::LayoutResult;

pub fn paint<Msg>(
    root: &Element<Msg>,
    layout: &LayoutResult,
    width: u16,
    height: u16,
) -> Grid {
    let mut grid = Grid::new(width, height);
    let mut idx = 0;
    paint_element(&mut grid, root, layout, &mut idx);
    grid
}

fn paint_element<Msg>(
    grid: &mut Grid,
    element: &Element<Msg>,
    layout: &LayoutResult,
    idx: &mut usize,
) {
    let node = &layout.nodes[*idx];
    *idx += 1;

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
                paint_element(grid, child, layout, idx);
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
                let display_line = match text_el.wrap {
                    TextWrap::Truncate => truncate_str(line, max_w),
                    _ => line.to_string(),
                };
                grid.write_str(node.x, node.y + line_idx as u16, &display_line, &style);
            }
        }
        Element::Spacer => {}
        Element::_Phantom(_) => {}
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
