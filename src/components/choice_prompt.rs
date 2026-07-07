use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::event::{KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use crate::interaction::{Activatable, Selectable};
use crate::style::{fit_visible, truncate_visible, visible_len, Color, Style};
use crate::theme::{Theme, ThemeRole};
use crossterm::event::KeyCode;

const MAX_CHOICE_PROMPT_INDENT: usize = u16::MAX as usize;

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
        self.indent = indent.min(MAX_CHOICE_PROMPT_INDENT);
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

    /// Apply semantic colors from a theme while preserving choices and layout.
    pub fn with_theme(mut self, theme: &Theme) -> Self {
        self.title_color = theme.color(ThemeRole::Primary);
        self.text_color = theme.color(ThemeRole::Foreground);
        self.muted_color = theme.color(ThemeRole::Muted);
        self.danger_color = theme.color(ThemeRole::Error);
        self.selected_fg = theme.color(ThemeRole::Foreground);
        self.selected_bg = theme.color(ThemeRole::Highlight);
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
        self.normalized_selected()
    }

    pub fn selected_choice(&self) -> Option<&ChoicePromptItem> {
        self.choices.get(self.normalized_selected())
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> Option<ChoicePromptMsg> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.normalized_selected().saturating_sub(1);
                None
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => {
                let selected = self.normalized_selected();
                if selected.saturating_add(1) < self.choices.len() {
                    self.selected = selected + 1;
                } else {
                    self.selected = selected;
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
        let local_row = super::relative_mouse_row(mouse.row, self.y_offset)?;
        if local_row >= self.row_count() {
            return None;
        }

        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.selected = self.normalized_selected().saturating_sub(1);
                None
            }
            MouseEventKind::ScrollDown => {
                let selected = self.normalized_selected();
                if selected.saturating_add(1) < self.choices.len() {
                    self.selected = selected + 1;
                } else {
                    self.selected = selected;
                }
                None
            }
            MouseEventKind::Down(MouseButton::Left) => {
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
        self.lines(width, height).join("\n")
    }

    /// Render bounded prompt rows without joining them into a single string.
    pub fn lines(&self, width: u16, height: usize) -> Vec<String> {
        let width = width as usize;
        if width == 0 || height == 0 {
            return Vec::new();
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
            .collect()
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(self.element_children()),
        )
    }

    pub fn element_with_height<Msg>(&self, height: usize) -> Element<Msg> {
        let mut children = self.element_children();
        children.truncate(height);
        if self.fill_height {
            while children.len() < height {
                children.push(Element::Text(TextElement::new("")));
            }
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(children),
        )
    }

    fn element_children<Msg>(&self) -> Vec<Element<Msg>> {
        let mut children = Vec::new();
        if !self.title.is_empty() {
            children.push(Element::Text(
                TextElement::new(self.title.as_str())
                    .fg(self.title_color)
                    .bold(),
            ));
        }

        let selected = self.normalized_selected();
        for (index, choice) in self.choices.iter().enumerate() {
            let text = self.plain_choice_line(index, None);
            let mut element = TextElement::new(text);
            if index == selected {
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

        children
    }

    fn render_lines(&self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        if !self.title.is_empty() {
            let raw = format!("{}{}", " ".repeat(self.indent_for_width(width)), self.title);
            lines.push(Style::new().fg(self.title_color).bold().render(&raw));
        }

        let selected = self.normalized_selected();
        for (index, choice) in self.choices.iter().enumerate() {
            let raw = fit_visible(&self.plain_choice_line(index, Some(width)), width);
            if index == selected {
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
            let raw = format!("{}{}", " ".repeat(self.indent_for_width(width)), hint);
            lines.push(Style::new().fg(self.muted_color).render(&raw));
        }

        lines
    }

    fn plain_choice_line(&self, index: usize, width: Option<usize>) -> String {
        let Some(choice) = self.choices.get(index) else {
            return String::new();
        };
        let marker = if index == self.normalized_selected() {
            self.marker.as_str()
        } else {
            " "
        };
        let shortcut = self.choice_label(index, choice);
        let indent = width.map_or_else(
            || self.indent_for_element(),
            |width| self.choice_indent_for_width(marker, &shortcut, width),
        );
        let prefix = self.choice_prefix(marker, &shortcut, indent);
        let suffix = choice
            .description
            .as_deref()
            .filter(|description| !description.is_empty())
            .map(|description| format!("  {description}"))
            .unwrap_or_default();
        let available = width
            .map(|width| width.saturating_sub(visible_len(&prefix)))
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
            Some(ChoicePromptMsg::Selected(self.normalized_selected()))
        }
    }

    fn choice_start_row(&self) -> usize {
        usize::from(!self.title.is_empty())
    }

    fn row_count(&self) -> usize {
        self.choice_start_row()
            + self.choices.len()
            + usize::from(self.hint.as_deref().is_some_and(|hint| !hint.is_empty()))
    }

    fn clamp_selected(&mut self) {
        self.selected = self.selected.min(self.choices.len().saturating_sub(1));
    }

    fn normalized_selected(&self) -> usize {
        self.selected.min(self.choices.len().saturating_sub(1))
    }

    fn indent_for_width(&self, width: usize) -> usize {
        self.indent.min(width).min(MAX_CHOICE_PROMPT_INDENT)
    }

    fn choice_indent_for_width(&self, marker: &str, shortcut: &str, width: usize) -> usize {
        self.indent
            .min(width.saturating_sub(self.choice_prefix_width(marker, shortcut)))
            .min(MAX_CHOICE_PROMPT_INDENT)
    }

    fn choice_prefix(&self, marker: &str, shortcut: &str, indent: usize) -> String {
        if shortcut.is_empty() {
            format!("{}{} ", " ".repeat(indent), marker)
        } else {
            format!("{}{} {shortcut} ", " ".repeat(indent), marker)
        }
    }

    fn choice_prefix_width(&self, marker: &str, shortcut: &str) -> usize {
        if shortcut.is_empty() {
            visible_len(marker).saturating_add(1)
        } else {
            visible_len(marker)
                .saturating_add(1)
                .saturating_add(visible_len(shortcut))
                .saturating_add(1)
        }
    }

    fn indent_for_element(&self) -> usize {
        self.indent.min(MAX_CHOICE_PROMPT_INDENT)
    }
}

impl Default for ChoicePrompt {
    fn default() -> Self {
        Self::new("", Vec::new())
    }
}

impl Selectable for ChoicePrompt {
    fn item_count(&self) -> usize {
        self.choices.len()
    }

    fn selected_index(&self) -> Option<usize> {
        (!self.choices.is_empty()).then(|| self.normalized_selected())
    }

    fn select_index(&mut self, index: usize) {
        self.selected = index;
        self.clamp_selected();
    }
}

impl Activatable for ChoicePrompt {
    fn is_item_disabled(&self, _index: usize) -> bool {
        false
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
    fn with_theme_applies_semantic_colors() {
        let theme = Theme::tokyo_night();
        let prompt = ChoicePrompt::approval("Allow?").with_theme(&theme);

        assert_eq!(prompt.title_color, theme.color(ThemeRole::Primary));
        assert_eq!(prompt.text_color, theme.color(ThemeRole::Foreground));
        assert_eq!(prompt.muted_color, theme.color(ThemeRole::Muted));
        assert_eq!(prompt.danger_color, theme.color(ThemeRole::Error));
        assert_eq!(prompt.selected_bg, theme.color(ThemeRole::Highlight));
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
    fn stale_selection_is_normalized_for_input_and_rendering() {
        let mut prompt = ChoicePrompt::approval("Allow edit?");
        prompt.selected = usize::MAX;

        assert_eq!(prompt.selected_index(), 2);
        assert_eq!(
            prompt.selected_choice().map(ChoicePromptItem::label),
            Some("No")
        );
        assert_eq!(
            prompt.handle_key(&key(KeyCode::Enter)),
            Some(ChoicePromptMsg::Selected(2))
        );

        let rendered = strip_ansi(&prompt.view(40, 5));
        assert!(rendered.contains("❯ 3. No"));

        let Element::Box(column) = prompt.element::<()>() else {
            panic!("expected column element");
        };
        let Element::Text(choice) = &column.children[3] else {
            panic!("expected choice text");
        };
        assert_eq!(choice.content, "  ❯ 3. No");

        assert_eq!(prompt.handle_key(&key(KeyCode::Down)), None);
        assert_eq!(prompt.selected_index(), 2);
        assert_eq!(prompt.handle_key(&key(KeyCode::Up)), None);
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
    fn mouse_click_above_offset_is_ignored() {
        let mut prompt = ChoicePrompt::approval("Allow edit?");
        prompt.set_y_offset(4);

        let msg = prompt.handle_mouse(&MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 0,
            row: 3,
            modifiers: KeyModifiers::NONE,
        });

        assert_eq!(msg, None);
        assert_eq!(prompt.selected_index(), 0);
    }

    #[test]
    fn mouse_wheel_updates_selected_choice() {
        let mut prompt = ChoicePrompt::approval("Allow edit?");
        prompt.set_y_offset(4);

        let down = prompt.handle_mouse(&MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 0,
            row: 5,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(down, None);
        assert_eq!(prompt.selected_index(), 1);

        let up = prompt.handle_mouse(&MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 0,
            row: 5,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(up, None);
        assert_eq!(prompt.selected_index(), 0);
    }

    #[test]
    fn mouse_wheel_below_prompt_is_ignored() {
        let mut prompt = ChoicePrompt::approval("Allow edit?");
        prompt.set_y_offset(4);

        let msg = prompt.handle_mouse(&MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 0,
            row: 9,
            modifiers: KeyModifiers::NONE,
        });

        assert_eq!(msg, None);
        assert_eq!(prompt.selected_index(), 0);
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
    fn lines_return_bounded_rows_without_joining() {
        let lines = ChoicePrompt::approval("Allow a very long command label?")
            .selected(1)
            .lines(20, 5);
        let plain = lines
            .iter()
            .map(|line| strip_ansi(line))
            .collect::<Vec<_>>();

        assert_eq!(lines.len(), 5);
        assert!(plain[0].contains("Allow"), "{plain:?}");
        assert!(plain[2].contains("2. Yes"), "{plain:?}");
        assert!(
            lines.iter().all(|line| visible_len(line) == 20),
            "{plain:?}"
        );
        assert!(lines[2].contains("\x1b["), "selected row should be styled");
    }

    #[test]
    fn oversized_indent_is_clamped_to_render_width() {
        let prompt = ChoicePrompt::new(
            "Allow?",
            vec![ChoicePromptItem::new("Run command").shortcut('r')],
        )
        .hint("Enter")
        .indent(usize::MAX);
        let rendered = prompt.view(8, 4);
        let choice = prompt.plain_choice_line(0, Some(8));

        assert_eq!(prompt.indent, MAX_CHOICE_PROMPT_INDENT);
        assert_eq!(prompt.indent_for_width(8), 8);
        assert_eq!(prompt.choice_indent_for_width("❯", "1.", 8), 3);
        assert_eq!(visible_len(&choice), 8);
        assert!(rendered.lines().all(|line| visible_len(line) == 8));

        let Element::Box(column) = prompt.element::<()>() else {
            panic!("expected column element");
        };
        let Element::Text(choice) = &column.children[1] else {
            panic!("expected choice text");
        };
        assert_eq!(
            visible_len(&choice.content),
            MAX_CHOICE_PROMPT_INDENT + visible_len("❯ 1. Run command")
        );
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

    #[test]
    fn element_with_height_zero_returns_empty_column() {
        let el: Element<()> = ChoicePrompt::approval("Allow edit?").element_with_height(0);

        let Element::Box(column) = el else {
            panic!("expected Box");
        };
        assert_eq!(column.style.flex_direction, FlexDirection::Column);
        assert!(column.children.is_empty());
    }

    #[test]
    fn element_with_height_limits_rows() {
        let el: Element<()> = ChoicePrompt::approval("Allow edit?").element_with_height(2);

        let Element::Box(column) = el else {
            panic!("expected Box");
        };
        assert_eq!(column.children.len(), 2);
        assert_eq!(column.children[0].text_content(), Some("Allow edit?"));
        assert!(column.children[1]
            .text_content()
            .is_some_and(|text| text.contains("Yes")));
        assert!(!column
            .children
            .iter()
            .any(|child| child.text_content().is_some_and(|text| text.contains("No"))));
    }

    #[test]
    fn element_with_height_fill_height_pads_empty_rows() {
        let el: Element<()> = ChoicePrompt::new("Allow?", Vec::new())
            .fill_height(true)
            .element_with_height(3);

        let Element::Box(column) = el else {
            panic!("expected Box");
        };
        assert_eq!(column.children.len(), 3);
        assert_eq!(column.children[0].text_content(), Some("Allow?"));
        assert_eq!(column.children[1].text_content(), Some(""));
        assert_eq!(column.children[2].text_content(), Some(""));
    }
}
