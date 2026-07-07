use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, repeat_visible_char, truncate_visible, visible_len, Color, Style};

const MAX_PANEL_FRAME_ROWS: usize = u16::MAX as usize;

/// Border style used by [`PanelFrame`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PanelFrameBorder {
    Single,
    Double,
    #[default]
    Rounded,
    Thick,
}

impl PanelFrameBorder {
    fn chars(self) -> PanelFrameChars {
        match self {
            Self::Single => PanelFrameChars {
                tl: '┌',
                tr: '┐',
                bl: '└',
                br: '┘',
                h: '─',
                v: '│',
            },
            Self::Double => PanelFrameChars {
                tl: '╔',
                tr: '╗',
                bl: '╚',
                br: '╝',
                h: '═',
                v: '║',
            },
            Self::Rounded => PanelFrameChars {
                tl: '╭',
                tr: '╮',
                bl: '╰',
                br: '╯',
                h: '─',
                v: '│',
            },
            Self::Thick => PanelFrameChars {
                tl: '┏',
                tr: '┓',
                bl: '┗',
                br: '┛',
                h: '━',
                v: '┃',
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct PanelFrameChars {
    tl: char,
    tr: char,
    bl: char,
    br: char,
    h: char,
    v: char,
}

/// Fixed-size titled terminal frame for split panes, previews, and footers.
///
/// `PanelFrame` renders a title embedded in the top border, body rows fitted to
/// the inner width, and a bottom border. It is intentionally line-rendered so
/// callers can horizontally join panels without first building an element tree.
#[derive(Debug, Clone)]
pub struct PanelFrame {
    title: String,
    rows: Vec<String>,
    focused: bool,
    border: PanelFrameBorder,
    border_color: Color,
    focused_border_color: Color,
    title_color: Color,
    focused_title_color: Color,
}

impl PanelFrame {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            rows: Vec::new(),
            focused: false,
            border: PanelFrameBorder::Rounded,
            border_color: Color::BrightBlack,
            focused_border_color: Color::Cyan,
            title_color: Color::White,
            focused_title_color: Color::Cyan,
        }
    }

    pub fn rows(mut self, rows: impl IntoIterator<Item = String>) -> Self {
        self.rows = rows.into_iter().take(MAX_PANEL_FRAME_ROWS).collect();
        self
    }

    pub fn row(mut self, row: impl Into<String>) -> Self {
        if self.rows.len() < MAX_PANEL_FRAME_ROWS {
            self.rows.push(row.into());
        }
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn border(mut self, border: PanelFrameBorder) -> Self {
        self.border = border;
        self
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }

    pub fn focused_border_color(mut self, color: Color) -> Self {
        self.focused_border_color = color;
        self
    }

    pub fn title_color(mut self, color: Color) -> Self {
        self.title_color = color;
        self
    }

    pub fn focused_title_color(mut self, color: Color) -> Self {
        self.focused_title_color = color;
        self
    }

    pub fn title_value(&self) -> &str {
        &self.title
    }

    pub fn rows_value(&self) -> &[String] {
        &self.rows
    }

    pub fn lines(&self, width: u16, height: usize) -> Vec<String> {
        let width = width as usize;
        if height == 0 {
            return Vec::new();
        }
        if width == 0 {
            return vec![String::new(); height];
        }
        if width == 1 {
            return (0..height)
                .map(|_| self.border_style().render("│"))
                .collect();
        }

        let chars = self.border.chars();
        let inner_width = width.saturating_sub(2);
        let border_style = self.border_style();
        let title_style = self.title_style();
        let mut lines = Vec::with_capacity(height);

        lines.push(self.top_line(chars, inner_width, &border_style, &title_style));
        if height == 1 {
            return lines;
        }

        let body_height = height.saturating_sub(2);
        for row in self.rows.iter().take(body_height) {
            lines.push(format!(
                "{}{}{}",
                border_style.render(&chars.v.to_string()),
                fit_visible(row, inner_width),
                border_style.render(&chars.v.to_string())
            ));
        }
        while lines.len() < height.saturating_sub(1) {
            lines.push(format!(
                "{}{}{}",
                border_style.render(&chars.v.to_string()),
                " ".repeat(inner_width),
                border_style.render(&chars.v.to_string())
            ));
        }
        lines.push(format!(
            "{}{}{}",
            border_style.render(&chars.bl.to_string()),
            border_style.render(&repeat_visible_char(chars.h, inner_width)),
            border_style.render(&chars.br.to_string())
        ));

        lines
    }

    pub fn view(&self, width: u16, height: usize) -> String {
        self.lines(width, height).join("\n")
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        let mut children = vec![Element::Text(
            TextElement::new(self.title.clone())
                .fg(if self.focused {
                    self.focused_title_color
                } else {
                    self.title_color
                })
                .bold(),
        )];
        children.extend(
            self.rows
                .iter()
                .cloned()
                .map(|row| Element::Text(TextElement::new(row))),
        );

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(children),
        )
    }

    fn top_line(
        &self,
        chars: PanelFrameChars,
        inner_width: usize,
        border_style: &Style,
        title_style: &Style,
    ) -> String {
        if inner_width < 4 {
            return format!(
                "{}{}{}",
                border_style.render(&chars.tl.to_string()),
                border_style.render(&repeat_visible_char(chars.h, inner_width)),
                border_style.render(&chars.tr.to_string())
            );
        }

        let title_budget = inner_width.saturating_sub(4);
        let mut title = truncate_visible(&self.title, title_budget);
        while visible_len(&title) > title_budget {
            title.pop();
        }

        let title_width = visible_len(&title);
        let title = title_style.render(&title);
        let fill_width = inner_width.saturating_sub(title_width + 3);
        format!(
            "{}{}{}{}{}",
            border_style.render(&format!("{}{} ", chars.tl, chars.h)),
            title,
            border_style.render(" "),
            border_style.render(&repeat_visible_char(chars.h, fill_width)),
            border_style.render(&chars.tr.to_string())
        )
    }

    fn border_style(&self) -> Style {
        Style::new().fg(if self.focused {
            self.focused_border_color
        } else {
            self.border_color
        })
    }

    fn title_style(&self) -> Style {
        let mut style = Style::new().fg(if self.focused {
            self.focused_title_color
        } else {
            self.title_color
        });
        if self.focused {
            style = style.bold();
        }
        style
    }
}

impl Default for PanelFrame {
    fn default() -> Self {
        Self::new("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, visible_len};

    #[test]
    fn renders_exact_rounded_frame_with_title() {
        let lines = PanelFrame::new("title")
            .row("hi")
            .focused(true)
            .lines(20, 5);

        assert_eq!(lines.len(), 5);
        assert!(lines.iter().all(|line| visible_len(line) == 20));
        assert!(strip_ansi(&lines[0]).starts_with("╭─ title"));
        assert!(strip_ansi(&lines[0]).ends_with('╮'));
        assert_eq!(strip_ansi(&lines[1]), "│hi                │");
        assert_eq!(strip_ansi(&lines[4]), "╰──────────────────╯");
        assert!(lines[0].contains("\x1b["));
        assert!(lines[0].contains("\x1b[1;36mtitle\x1b[0m"));
    }

    #[test]
    fn fits_styled_body_rows_and_title_to_width() {
        let row = Style::new()
            .fg(Color::Yellow)
            .render("body row with 中文 and a very long tail");
        let lines = PanelFrame::new("very long title with 中文")
            .row(row)
            .lines(18, 3);
        let plain = lines
            .iter()
            .map(|line| strip_ansi(line))
            .collect::<Vec<_>>();

        assert!(lines.iter().all(|line| visible_len(line) == 18));
        assert!(plain[0].contains('…'), "{plain:?}");
        assert!(plain[1].contains('…'), "{plain:?}");
        assert!(lines[1].contains("\x1b["));
    }

    #[test]
    fn can_render_other_border_styles() {
        let plain = strip_ansi(
            &PanelFrame::new("x")
                .border(PanelFrameBorder::Double)
                .view(8, 3),
        );

        assert!(plain.starts_with("╔═ x"));
        assert!(plain.contains("║"));
        assert!(plain.ends_with("╚══════╝"));
    }

    #[test]
    fn handles_tiny_sizes_without_overflowing() {
        assert_eq!(PanelFrame::new("x").lines(0, 2), vec!["", ""]);

        let one = PanelFrame::new("x").lines(1, 2);
        assert_eq!(one.len(), 2);
        assert!(one.iter().all(|line| visible_len(line) == 1));

        let two = PanelFrame::new("x").view(2, 1);
        assert_eq!(visible_len(&two), 2);
    }

    #[test]
    fn element_exposes_title_and_rows_for_tree_rendering() {
        let element: Element<()> = PanelFrame::new("Panel").row("body").element();

        match element {
            Element::Box(row) => {
                assert_eq!(row.children.len(), 2);
                match &row.children[0] {
                    Element::Text(text) => assert_eq!(text.content, "Panel"),
                    _ => panic!("expected title text"),
                }
            }
            _ => panic!("expected box element"),
        }
    }
}
