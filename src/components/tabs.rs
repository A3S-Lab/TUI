use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::event::{KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use crate::style::{fit_visible, visible_len, Color, Style};
use crossterm::event::KeyCode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabSegment {
    text: String,
    color: Option<Color>,
    bold: bool,
}

impl TabSegment {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: None,
            bold: false,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

pub struct Tabs {
    labels: Vec<String>,
    tab_colors: Vec<Option<Color>>,
    active: usize,
    focused: bool,
    x_offset: u16,
    suffix_segments: Vec<TabSegment>,
    active_fg: Color,
    active_bg: Color,
    inactive_fg: Color,
    suffix_fg: Color,
    gap: usize,
}

#[derive(Debug, Clone)]
pub enum TabsMsg {
    Changed(usize),
}

impl Tabs {
    pub fn new(labels: Vec<impl Into<String>>) -> Self {
        Self {
            labels: labels.into_iter().map(|l| l.into()).collect(),
            tab_colors: Vec::new(),
            active: 0,
            focused: true,
            x_offset: 0,
            suffix_segments: Vec::new(),
            active_fg: Color::BrightWhite,
            active_bg: Color::Blue,
            inactive_fg: Color::BrightBlack,
            suffix_fg: Color::BrightBlack,
            gap: 1,
        }
    }

    pub fn focus(&mut self) {
        self.focused = true;
    }
    pub fn blur(&mut self) {
        self.focused = false;
    }
    pub fn active(&self) -> usize {
        self.active
    }
    pub fn labels(&self) -> &[String] {
        &self.labels
    }
    pub fn active_label(&self) -> Option<&str> {
        self.labels.get(self.active).map(String::as_str)
    }
    pub fn set_active(&mut self, idx: usize) {
        if idx < self.labels.len() {
            self.active = idx;
        }
    }

    pub fn tab_color(mut self, index: usize, color: Color) -> Self {
        if index < self.labels.len() {
            self.ensure_tab_colors();
            self.tab_colors[index] = Some(color);
        }
        self
    }

    pub fn tab_colors(mut self, colors: Vec<Option<Color>>) -> Self {
        self.tab_colors = colors;
        self.tab_colors.resize(self.labels.len(), None);
        self
    }

    pub fn suffix(mut self, text: impl Into<String>) -> Self {
        self.suffix_segments.push(TabSegment::new(text));
        self
    }

    pub fn segment(mut self, segment: TabSegment) -> Self {
        self.suffix_segments.push(segment);
        self
    }

    pub fn clear_segments(&mut self) {
        self.suffix_segments.clear();
    }

    pub fn active_colors(mut self, fg: Color, bg: Color) -> Self {
        self.active_fg = fg;
        self.active_bg = bg;
        self
    }

    pub fn inactive_color(mut self, color: Color) -> Self {
        self.inactive_fg = color;
        self
    }

    pub fn suffix_color(mut self, color: Color) -> Self {
        self.suffix_fg = color;
        self
    }

    pub fn gap(mut self, gap: usize) -> Self {
        self.gap = gap;
        self
    }

    /// Set the horizontal offset for mouse click calculations.
    pub fn set_x_offset(&mut self, x: u16) {
        self.x_offset = x;
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> Option<TabsMsg> {
        if !self.focused {
            return None;
        }
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => {
                if self.active > 0 {
                    self.active -= 1;
                    Some(TabsMsg::Changed(self.active))
                } else {
                    None
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if self.active + 1 < self.labels.len() {
                    self.active += 1;
                    Some(TabsMsg::Changed(self.active))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Handle mouse click to select a tab.
    pub fn handle_mouse(&mut self, mouse: &MouseEvent) -> Option<TabsMsg> {
        if !self.focused {
            return None;
        }
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let click_x = mouse.column.saturating_sub(self.x_offset) as usize;
                let mut x = 0;
                for (i, label) in self.labels.iter().enumerate() {
                    let tab_width = visible_len(label) + 2; // " label "
                    if click_x >= x && click_x < x + tab_width {
                        self.active = i;
                        return Some(TabsMsg::Changed(i));
                    }
                    x += tab_width + self.gap;
                }
                None
            }
            _ => None,
        }
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        let mut children: Vec<Element<Msg>> = Vec::new();
        for (i, label) in self.labels.iter().enumerate() {
            let padded = format!(" {} ", label);
            if i == self.active {
                children.push(Element::Text(
                    TextElement::new(padded)
                        .bold()
                        .fg(self.active_fg)
                        .bg(self.active_bg_for(i)),
                ));
            } else {
                children.push(Element::Text(
                    TextElement::new(padded).fg(self.inactive_fg_for(i)),
                ));
            }
            if i + 1 < self.labels.len() {
                children.push(Element::Text(
                    TextElement::new(" ".repeat(self.gap)).fg(self.inactive_fg),
                ));
            }
        }
        for segment in &self.suffix_segments {
            let mut text = TextElement::new(format!("  {}", segment.text))
                .fg(segment.color.unwrap_or(self.suffix_fg));
            if segment.bold {
                text = text.bold();
            }
            children.push(Element::Text(text));
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Row)
                .children(children),
        )
    }

    pub fn view(&self, width: u16) -> String {
        let width = width as usize;
        if width == 0 {
            return String::new();
        }

        let mut line = String::new();
        for (index, label) in self.labels.iter().enumerate() {
            if index > 0 {
                line.push_str(&" ".repeat(self.gap));
            }
            let raw = format!(" {label} ");
            if index == self.active {
                line.push_str(
                    &Style::new()
                        .fg(self.active_fg)
                        .bg(self.active_bg_for(index))
                        .bold()
                        .render(&raw),
                );
            } else {
                line.push_str(&Style::new().fg(self.inactive_fg_for(index)).render(&raw));
            }
        }

        for segment in &self.suffix_segments {
            line.push_str("  ");
            let mut style = Style::new().fg(segment.color.unwrap_or(self.suffix_fg));
            if segment.bold {
                style = style.bold();
            }
            line.push_str(&style.render(&segment.text));
        }

        fit_visible(&line, width)
    }

    fn ensure_tab_colors(&mut self) {
        self.tab_colors.resize(self.labels.len(), None);
    }

    fn tab_color_value(&self, index: usize) -> Option<Color> {
        self.tab_colors.get(index).copied().flatten()
    }

    fn active_bg_for(&self, index: usize) -> Color {
        self.tab_color_value(index).unwrap_or(self.active_bg)
    }

    fn inactive_fg_for(&self, index: usize) -> Color {
        self.tab_color_value(index).unwrap_or(self.inactive_fg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
        }
    }

    #[test]
    fn initial_state() {
        let tabs = Tabs::new(vec!["A", "B", "C"]);
        assert_eq!(tabs.active(), 0);
    }

    #[test]
    fn navigate_right() {
        let mut tabs = Tabs::new(vec!["A", "B", "C"]);
        tabs.handle_key(&key(KeyCode::Right));
        assert_eq!(tabs.active(), 1);
        tabs.handle_key(&key(KeyCode::Char('l')));
        assert_eq!(tabs.active(), 2);
    }

    #[test]
    fn navigate_left() {
        let mut tabs = Tabs::new(vec!["A", "B", "C"]);
        tabs.handle_key(&key(KeyCode::Right));
        tabs.handle_key(&key(KeyCode::Right));
        tabs.handle_key(&key(KeyCode::Left));
        assert_eq!(tabs.active(), 1);
    }

    #[test]
    fn bounds_check() {
        let mut tabs = Tabs::new(vec!["A", "B"]);
        tabs.handle_key(&key(KeyCode::Left));
        assert_eq!(tabs.active(), 0);
        tabs.handle_key(&key(KeyCode::Right));
        tabs.handle_key(&key(KeyCode::Right));
        assert_eq!(tabs.active(), 1);
    }

    #[test]
    fn set_active() {
        let mut tabs = Tabs::new(vec!["A", "B", "C"]);
        tabs.set_active(2);
        assert_eq!(tabs.active(), 2);
        tabs.set_active(99);
        assert_eq!(tabs.active(), 2);
    }

    #[test]
    fn returns_changed_msg() {
        let mut tabs = Tabs::new(vec!["A", "B"]);
        let msg = tabs.handle_key(&key(KeyCode::Right));
        assert!(matches!(msg, Some(TabsMsg::Changed(1))));
    }

    #[test]
    fn view_renders_active_tab_suffix_and_fixed_width() {
        let tabs = Tabs::new(vec!["Agents", "Containers", "Events"])
            .active_colors(Color::Black, Color::Cyan)
            .inactive_color(Color::BrightBlack)
            .suffix("/filter")
            .segment(TabSegment::new("focus:api").color(Color::Cyan));
        let rendered = tabs.view(72);
        let plain = crate::style::strip_ansi(&rendered);

        assert!(plain.contains("Agents"));
        assert!(plain.contains("/filter"));
        assert!(plain.contains("focus:api"));
        assert_eq!(visible_len(&rendered), 72);
        assert!(rendered.contains("\x1b["));
    }

    #[test]
    fn view_truncates_cjk_suffix_to_width() {
        let tabs = Tabs::new(vec!["代理", "容器"])
            .suffix("过滤:中文测试内容 with long suffix")
            .gap(2);
        let rendered = tabs.view(40);

        assert_eq!(visible_len(&rendered), 40);
        assert!(crate::style::strip_ansi(&rendered).contains("中文"));
    }

    #[test]
    fn mouse_uses_display_width_for_wide_labels() {
        let mut tabs = Tabs::new(vec!["代理", "Containers"]);
        let msg = tabs.handle_mouse(&MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 8,
            row: 0,
            modifiers: KeyModifiers::NONE,
        });

        assert!(matches!(msg, Some(TabsMsg::Changed(1))));
        assert_eq!(tabs.active(), 1);
    }

    #[test]
    fn per_tab_colors_style_active_and_inactive_tabs() {
        let tabs = Tabs::new(vec!["a3s-code", "Claude Code", "Codex"])
            .active_colors(Color::Black, Color::Blue)
            .inactive_color(Color::BrightBlack)
            .tab_color(1, Color::Yellow)
            .tab_color(2, Color::Rgb(115, 218, 202));
        let rendered = tabs.view(64);

        assert!(rendered.contains("\x1b[1;30;")); // active fg remains black
        assert!(rendered.contains("\x1b[33")); // inactive Claude tab uses yellow
        assert!(rendered.contains("38;2;115;218;202")); // inactive Codex tab uses teal
    }

    #[test]
    fn active_tab_uses_its_configured_background_color() {
        let mut tabs = Tabs::new(vec!["a3s-code", "Claude Code"])
            .active_colors(Color::Black, Color::Blue)
            .tab_color(1, Color::Yellow);
        tabs.set_active(1);
        let rendered = tabs.view(40);

        assert!(rendered.contains("\x1b[1;30;43")); // bold black on yellow
    }
}
