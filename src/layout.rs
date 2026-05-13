use crate::style::visible_len;

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
                    let s = (total as u32 * *p as u32 / 100) as u16;
                    sizes[i] = s.min(remaining);
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
            let share = remaining / fill_indices.len() as u16;
            let extra = remaining % fill_indices.len() as u16;
            for (j, &idx) in fill_indices.iter().enumerate() {
                sizes[idx] += share + if j == 0 { extra } else { 0 };
            }
        }

        sizes
    }

    fn render_vertical(&self, sizes: &[u16]) -> String {
        let mut result = Vec::new();

        for (i, (content, _)) in self.items.iter().enumerate() {
            let height = sizes[i] as usize;
            let lines: Vec<&str> = content.lines().collect();

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
            .map(|(content, _)| content.lines().count().max(1))
            .max()
            .unwrap_or(1);

        let columns: Vec<Vec<String>> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, (content, _))| {
                let width = sizes[i] as usize;
                let lines: Vec<&str> = content.lines().collect();
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
    let mut in_escape = false;

    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
            out.push(c);
            continue;
        }
        if in_escape {
            out.push(c);
            if c.is_ascii_alphabetic() {
                in_escape = false;
            }
            continue;
        }
        let cw = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if current_width + cw > width {
            break;
        }
        current_width += cw;
        out.push(c);
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
    fn render_horizontal_basic() {
        let layout = Layout::horizontal()
            .item("left", Constraint::Fixed(6))
            .item("right", Constraint::Fixed(6));
        let output = layout.render(12);
        assert!(output.contains("left"));
        assert!(output.contains("right"));
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
}
