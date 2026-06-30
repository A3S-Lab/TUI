use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, wrap_words, Color, Style};

/// Side-note panel for background questions, side-channel answers, and compact
/// transient notes.
///
/// This extracts the CLI `/btw` pattern: a colored title row, a bold question,
/// and a capped answer body with a loading fallback.
#[derive(Debug, Clone)]
pub struct SideNotePanel {
    title: String,
    question: Option<String>,
    answer: Option<String>,
    loading_text: String,
    footer: Option<String>,
    max_body_lines: usize,
    fill_height: bool,
    indent: usize,
    title_color: Color,
    question_color: Color,
    answer_color: Color,
    muted_color: Color,
}

impl SideNotePanel {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            question: None,
            answer: None,
            loading_text: "thinking...".to_string(),
            footer: None,
            max_body_lines: 12,
            fill_height: false,
            indent: 2,
            title_color: Color::Yellow,
            question_color: Color::Yellow,
            answer_color: Color::Yellow,
            muted_color: Color::BrightBlack,
        }
    }

    pub fn question(mut self, question: impl Into<String>) -> Self {
        let question = question.into();
        if !question.is_empty() {
            self.question = Some(question);
        }
        self
    }

    pub fn answer(mut self, answer: impl Into<String>) -> Self {
        let answer = answer.into();
        if !answer.is_empty() {
            self.answer = Some(answer);
        }
        self
    }

    pub fn loading_text(mut self, text: impl Into<String>) -> Self {
        self.loading_text = text.into();
        self
    }

    pub fn footer(mut self, footer: impl Into<String>) -> Self {
        let footer = footer.into();
        if !footer.is_empty() {
            self.footer = Some(footer);
        }
        self
    }

    pub fn max_body_lines(mut self, max: usize) -> Self {
        self.max_body_lines = max.max(1);
        self
    }

    pub fn fill_height(mut self, enabled: bool) -> Self {
        self.fill_height = enabled;
        self
    }

    pub fn indent(mut self, indent: usize) -> Self {
        self.indent = indent;
        self
    }

    pub fn title_color(mut self, color: Color) -> Self {
        self.title_color = color;
        self
    }

    pub fn question_color(mut self, color: Color) -> Self {
        self.question_color = color;
        self
    }

    pub fn answer_color(mut self, color: Color) -> Self {
        self.answer_color = color;
        self
    }

    pub fn muted_color(mut self, color: Color) -> Self {
        self.muted_color = color;
        self
    }

    pub fn title_value(&self) -> &str {
        &self.title
    }

    pub fn question_value(&self) -> Option<&str> {
        self.question.as_deref()
    }

    pub fn answer_value(&self) -> Option<&str> {
        self.answer.as_deref()
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

    pub fn element<Msg>(&self, width: u16) -> Element<Msg> {
        let width = width as usize;
        let mut children = Vec::new();
        children.push(Element::Text(
            TextElement::new(self.indented(&self.title))
                .fg(self.title_color)
                .bold(),
        ));

        if let Some(question) = self.question.as_deref() {
            for row in self.wrap_prefixed("Q: ", question, width) {
                children.push(Element::Text(
                    TextElement::new(row).fg(self.question_color).bold(),
                ));
            }
        }

        let body = self.answer.as_deref().unwrap_or(&self.loading_text);
        for row in self
            .wrap_prefixed("", body, width)
            .into_iter()
            .take(self.max_body_lines)
        {
            children.push(Element::Text(TextElement::new(row).fg(self.answer_color)));
        }

        if let Some(footer) = self.footer.as_deref() {
            children.push(Element::Text(
                TextElement::new(self.indented(footer)).fg(self.muted_color),
            ));
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(children),
        )
    }

    fn render_lines(&self, width: usize) -> Vec<String> {
        let mut lines = vec![Style::new()
            .fg(self.title_color)
            .bold()
            .render(&fit_visible(&self.indented(&self.title), width))];

        if let Some(question) = self.question.as_deref() {
            for row in self.wrap_prefixed("Q: ", question, width) {
                lines.push(
                    Style::new()
                        .fg(self.question_color)
                        .bold()
                        .render(&fit_visible(&row, width)),
                );
            }
        }

        let body = self.answer.as_deref().unwrap_or(&self.loading_text);
        for row in self
            .wrap_prefixed("", body, width)
            .into_iter()
            .take(self.max_body_lines)
        {
            lines.push(
                Style::new()
                    .fg(self.answer_color)
                    .render(&fit_visible(&row, width)),
            );
        }

        if let Some(footer) = self.footer.as_deref() {
            lines.push(
                Style::new()
                    .fg(self.muted_color)
                    .render(&fit_visible(&self.indented(footer), width)),
            );
        }

        lines
    }

    fn wrap_prefixed(&self, prefix: &str, text: &str, width: usize) -> Vec<String> {
        let indent = " ".repeat(self.indent);
        let first_prefix = format!("{indent}{prefix}");
        let continuation_prefix = " ".repeat(first_prefix.chars().count());
        let first_width = width
            .saturating_sub(crate::style::visible_len(&first_prefix))
            .max(1);
        let continuation_width = width
            .saturating_sub(crate::style::visible_len(&continuation_prefix))
            .max(1);
        let mut rows = Vec::new();
        for line in text.lines() {
            let wrapped = wrap_words(line, first_width);
            if wrapped.is_empty() {
                rows.push(first_prefix.clone());
                continue;
            }
            for (index, part) in wrapped.into_iter().enumerate() {
                if index == 0 {
                    rows.push(format!("{first_prefix}{part}"));
                } else {
                    for continuation in wrap_words(&part, continuation_width) {
                        rows.push(format!("{continuation_prefix}{continuation}"));
                    }
                }
            }
        }
        if rows.is_empty() {
            rows.push(first_prefix);
        }
        rows
    }

    fn indented(&self, value: &str) -> String {
        format!("{}{}", " ".repeat(self.indent), value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, visible_len};

    fn sample() -> SideNotePanel {
        SideNotePanel::new("↘ by the way · Esc to close")
            .question("Can we reuse this panel for background side questions?")
            .answer("Yes. It renders a compact answer body and keeps rows width-safe.")
            .footer("side-channel")
    }

    #[test]
    fn renders_title_question_answer_and_footer() {
        let rendered = sample().view(48, 8);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("by the way"));
        assert!(plain.contains("Q: Can we reuse"));
        assert!(plain.contains("compact answer"));
        assert!(plain.contains("side-channel"));
        for line in rendered.lines() {
            assert_eq!(visible_len(line), 48, "{line:?}");
        }
    }

    #[test]
    fn loading_state_uses_fallback_answer() {
        let rendered = SideNotePanel::new("↘ by the way")
            .question("still working?")
            .loading_text("thinking...")
            .view(32, 4);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("still working"));
        assert!(plain.contains("thinking"));
    }

    #[test]
    fn caps_answer_lines() {
        let rendered = SideNotePanel::new("note")
            .answer("one\ntwo\nthree\nfour")
            .max_body_lines(2)
            .view(24, 5);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("one"));
        assert!(plain.contains("two"));
        assert!(!plain.contains("three"));
    }

    #[test]
    fn cjk_text_wraps_by_display_width() {
        let rendered = SideNotePanel::new("提示")
            .question("中文问题 with a long suffix")
            .answer("中文答案 with another long suffix")
            .view(20, 8);

        assert!(strip_ansi(&rendered).contains("中文"));
        for line in rendered.lines() {
            assert_eq!(visible_len(line), 20, "{line:?}");
        }
    }

    #[test]
    fn fill_height_pads_remaining_rows() {
        let rendered = sample().fill_height(true).view(32, 10);

        assert_eq!(rendered.lines().count(), 10);
        for line in rendered.lines() {
            assert_eq!(visible_len(line), 32, "{line:?}");
        }
    }

    #[test]
    fn zero_size_renders_empty_string() {
        assert_eq!(sample().view(0, 4), "");
        assert_eq!(sample().view(40, 0), "");
    }

    #[test]
    fn element_produces_column() {
        let el: Element<()> = sample().element(48);

        match el {
            Element::Box(column) => {
                assert_eq!(column.style.flex_direction, FlexDirection::Column);
                assert!(!column.children.is_empty());
            }
            _ => panic!("expected Box"),
        }
    }
}
