use crate::style::{
    next_display_cell_boundary, split_lines_preserving_trailing_blank, visible_len,
};

#[derive(Debug, Clone, Copy)]
pub enum Constraint {
    Fixed(u16),
    Percentage(u16),
    Fill,
    Min(u16),
    Max(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Horizontal,
    Vertical,
}

pub struct Layout {
    direction: Direction,
    items: Vec<(String, Constraint)>,
}

impl Layout {
    pub fn horizontal() -> Self {
        Self {
            direction: Direction::Horizontal,
            items: Vec::new(),
        }
    }

    pub fn vertical() -> Self {
        Self {
            direction: Direction::Vertical,
            items: Vec::new(),
        }
    }

    pub fn item(mut self, content: &str, constraint: Constraint) -> Self {
        self.items.push((content.to_string(), constraint));
        self
    }

    pub fn render(&self, total: u16) -> String {
        let sizes = self.resolve_sizes(total);

        match self.direction {
            Direction::Vertical => self.render_vertical(&sizes),
            Direction::Horizontal => self.render_horizontal(&sizes),
        }
    }

    fn resolve_sizes(&self, total: u16) -> Vec<u16> {
        let count = self.items.len();
        let mut sizes = vec![0u16; count];
        let mut remaining = total;
        let mut fill_indices = Vec::new();

        for (i, (_, constraint)) in self.items.iter().enumerate() {
            match constraint {
                Constraint::Fixed(n) => {
                    sizes[i] = (*n).min(remaining);
                    remaining = remaining.saturating_sub(sizes[i]);
                }
                Constraint::Percentage(p) => {
                    let s = (total as usize)
                        .saturating_mul(*p as usize)
                        .saturating_div(100)
                        .min(remaining as usize) as u16;
                    sizes[i] = s;
                    remaining = remaining.saturating_sub(sizes[i]);
                }
                Constraint::Min(n) => {
                    sizes[i] = (*n).min(remaining);
                    remaining = remaining.saturating_sub(sizes[i]);
                    fill_indices.push(i);
                }
                Constraint::Max(n) => {
                    sizes[i] = (*n).min(remaining);
                    remaining = remaining.saturating_sub(sizes[i]);
                }
                Constraint::Fill => {
                    fill_indices.push(i);
                }
            }
        }

        if !fill_indices.is_empty() {
            let fill_count = fill_indices.len();
            let share = remaining as usize / fill_count;
            let extra = remaining as usize % fill_count;
            for (j, &idx) in fill_indices.iter().enumerate() {
                let add = share + if j == 0 { extra } else { 0 };
                sizes[idx] = sizes[idx].saturating_add(add as u16);
            }
        }

        sizes
    }

    fn render_vertical(&self, sizes: &[u16]) -> String {
        let mut result = Vec::new();

        for (i, (content, _)) in self.items.iter().enumerate() {
            let height = sizes[i] as usize;
            let lines = split_lines_preserving_trailing_blank(content);

            for row in 0..height {
                if row < lines.len() {
                    result.push(lines[row].to_string());
                } else {
                    result.push(String::new());
                }
            }
        }

        result.join("\n")
    }

    fn render_horizontal(&self, sizes: &[u16]) -> String {
        let max_height = self
            .items
            .iter()
            .map(|(content, _)| split_lines_preserving_trailing_blank(content).len())
            .max()
            .unwrap_or(1);

        let columns: Vec<Vec<String>> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, (content, _))| {
                let width = sizes[i] as usize;
                let lines = split_lines_preserving_trailing_blank(content);
                (0..max_height)
                    .map(|row| {
                        if row < lines.len() {
                            pad_or_truncate(lines[row], width)
                        } else {
                            " ".repeat(width)
                        }
                    })
                    .collect()
            })
            .collect();

        let mut result = Vec::new();
        for row in 0..max_height {
            let line: String = columns.iter().map(|col| col[row].as_str()).collect();
            result.push(line);
        }

        result.join("\n")
    }
}

fn pad_or_truncate(s: &str, width: usize) -> String {
    let vis_width = visible_len(s);
    if vis_width >= width {
        truncate_to_width(s, width)
    } else {
        format!("{}{}", s, " ".repeat(width - vis_width))
    }
}

