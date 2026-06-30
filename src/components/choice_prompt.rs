use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::event::{KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use crate::style::{fit_visible, truncate_visible, Color, Style};
use crossterm::event::KeyCode;

/// One selectable action in a [`ChoicePrompt`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoicePromptItem {
    label: String,
    shortcut: Option<char>,
    description: Option<String>,
    danger: bool,
}

impl ChoicePromptItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            shortcut: None,
            description: None,
            danger: false,
        }
    }

    pub fn shortcut(mut self, shortcut: char) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn shortcut_value(&self) -> Option<char> {
        self.shortcut
    }

    pub fn description_value(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn is_danger(&self) -> bool {
        self.danger
    }
}

/// Message returned by [`ChoicePrompt`] input handlers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChoicePromptMsg {
    Selected(usize),
    Cancelled,
}

/// A compact numbered action picker for approvals and command choices.
///
/// This component extracts the common terminal pattern of a prompt line,
/// several selectable options, and a hint row. It supports arrow-key movement,
/// Enter, Escape, number shortcuts, optional mnemonic shortcuts, line-based
/// rendering, and Element-tree rendering.
#[derive(Debug, Clone)]
pub struct ChoicePrompt {
    title: String,
    choices: Vec<ChoicePromptItem>,
    selected: usize,
    hint: Option<String>,
    indent: usize,
    marker: String,
    number_shortcuts: bool,
    fill_height: bool,
    y_offset: u16,
    title_color: Color,
    text_color: Color,
    muted_color: Color,
    danger_color: Color,
    selected_fg: Color,
    selected_bg: Color,
}

impl ChoicePrompt {
    pub fn new(title: impl Into<String>, choices: Vec<ChoicePromptItem>) -> Self {
        Self {
            title: title.into(),
            choices,
            selected: 0,
            hint: None,
            indent: 2,
            marker: "❯".to_string(),
            number_shortcuts: true,
            fill_height: false,
            y_offset: 0,
            title_color: Color::Yellow,
            text_color: Color::White,
            muted_color: Color::BrightBlack,
            danger_color: Color::Red,
            selected_fg: Color::BrightWhite,
            selected_bg: Color::Cyan,
        }
    }

    pub fn approval(title: impl Into<String>) -> Self {
        Self::new(
            title,
            vec![
                ChoicePromptItem::new("Yes").shortcut('y'),
                ChoicePromptItem::new("Yes, and don't ask again").shortcut('a'),
                ChoicePromptItem::new("No").shortcut('n').danger(),
            ],
        )
        .hint("Enter select · ↑/↓ · 1-3 · Esc")
    }

    pub fn choice(mut self, choice: ChoicePromptItem) -> Self {
        self.choices.push(choice);
        self.clamp_selected();
        self
    }

    pub fn add_choice(&mut self, choice: ChoicePromptItem) {
        self.choices.push(choice);
        self.clamp_selected();
    }

