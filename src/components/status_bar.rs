use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{truncate_visible, visible_len, Color, Style};
use crate::theme::{Theme, ThemeRole};

pub struct StatusBar {
    left: String,
    center: String,
    right: String,
    fg: Color,
    bg: Option<Color>,
    bold: bool,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            left: String::new(),
            center: String::new(),
            right: String::new(),
            fg: Color::White,
            bg: Some(Color::BrightBlack),
            bold: false,
        }
    }

    pub fn left(mut self, s: impl Into<String>) -> Self {
        self.left = s.into();
        self
    }

    pub fn center(mut self, s: impl Into<String>) -> Self {
        self.center = s.into();
        self
    }

    pub fn right(mut self, s: impl Into<String>) -> Self {
        self.right = s.into();
        self
    }

    pub fn fg(mut self, color: Color) -> Self {
        self.fg = color;
        self
    }

    pub fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    pub fn no_bg(mut self) -> Self {
        self.bg = None;
        self
    }

    pub fn bold(mut self, enabled: bool) -> Self {
        self.bold = enabled;
        self
    }

    pub fn with_theme(mut self, theme: &Theme) -> Self {
        self.fg = theme.color(ThemeRole::Foreground);
        self.bg = Some(theme.color(ThemeRole::Surface));
        self
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        let mut children: Vec<Element<Msg>> = Vec::new();

        if !self.left.is_empty() {
            children.push(Element::Text(self.text_element(&self.left)));
        }

        children.push(Element::Spacer);

        if !self.center.is_empty() {
            children.push(Element::Text(self.text_element(&self.center)));
            children.push(Element::Spacer);
        }

        if !self.right.is_empty() {
            children.push(Element::Text(self.text_element(&self.right)));
        }

        let mut row = BoxElement::new().direction(FlexDirection::Row);
        if let Some(bg) = self.bg {
            row = row.bg(bg);
        }
        Element::Box(row.children(children))
    }

    pub fn view(&self, width: u16) -> String {
        let w = width as usize;
        if w == 0 {
            return String::new();
        }

        let right = truncate_visible(&self.right, w);
        let right_width = visible_len(&right);
        let right_start = w.saturating_sub(right_width);
        let center = truncate_visible(&self.center, w);
        let center_width = visible_len(&center);
        let center_start = (w.saturating_sub(center_width)) / 2;
        let left_full_width = visible_len(&self.left);
        let center_end = center_start.saturating_add(center_width);
        let left_clear = if left_full_width == 0 {
            center_start >= left_full_width
        } else {
            center_start > left_full_width
        };
        let right_clear = if right_width == 0 {
            center_end <= right_start
        } else {
            center_end < right_start
        };
        let center_fits = center_width > 0 && left_clear && right_clear;

        let line = if center_fits {
            let left_budget = center_start.saturating_sub(1);
            let left = truncate_visible(&self.left, left_budget);
            let left_width = visible_len(&left);
            let mut line = left;
            line.push_str(&" ".repeat(center_start.saturating_sub(left_width)));
            line.push_str(&center);
            line.push_str(&" ".repeat(right_start.saturating_sub(center_start + center_width)));
            line.push_str(&right);
            line
        } else {
            let left_budget = w.saturating_sub(right_width);
            let left = truncate_visible(&self.left, left_budget);
            let left_width = visible_len(&left);
            let mut line = left;
            line.push_str(&" ".repeat(w.saturating_sub(left_width + right_width)));
            line.push_str(&right);
            line
        };

        self.style().render(&line)
    }

    fn text_element(&self, text: &str) -> TextElement {
        let mut element = TextElement::new(text).fg(self.fg);
        if self.bold {
            element = element.bold();
        }
        element
    }

    fn style(&self) -> Style {
        let mut style = Style::new().fg(self.fg);
        if let Some(bg) = self.bg {
            style = style.bg(bg);
        }
        if self.bold {
            style = style.bold();
        }
        style
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, visible_len};

    #[test]
    fn empty_status_bar() {
        let sb = StatusBar::new();
        let view = sb.view(40);
        let plain = strip_ansi(&view);
        assert_eq!(visible_len(&plain), 40);
    }

    #[test]
    fn left_content() {
        let sb = StatusBar::new().left("hello");
        let view = sb.view(40);
        let plain = strip_ansi(&view);
        assert!(plain.starts_with("hello"));
    }

    #[test]
    fn right_content() {
        let sb = StatusBar::new().right("end");
        let view = sb.view(40);
        let plain = strip_ansi(&view);
        assert!(plain.ends_with("end"));
    }

    #[test]
    fn left_and_right() {
        let sb = StatusBar::new().left("L").right("R");
        let view = sb.view(20);
        let plain = strip_ansi(&view);
        assert!(plain.starts_with('L'));
        assert!(plain.ends_with('R'));
        assert_eq!(visible_len(&plain), 20);
    }

    #[test]
    fn center_only_renders_when_it_fills_width() {
        let sb = StatusBar::new().center("mid");
        let view = sb.view(3);
        let plain = strip_ansi(&view);

        assert_eq!(plain, "mid");
        assert_eq!(visible_len(&plain), 3);
    }

    #[test]
    fn hides_center_when_it_would_touch_left_or_right() {
        let sb = StatusBar::new()
            .left("left side is fairly long")
            .center("center")
            .right("right");
        let view = sb.view(32);
        let plain = strip_ansi(&view);

        assert!(!plain.contains("longcenter"));
        assert!(!plain.contains("centerright"));
        assert!(!plain.contains("center"));
        assert_eq!(visible_len(&plain), 32);
    }

    #[test]
    fn truncates_left_to_preserve_right_status() {
        let sb = StatusBar::new()
            .left(" a3s top boxes:10 agents:20 processes:300 events:400 ")
            .right("live")
            .bold(true);
        let view = sb.view(32);
        let plain = strip_ansi(&view);

        assert_eq!(visible_len(&view), 32);
        assert!(plain.ends_with("live"));
        assert!(plain.contains('…'));
        assert!(view.contains("\x1b[1;"));
    }

    #[test]
    fn handles_cjk_width_when_truncating() {
        let sb = StatusBar::new()
            .left(" 状态 中文测试内容")
            .right("运行中")
            .fg(Color::BrightWhite)
            .bg(Color::Blue);
        let view = sb.view(18);
        let plain = strip_ansi(&view);

        assert_eq!(visible_len(&view), 18);
        assert!(plain.ends_with("运行中"));
    }

    #[test]
    fn no_bg_keeps_foreground_without_background() {
        let sb = StatusBar::new()
            .left("left")
            .right("right")
            .fg(Color::Cyan)
            .no_bg();
        let view = sb.view(20);
        let plain = strip_ansi(&view);

        assert_eq!(visible_len(&view), 20);
        assert!(plain.starts_with("left"));
        assert!(plain.ends_with("right"));
        assert!(view.contains("\x1b[36m"));
        assert!(!view.contains("\x1b[46m"));

        let Element::Box(row) = sb.element::<()>() else {
            panic!("expected row");
        };
        assert_eq!(row.style.bg, None);
    }

    #[test]
    fn with_theme_applies_semantic_colors() {
        let theme = Theme::tokyo_night();
        let bar = StatusBar::new().no_bg().with_theme(&theme);

        assert_eq!(bar.fg, theme.color(ThemeRole::Foreground));
        assert_eq!(bar.bg, Some(theme.color(ThemeRole::Surface)));
    }

    #[test]
    fn element_produces_row() {
        let sb = StatusBar::new().left("a").right("b");
        let el: Element<()> = sb.element();
        match el {
            Element::Box(b) => {
                assert_eq!(b.style.flex_direction, crate::element::FlexDirection::Row);
            }
            _ => panic!("expected Box"),
        }
    }
}
