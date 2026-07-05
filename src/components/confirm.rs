use crate::element::{BorderStyle, BoxElement, Element, FlexDirection, TextElement};
use crate::event::{KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use crate::style::{
    center_visible, fit_visible, truncate_visible, visible_len, wrap_words, Color, Style,
};
use crossterm::event::KeyCode;

const MAX_CONFIRM_WIDTH: usize = u16::MAX as usize;

/// A confirmation dialog with Yes/No options.
pub struct Confirm {
    title: Option<String>,
    message: String,
    selected: bool,
    yes_label: String,
    no_label: String,
    hint: Option<String>,
    max_width: usize,
    fg: Color,
    bg: Option<Color>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmMsg {
    Confirmed,
    Cancelled,
}

impl Confirm {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            title: None,
            message: message.into(),
            selected: true,
            yes_label: "Yes".to_string(),
            no_label: "No".to_string(),
            hint: None,
            max_width: 58,
            fg: Color::BrightWhite,
            bg: None,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_labels(mut self, yes: impl Into<String>, no: impl Into<String>) -> Self {
        self.yes_label = yes.into();
        self.no_label = no.into();
        self
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn selected(mut self, yes_selected: bool) -> Self {
        self.selected = yes_selected;
        self
    }

    pub fn selected_yes(&self) -> bool {
        self.selected
    }

    pub fn max_width(mut self, width: usize) -> Self {
        self.max_width = width.clamp(8, MAX_CONFIRM_WIDTH);
        self
    }

    pub fn colors(mut self, fg: Color, bg: Option<Color>) -> Self {
        self.fg = fg;
        self.bg = bg;
        self
    }

    pub fn danger(mut self) -> Self {
        self.fg = Color::BrightWhite;
        self.bg = Some(Color::Red);
        self
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> Option<ConfirmMsg> {
        match key.code {
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Tab => {
                self.selected = !self.selected;
                None
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.selected = !self.selected;
                None
            }
            KeyCode::Char('y') | KeyCode::Char('Y') => Some(ConfirmMsg::Confirmed),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => Some(ConfirmMsg::Cancelled),
            KeyCode::Enter => {
                if self.selected {
                    Some(ConfirmMsg::Confirmed)
                } else {
                    Some(ConfirmMsg::Cancelled)
                }
            }
            _ => None,
        }
    }

    pub fn handle_mouse(&mut self, mouse: &MouseEvent) -> Option<ConfirmMsg> {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if self.selected {
                    Some(ConfirmMsg::Confirmed)
                } else {
                    Some(ConfirmMsg::Cancelled)
                }
            }
            _ => None,
        }
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        self.element_with_content_width(None)
    }

    pub fn element_with_width<Msg>(&self, width: u16) -> Element<Msg> {
        let width = width as usize;
        if width == 0 {
            return Element::Box(BoxElement::new().direction(FlexDirection::Column));
        }

        let content_width = self.max_width.min(width).saturating_sub(4).max(1);
        self.element_with_content_width(Some(content_width))
    }

    fn element_with_content_width<Msg>(&self, content_width: Option<usize>) -> Element<Msg> {
        let mut children = Vec::new();
        if let Some(title) = self.title.as_deref().filter(|title| !title.is_empty()) {
            let title = Self::bounded_text(title, content_width);
            children.push(Element::Text(
                TextElement::new(title).bold().fg(Color::BrightWhite),
            ));
        }

        if let Some(width) = content_width {
            for line in wrap_words(&self.message, width.max(1)) {
                children.push(Element::Text(TextElement::new(line).bold()));
            }
        } else {
            children.push(Element::Text(TextElement::new(&self.message).bold()));
        }

        let button_width =
            content_width.map(|width| width.saturating_sub(2).saturating_div(2).max(1));
        let yes_el =
            self.button_element(&self.yes_label, self.selected, Color::Green, button_width);
        let no_el = self.button_element(&self.no_label, !self.selected, Color::Red, button_width);

        children.push(Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Row)
                .gap(2)
                .child(yes_el)
                .child(no_el),
        ));
        if let Some(hint) = self.hint.as_deref().filter(|hint| !hint.is_empty()) {
            children.push(Element::Text(
                TextElement::new(Self::bounded_text(hint, content_width)).fg(Color::BrightBlack),
            ));
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .border(BorderStyle::Rounded)
                .border_color(Color::BrightBlack)
                .padding(1)
                .gap(1)
                .children(children),
        )
    }

    fn button_element<Msg>(
        &self,
        label: &str,
        selected: bool,
        selected_bg: Color,
        width: Option<usize>,
    ) -> Element<Msg> {
        let raw = if selected {
            format!(" [{label}] ")
        } else {
            format!("  {label}  ")
        };
        let content = Self::bounded_text(&raw, width);
        if selected {
            Element::Text(
                TextElement::new(content)
                    .bold()
                    .fg(Color::White)
                    .bg(selected_bg),
            )
        } else {
            Element::Text(TextElement::new(content).fg(Color::BrightBlack))
        }
    }

    fn bounded_text(value: &str, width: Option<usize>) -> String {
        width
            .map(|width| truncate_visible(value, width))
            .unwrap_or_else(|| value.to_string())
    }

    /// Render a horizontally centered confirmation box.
    pub fn box_view(&self, screen_width: u16) -> String {
        let screen_width = screen_width as usize;
        if screen_width == 0 {
            return String::new();
        }

        let max_inner = screen_width.saturating_sub(2);
        if max_inner == 0 {
            return fit_visible("", screen_width);
        }
        let inner = self.max_width.min(max_inner).max(max_inner.min(8));
        let border = "─".repeat(inner);
        let mut rows = vec![format!("┌{border}┐")];

        if let Some(title) = self.title.as_deref().filter(|title| !title.is_empty()) {
            rows.push(format!("│{}│", center_visible(title, inner)));
            rows.push(format!("│{}│", " ".repeat(inner)));
        }

        for line in wrap_words(&self.message, inner.saturating_sub(4).max(1)) {
            rows.push(format!("│{}│", center_visible(&line, inner)));
        }

        rows.push(format!("│{}│", " ".repeat(inner)));
        rows.push(format!("│{}│", center_visible(&self.prompt(), inner)));

        if let Some(hint) = self.hint.as_deref().filter(|hint| !hint.is_empty()) {
            rows.push(format!("│{}│", center_visible(hint, inner)));
        }

        rows.push(format!("└{border}┘"));

        let left = screen_width.saturating_sub(inner + 2) / 2;
        rows.into_iter()
            .map(|row| format!("{}{}", " ".repeat(left), self.style_line(&row)))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Render a compact inline confirmation row.
    pub fn line(&self, width: u16) -> String {
        let width = width as usize;
        if width == 0 {
            return String::new();
        }

        let prompt = self.compact_prompt();
        let suffix = match self.hint.as_deref().filter(|hint| !hint.is_empty()) {
            Some(hint) => format!("{prompt}  {hint}"),
            None => prompt,
        };
        let suffix_len = visible_len(&suffix);
        let message_width = width.saturating_sub(suffix_len.saturating_add(2));
        let message = truncate_visible(&self.message, message_width);
        let raw = if message.is_empty() {
            suffix
        } else {
            format!("{message}  {suffix}")
        };
        fit_visible(&self.style_line(&raw), width)
    }

    /// Render a full-screen confirmation view centered in both axes.
    pub fn view(&self, screen_width: u16, screen_height: u16) -> String {
        let width = screen_width as usize;
        let height = screen_height as usize;
        if width == 0 || height == 0 {
            return String::new();
        }

        let box_view = self.box_view(screen_width);
        let box_lines = box_view.lines().collect::<Vec<_>>();
        let top = height.saturating_sub(box_lines.len()) / 2;
        let mut lines = Vec::with_capacity(height);
        for _ in 0..top {
            lines.push(String::new());
        }
        lines.extend(box_lines.into_iter().map(ToString::to_string));
        lines.truncate(height);
        while lines.len() < height {
            lines.push(String::new());
        }

        lines
            .into_iter()
            .map(|line| fit_visible(&line, width))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn prompt(&self) -> String {
        if self.bg.is_some() {
            format!(
                "[ {} / Enter ] confirm     [ {} / Esc ] cancel",
                self.yes_label, self.no_label
            )
        } else {
            let yes = if self.selected {
                format!("[{}]", self.yes_label)
            } else {
                self.yes_label.clone()
            };
            let no = if !self.selected {
                format!("[{}]", self.no_label)
            } else {
                self.no_label.clone()
            };
            format!("{yes} confirm     {no} cancel")
        }
    }

    fn compact_prompt(&self) -> String {
        let yes = if self.selected {
            format!("[{}]", self.yes_label)
        } else {
            self.yes_label.clone()
        };
        let no = if !self.selected {
            format!("[{}]", self.no_label)
        } else {
            self.no_label.clone()
        };
        format!("{yes} / {no}")
    }

    fn style_line(&self, line: &str) -> String {
        let mut style = Style::new().fg(self.fg).bold();
        if let Some(bg) = self.bg {
            style = style.bg(bg);
        }
        style.render(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::visible_len;
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
        }
    }

    #[test]
    fn default_selects_yes() {
        let confirm = Confirm::new("Delete?");
        assert!(confirm.selected);
    }

    #[test]
    fn enter_confirms_when_yes_selected() {
        let mut confirm = Confirm::new("Delete?");
        let msg = confirm.handle_key(&key(KeyCode::Enter));
        assert_eq!(msg, Some(ConfirmMsg::Confirmed));
    }

    #[test]
    fn enter_cancels_when_no_selected() {
        let mut confirm = Confirm::new("Delete?");
        confirm.handle_key(&key(KeyCode::Right));
        let msg = confirm.handle_key(&key(KeyCode::Enter));
        assert_eq!(msg, Some(ConfirmMsg::Cancelled));
    }

    #[test]
    fn y_key_confirms() {
        let mut confirm = Confirm::new("Sure?");
        let msg = confirm.handle_key(&key(KeyCode::Char('y')));
        assert_eq!(msg, Some(ConfirmMsg::Confirmed));
    }

    #[test]
    fn n_key_cancels() {
        let mut confirm = Confirm::new("Sure?");
        let msg = confirm.handle_key(&key(KeyCode::Char('n')));
        assert_eq!(msg, Some(ConfirmMsg::Cancelled));
    }

    #[test]
    fn esc_cancels() {
        let mut confirm = Confirm::new("Sure?");
        let msg = confirm.handle_key(&key(KeyCode::Esc));
        assert_eq!(msg, Some(ConfirmMsg::Cancelled));
    }

    #[test]
    fn tab_toggles_selection() {
        let mut confirm = Confirm::new("Sure?");
        assert!(confirm.selected);
        confirm.handle_key(&key(KeyCode::Tab));
        assert!(!confirm.selected);
        confirm.handle_key(&key(KeyCode::Tab));
        assert!(confirm.selected);
    }

    #[test]
    fn custom_labels() {
        let confirm = Confirm::new("Proceed?").with_labels("Continue", "Abort");
        assert_eq!(confirm.yes_label, "Continue");
        assert_eq!(confirm.no_label, "Abort");
    }

    #[test]
    fn box_view_centers_and_truncates_to_screen_width() {
        let rendered = Confirm::new("Terminate process with a long label?")
            .title("Terminate process?")
            .max_width(24)
            .box_view(32);
        let plain = crate::style::strip_ansi(&rendered);

        assert!(plain.contains("Terminate process?"));
        for line in rendered.lines() {
            assert!(visible_len(line) <= 32, "{line:?}");
        }
    }

    #[test]
    fn oversized_max_width_is_clamped() {
        let confirm = Confirm::new("Proceed?").max_width(usize::MAX);
        let rendered = confirm.box_view(16);

        assert_eq!(confirm.max_width, MAX_CONFIRM_WIDTH);
        assert!(rendered.lines().all(|line| visible_len(line) <= 16));
    }

    #[test]
    fn danger_view_fills_requested_screen() {
        let rendered = Confirm::new("PID 42")
            .title("FORCE-KILL THIS PROCESS?")
            .with_labels("Y", "N")
            .danger()
            .view(48, 10);

        assert_eq!(rendered.lines().count(), 10);
        assert!(rendered.contains("\x1b["));
        for line in rendered.lines() {
            assert_eq!(visible_len(line), 48, "{line:?}");
        }
    }

    #[test]
    fn line_view_is_width_bounded_and_styled() {
        let rendered = Confirm::new("Delete knowledge note?")
            .with_labels("Delete", "Cancel")
            .hint("Enter/y | n/Esc")
            .danger()
            .line(48);
        let plain = crate::style::strip_ansi(&rendered);

        assert_eq!(visible_len(&rendered), 48);
        assert!(plain.contains("Delete"), "{plain}");
        assert!(plain.contains("Enter/y"), "{plain}");
        assert!(
            rendered.contains("\x1b["),
            "inline confirm should carry styling"
        );
    }

    #[test]
    fn line_view_handles_tiny_widths() {
        assert_eq!(Confirm::new("Delete?").line(0), "");
        assert_eq!(visible_len(&Confirm::new("Delete?").line(1)), 1);
    }

    #[test]
    fn selected_no_is_visible_in_prompt() {
        let rendered = Confirm::new("Continue?").selected(false).box_view(40);
        let plain = crate::style::strip_ansi(&rendered);

        assert!(plain.contains("[No] cancel"));
    }

    #[test]
    fn element_with_width_zero_returns_empty_column() {
        let el: Element<()> = Confirm::new("Continue?").element_with_width(0);

        let Element::Box(column) = el else {
            panic!("expected column");
        };
        assert!(column.children.is_empty());
    }

    #[test]
    fn element_with_width_wraps_long_message() {
        let el: Element<()> = Confirm::new(
            "Delete the selected workspace artifact and all generated runtime outputs?",
        )
        .title("Confirm destructive action")
        .hint("Enter to confirm · Esc to cancel")
        .element_with_width(24);

        let Element::Box(column) = el else {
            panic!("expected column");
        };
        let texts = text_contents(&column.children);

        assert!(texts.iter().any(|text| text.contains("Confirm")));
        assert!(texts.iter().any(|text| text.contains("Delete")));
        assert!(texts.iter().any(|text| text.contains("runtime")));
        assert!(texts.iter().all(|text| visible_len(text) <= 20));
    }

    #[test]
    fn element_with_width_truncates_long_button_labels() {
        let el: Element<()> = Confirm::new("Proceed?")
            .with_labels("Absolutely continue", "No, abort everything")
            .element_with_width(18);

        let Element::Box(column) = el else {
            panic!("expected column");
        };
        let Element::Box(buttons) = &column.children[1] else {
            panic!("expected button row");
        };

        for child in &buttons.children {
            let Element::Text(text) = child else {
                panic!("expected button text");
            };
            assert!(visible_len(&text.content) <= 7, "{:?}", text.content);
        }
    }

    fn text_contents<Msg>(elements: &[Element<Msg>]) -> Vec<&str> {
        let mut out = Vec::new();
        for element in elements {
            collect_text_contents(element, &mut out);
        }
        out
    }

    fn collect_text_contents<'a, Msg>(element: &'a Element<Msg>, out: &mut Vec<&'a str>) {
        match element {
            Element::Text(text) => out.push(text.content.as_str()),
            Element::Box(box_element) => {
                for child in &box_element.children {
                    collect_text_contents(child, out);
                }
            }
            Element::Spacer | Element::_Phantom(_) => {}
        }
    }
}