    pub fn selected(mut self, selected: usize) -> Self {
        self.selected = selected;
        self.clamp_selected();
        self
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn indent(mut self, indent: usize) -> Self {
        self.indent = indent;
        self
    }

    pub fn marker(mut self, marker: impl Into<String>) -> Self {
        self.marker = marker.into();
        self
    }

    pub fn number_shortcuts(mut self, enabled: bool) -> Self {
        self.number_shortcuts = enabled;
        self
    }

    pub fn fill_height(mut self, enabled: bool) -> Self {
        self.fill_height = enabled;
        self
    }

    pub fn title_color(mut self, color: Color) -> Self {
        self.title_color = color;
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    pub fn muted_color(mut self, color: Color) -> Self {
        self.muted_color = color;
        self
    }

    pub fn danger_color(mut self, color: Color) -> Self {
        self.danger_color = color;
        self
    }

    pub fn selected_colors(mut self, fg: Color, bg: Color) -> Self {
        self.selected_fg = fg;
        self.selected_bg = bg;
        self
    }

    pub fn set_y_offset(&mut self, y: u16) {
        self.y_offset = y;
    }

    pub fn title_value(&self) -> &str {
        &self.title
    }

    pub fn choices(&self) -> &[ChoicePromptItem] {
        &self.choices
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub fn selected_choice(&self) -> Option<&ChoicePromptItem> {
        self.choices.get(self.selected)
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> Option<ChoicePromptMsg> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.selected.saturating_sub(1);
                None
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => {
                if self.selected + 1 < self.choices.len() {
                    self.selected += 1;
                }
                None
            }
            KeyCode::Home => {
                self.selected = 0;
                None
            }
            KeyCode::End => {
                self.selected = self.choices.len().saturating_sub(1);
                None
            }
            KeyCode::Enter => self.selected_msg(),
            KeyCode::Esc => Some(ChoicePromptMsg::Cancelled),
            KeyCode::Char(c) => self.shortcut_msg(c),
            _ => None,
        }
    }

    pub fn handle_mouse(&mut self, mouse: &MouseEvent) -> Option<ChoicePromptMsg> {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let local_row = mouse.row.saturating_sub(self.y_offset) as usize;
                let choice_row = local_row.checked_sub(self.choice_start_row())?;
                if choice_row < self.choices.len() {
                    self.selected = choice_row;
                    Some(ChoicePromptMsg::Selected(choice_row))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn view(&self, width: u16, height: usize) -> String {
        let width = width as usize;
        if width == 0 || height == 0 {
            return String::new();
        }

        let mut lines = self.render_lines(width);
        lines.truncate(height);
        if self.fill_height {
            while lines.len() < height {
                lines.push(String::new());
            }
        }

        lines
            .into_iter()
            .map(|line| fit_visible(&line, width))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        let mut children = Vec::new();
        if !self.title.is_empty() {
            children.push(Element::Text(
                TextElement::new(self.title.as_str())
                    .fg(self.title_color)
                    .bold(),
            ));
        }

        for (index, choice) in self.choices.iter().enumerate() {
            let text = self.plain_choice_line(index, None);
            let mut element = TextElement::new(text);
            if index == self.selected {
                element = element.fg(self.selected_fg).bg(self.selected_bg).bold();
            } else if choice.danger {
                element = element.fg(self.danger_color);
            } else {
                element = element.fg(self.text_color);
            }
            children.push(Element::Text(element));
        }

        if let Some(hint) = self.hint.as_deref().filter(|hint| !hint.is_empty()) {
            children.push(Element::Text(TextElement::new(hint).fg(self.muted_color)));
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(children),
        )
    }

    fn render_lines(&self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        if !self.title.is_empty() {
            let raw = format!("{}{}", " ".repeat(self.indent), self.title);
            lines.push(Style::new().fg(self.title_color).bold().render(&raw));
        }

        for (index, choice) in self.choices.iter().enumerate() {
            let raw = fit_visible(&self.plain_choice_line(index, Some(width)), width);
            if index == self.selected {
                lines.push(
                    Style::new()
                        .fg(self.selected_fg)
                        .bg(self.selected_bg)
                        .render(&raw),
                );
            } else {
                let color = if choice.danger {
                    self.danger_color
                } else {
                    self.text_color
                };
                lines.push(Style::new().fg(color).render(&raw));
            }
        }

        if let Some(hint) = self.hint.as_deref().filter(|hint| !hint.is_empty()) {
            let raw = format!("{}{}", " ".repeat(self.indent), hint);
            lines.push(Style::new().fg(self.muted_color).render(&raw));
        }

        lines
    }

    fn plain_choice_line(&self, index: usize, width: Option<usize>) -> String {
        let Some(choice) = self.choices.get(index) else {
            return String::new();
        };
        let marker = if index == self.selected {
            self.marker.as_str()
        } else {
            " "
        };
        let shortcut = self.choice_label(index, choice);
        let prefix = if shortcut.is_empty() {
            format!("{}{} ", " ".repeat(self.indent), marker)
        } else {
            format!("{}{} {shortcut} ", " ".repeat(self.indent), marker)
        };
        let suffix = choice
            .description
            .as_deref()
            .filter(|description| !description.is_empty())
            .map(|description| format!("  {description}"))
            .unwrap_or_default();
        let available = width
            .map(|width| width.saturating_sub(crate::style::visible_len(&prefix)))
            .unwrap_or(usize::MAX);

        format!(
            "{prefix}{}",
            truncate_visible(&format!("{}{}", choice.label, suffix), available)
        )
    }

    fn choice_label(&self, index: usize, choice: &ChoicePromptItem) -> String {
        if self.number_shortcuts {
            number_shortcut_label(index)
                .map(|label| format!("{label}."))
                .unwrap_or_default()
        } else {
            choice
                .shortcut
                .map(|shortcut| format!("{shortcut}."))
                .unwrap_or_default()
        }
    }

    fn shortcut_msg(&mut self, c: char) -> Option<ChoicePromptMsg> {
        if self.number_shortcuts {
            if let Some(index) = number_shortcut_index(c) {
                if index < self.choices.len() {
                    self.selected = index;
                    return Some(ChoicePromptMsg::Selected(index));
                }
            }
        }

        let matched = self.choices.iter().position(|choice| {
            choice
                .shortcut
                .is_some_and(|shortcut| shortcut.eq_ignore_ascii_case(&c))
        })?;
        self.selected = matched;
        Some(ChoicePromptMsg::Selected(matched))
    }

    fn selected_msg(&self) -> Option<ChoicePromptMsg> {
        if self.choices.is_empty() {
            None
        } else {
            Some(ChoicePromptMsg::Selected(self.selected))
        }
    }

    fn choice_start_row(&self) -> usize {
        usize::from(!self.title.is_empty())
    }

    fn clamp_selected(&mut self) {
        self.selected = self.selected.min(self.choices.len().saturating_sub(1));
    }
}

impl Default for ChoicePrompt {
    fn default() -> Self {
        Self::new("", Vec::new())
    }
}

fn number_shortcut_index(c: char) -> Option<usize> {
    match c {
        '1'..='9' => Some((c as u8 - b'1') as usize),
        '0' => Some(9),
        _ => None,
    }
}

fn number_shortcut_label(index: usize) -> Option<char> {
    match index {
        0..=8 => Some((b'1' + index as u8) as char),
        9 => Some('0'),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, visible_len};
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
        }
    }

    #[test]
    fn approval_prompt_has_three_standard_choices() {
        let prompt = ChoicePrompt::approval("Allow shell_command?");

        assert_eq!(prompt.choices().len(), 3);
        assert_eq!(prompt.choices()[1].label(), "Yes, and don't ask again");
        assert!(prompt.choices()[2].is_danger());
    }

    #[test]
    fn arrow_keys_move_selection_within_bounds() {
        let mut prompt = ChoicePrompt::approval("Allow edit?");

        prompt.handle_key(&key(KeyCode::Down));
        assert_eq!(prompt.selected_index(), 1);
        prompt.handle_key(&key(KeyCode::Down));
        prompt.handle_key(&key(KeyCode::Down));
        assert_eq!(prompt.selected_index(), 2);
        prompt.handle_key(&key(KeyCode::Up));
        assert_eq!(prompt.selected_index(), 1);
    }

    #[test]
    fn enter_selects_current_choice() {
        let mut prompt = ChoicePrompt::approval("Allow edit?").selected(1);

        assert_eq!(
            prompt.handle_key(&key(KeyCode::Enter)),
            Some(ChoicePromptMsg::Selected(1))
        );
    }

    #[test]
    fn number_shortcuts_select_by_index() {
        let mut prompt = ChoicePrompt::approval("Allow edit?");

        assert_eq!(
            prompt.handle_key(&key(KeyCode::Char('3'))),
            Some(ChoicePromptMsg::Selected(2))
        );
        assert_eq!(prompt.selected_index(), 2);
    }

    #[test]
    fn mnemonic_shortcuts_select_by_choice() {
        let mut prompt = ChoicePrompt::approval("Allow edit?");

        assert_eq!(
            prompt.handle_key(&key(KeyCode::Char('a'))),
            Some(ChoicePromptMsg::Selected(1))
        );
        assert_eq!(prompt.selected_index(), 1);
    }

    #[test]
    fn escape_cancels() {
        let mut prompt = ChoicePrompt::approval("Allow edit?");

        assert_eq!(
            prompt.handle_key(&key(KeyCode::Esc)),
            Some(ChoicePromptMsg::Cancelled)
        );
    }

    #[test]
    fn mouse_click_selects_choice_row() {
        let mut prompt = ChoicePrompt::approval("Allow edit?");
        prompt.set_y_offset(4);
        let msg = prompt.handle_mouse(&MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 0,
            row: 6,
            modifiers: KeyModifiers::NONE,
        });

        assert_eq!(msg, Some(ChoicePromptMsg::Selected(1)));
        assert_eq!(prompt.selected_index(), 1);
    }

    #[test]
    fn view_pads_and_truncates_to_width() {
        let prompt = ChoicePrompt::new(
            "允许执行命令?",
            vec![
                ChoicePromptItem::new("是").shortcut('y'),
                ChoicePromptItem::new("中文测试内容 with a long suffix").shortcut('n'),
            ],
        )
        .hint("Enter select")
        .fill_height(true);
        let rendered = prompt.view(18, 5);

        assert_eq!(rendered.lines().count(), 5);
        for line in rendered.lines() {
            assert_eq!(visible_len(line), 18, "{line:?}");
        }
        assert!(strip_ansi(&rendered).contains("允许执行命"));
    }

    #[test]
    fn element_produces_column_rows() {
        let el: Element<()> = ChoicePrompt::approval("Allow edit?").element();

        match el {
            Element::Box(column) => {
                assert_eq!(column.style.flex_direction, FlexDirection::Column);
                assert_eq!(column.children.len(), 5);
            }
            _ => panic!("expected Box"),
        }
    }
}