fn truncate_to_width(s: &str, width: usize) -> String {
    let mut out = String::new();
    let mut current_width = 0;
    let mut saw_escape = false;
    let mut truncated = false;
    let mut index = 0usize;

    while index < s.len() {
        if s[index..].starts_with("\x1b[") {
            saw_escape = true;
            let escape_start = index;
            index += "\x1b[".len();
            for next in s[index..].chars() {
                index += next.len_utf8();
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
            out.push_str(&s[escape_start..index]);
            continue;
        }

        let Some((end, cw)) = next_display_cell_boundary(s, index) else {
            break;
        };
        if current_width + cw > width {
            truncated = true;
            break;
        }
        current_width += cw;
        out.push_str(&s[index..end]);
        index = end;
    }

    if truncated && saw_escape {
        out.push_str("\x1b[0m");
    }
    if current_width < width {
        out.push_str(&" ".repeat(width - current_width));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_constraint() {
        let layout = Layout::horizontal()
            .item("A", Constraint::Fixed(10))
            .item("B", Constraint::Fixed(20));
        let sizes = layout.resolve_sizes(80);
        assert_eq!(sizes, vec![10, 20]);
    }

    #[test]
    fn percentage_constraint() {
        let layout = Layout::horizontal()
            .item("A", Constraint::Percentage(50))
            .item("B", Constraint::Percentage(50));
        let sizes = layout.resolve_sizes(80);
        assert_eq!(sizes, vec![40, 40]);
    }

    #[test]
    fn oversized_percentage_clamps_to_remaining_space() {
        let layout = Layout::horizontal()
            .item("A", Constraint::Percentage(u16::MAX))
            .item("B", Constraint::Fill);
        let sizes = layout.resolve_sizes(u16::MAX);

        assert_eq!(sizes, vec![u16::MAX, 0]);
    }

    #[test]
    fn fill_distributes_remaining() {
        let layout = Layout::horizontal()
            .item("A", Constraint::Fixed(20))
            .item("B", Constraint::Fill)
            .item("C", Constraint::Fixed(10));
        let sizes = layout.resolve_sizes(80);
        assert_eq!(sizes[0], 20);
        assert_eq!(sizes[1], 50);
        assert_eq!(sizes[2], 10);
    }

    #[test]
    fn multiple_fills_share_equally() {
        let layout = Layout::horizontal()
            .item("A", Constraint::Fill)
            .item("B", Constraint::Fill);
        let sizes = layout.resolve_sizes(80);
        assert_eq!(sizes[0], 40);
        assert_eq!(sizes[1], 40);
    }

    #[test]
    fn many_fills_do_not_overflow_share_count() {
        let layout = (0..u16::MAX as usize + 1).fold(Layout::horizontal(), |layout, _| {
            layout.item("", Constraint::Fill)
        });
        let sizes = layout.resolve_sizes(3);

        assert_eq!(sizes.iter().copied().map(u32::from).sum::<u32>(), 3);
        assert_eq!(sizes[0], 3);
        assert!(sizes[1..].iter().all(|size| *size == 0));
    }

    #[test]
    fn render_horizontal_basic() {
        let layout = Layout::horizontal()
            .item("left", Constraint::Fixed(6))
            .item("right", Constraint::Fixed(6));
        let output = layout.render(12);
        assert!(output.contains("left"));
        assert!(output.contains("right"));
    }

    #[test]
    fn render_horizontal_preserves_trailing_blank_rows() {
        let layout = Layout::horizontal()
            .item("A\n", Constraint::Fixed(2))
            .item("B", Constraint::Fixed(2));

        assert_eq!(layout.render(4), "A B \n    ");
    }

    #[test]
    fn render_vertical_basic() {
        let layout = Layout::vertical()
            .item("top", Constraint::Fixed(1))
            .item("bottom", Constraint::Fixed(1));
        let output = layout.render(2);
        assert!(output.contains("top"));
        assert!(output.contains("bottom"));
    }

    #[test]
    fn pad_or_truncate_pads() {
        assert_eq!(pad_or_truncate("hi", 5), "hi   ");
    }

    #[test]
    fn pad_or_truncate_truncates() {
        let result = pad_or_truncate("hello world", 5);
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn pad_or_truncate_keeps_zero_width_marks_with_base_glyph() {
        let result = pad_or_truncate("e\u{0301}xyz", 2);

        assert_eq!(result, "e\u{0301}x");
        assert_eq!(visible_len(&result), 2);
    }

    #[test]
    fn pad_or_truncate_resets_ansi_after_truncating_styled_text() {
        let result = pad_or_truncate("\x1b[31mhello\x1b[0m", 3);

        assert_eq!(visible_len(&result), 3);
        assert!(result.ends_with("\x1b[0m"), "{result:?}");
    }

    #[test]
    fn pad_or_truncate_skips_ansi_reset_between_segments() {
        let result = pad_or_truncate("\x1b[32mok\x1b[0mabcdef", 5);

        assert_eq!(visible_len(&result), 5);
        assert_eq!(crate::style::strip_ansi(&result), "okabc");
        assert!(result.contains("\x1b[0mabc"), "{result:?}");
    }
}
