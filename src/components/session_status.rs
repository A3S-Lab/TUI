use crate::components::Meter;
use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, Color, Style};
use crate::theme::{Theme, ThemeRole};

const MAX_SESSION_STATUS_CHIP_LABEL_WIDTH: usize = u16::MAX as usize;
const MAX_SESSION_STATUS_MARGIN: usize = u16::MAX as usize;
const CONTEXT_METER_WIDTH: usize = 6;

/// Agent/session status row with workspace, model, context, and live chips.
///
/// This extracts the common CLI footer pattern: current workspace, git branch,
/// model/context window, context fill percentage, and short activity chips.
#[derive(Debug, Clone)]
pub struct SessionStatus {
    cwd: String,
    branch: Option<String>,
    model: Option<String>,
    context: Option<(usize, usize)>,
    output_tokens: Option<usize>,
    chips: Vec<SessionStatusChip>,
    margin: usize,
    accent_color: Color,
    branch_color: Color,
    text_color: Color,
    muted_color: Color,
    warning_color: Color,
    danger_color: Color,
}

impl SessionStatus {
    pub fn new(cwd: impl Into<String>) -> Self {
        Self {
            cwd: cwd.into(),
            branch: None,
            model: None,
            context: None,
            output_tokens: None,
            chips: Vec::new(),
            margin: 2,
            accent_color: Color::Cyan,
            branch_color: Color::Yellow,
            text_color: Color::White,
            muted_color: Color::BrightBlack,
            warning_color: Color::Yellow,
            danger_color: Color::Red,
        }
    }

    pub fn branch(mut self, branch: impl Into<String>) -> Self {
        let branch = branch.into();
        if !branch.is_empty() {
            self.branch = Some(branch);
        }
        self
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        let model = model.into();
        if !model.is_empty() {
            self.model = Some(model);
        }
        self
    }

    pub fn context(mut self, used_tokens: usize, limit_tokens: usize) -> Self {
        if limit_tokens > 0 {
            self.context = Some((used_tokens, limit_tokens));
        }
        self
    }

    pub fn output_tokens(mut self, output_tokens: usize) -> Self {
        self.output_tokens = Some(output_tokens);
        self
    }

    pub fn chip(mut self, glyph: impl Into<String>, label: impl Into<String>) -> Self {
        self.chips.push(SessionStatusChip::new(glyph, label));
        self
    }

    pub fn status_chip(mut self, chip: SessionStatusChip) -> Self {
        self.chips.push(chip);
        self
    }

    pub fn margin(mut self, margin: usize) -> Self {
        self.margin = margin.min(MAX_SESSION_STATUS_MARGIN);
        self
    }

    pub fn accent_color(mut self, color: Color) -> Self {
        self.accent_color = color;
        self
    }

    pub fn branch_color(mut self, color: Color) -> Self {
        self.branch_color = color;
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

    pub fn threshold_colors(mut self, warning: Color, danger: Color) -> Self {
        self.warning_color = warning;
        self.danger_color = danger;
        self
    }

    pub fn with_theme(mut self, theme: &Theme) -> Self {
        self.accent_color = theme.color(ThemeRole::Primary);
        self.branch_color = theme.color(ThemeRole::Secondary);
        self.text_color = theme.color(ThemeRole::Foreground);
        self.muted_color = theme.color(ThemeRole::Muted);
        self.warning_color = theme.color(ThemeRole::Warning);
        self.danger_color = theme.color(ThemeRole::Error);
        self
    }

    pub fn cwd_value(&self) -> &str {
        &self.cwd
    }

    pub fn branch_value(&self) -> Option<&str> {
        self.branch.as_deref()
    }

    pub fn model_value(&self) -> Option<&str> {
        self.model.as_deref()
    }

    pub fn chips_value(&self) -> &[SessionStatusChip] {
        &self.chips
    }

    pub fn view(&self, width: u16) -> String {
        let width = width as usize;
        if width == 0 {
            return String::new();
        }
        fit_visible(&self.render_raw(self.margin_for_width(width)), width)
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        let mut row = BoxElement::new().direction(FlexDirection::Row);
        row = row.child(Element::Text(TextElement::new(
            " ".repeat(self.margin_for_element()),
        )));
        row = row.child(Element::Text(
            TextElement::new(self.workspace_name())
                .fg(self.accent_color)
                .bold(),
        ));

        if let Some(branch) = self.branch.as_deref() {
            row = row
                .child(Element::Text(
                    TextElement::new(" git:(").fg(self.muted_color),
                ))
                .child(Element::Text(
                    TextElement::new(branch).fg(self.branch_color),
                ))
                .child(Element::Text(TextElement::new(")").fg(self.muted_color)));
        }

        if let Some(model) = self.model.as_deref() {
            row = row.child(Element::Text(TextElement::new("  ")));
            row = row.child(Element::Text(
                TextElement::new(short_model_name(model)).fg(self.text_color),
            ));
            if let Some((_, limit)) = self.context {
                row = row.child(Element::Text(
                    TextElement::new(format!(" ({} context)", context_window_label(limit)))
                        .fg(self.muted_color),
                ));
            }
        }

        if let Some((used, limit)) = self.context {
            row = row
                .child(Element::Text(TextElement::new(" ")))
                .child(Element::Text(
                    TextElement::new(format!("ctx:{}%", context_percent(used, limit)))
                        .fg(self.context_color()),
                ))
                .child(Element::Text(TextElement::new(" ")))
                .child(Element::Text(
                    TextElement::new(self.context_meter(used, limit).plain())
                        .fg(self.context_color()),
                ));
        } else if let Some(tokens) = self.output_tokens.filter(|tokens| *tokens > 0) {
            row = row.child(Element::Text(
                TextElement::new(format!(" {tokens} tok")).fg(self.muted_color),
            ));
        }

        for chip in &self.chips {
            let color = chip.color.unwrap_or(self.muted_color);
            row = row
                .child(Element::Text(TextElement::new("  ")))
                .child(Element::Text(
                    TextElement::new(chip.glyph.as_str()).fg(color),
                ))
                .child(Element::Text(TextElement::new(" ")))
                .child(Element::Text(
                    TextElement::new(chip.visible_label()).fg(color),
                ));
        }

        Element::Box(row)
    }

    fn render_raw(&self, margin: usize) -> String {
        let mut raw = format!(
            "{}{}",
            " ".repeat(margin),
            Style::new()
                .fg(self.accent_color)
                .bold()
                .render(&self.workspace_name())
        );

        if let Some(branch) = self.branch.as_deref() {
            raw.push_str(&format!(
                " {}{}{}",
                Style::new().fg(self.muted_color).render("git:("),
                Style::new().fg(self.branch_color).render(branch),
                Style::new().fg(self.muted_color).render(")")
            ));
        }

        if let Some(model) = self.model.as_deref() {
            raw.push_str(&format!(
                "  {}",
                Style::new()
                    .fg(self.text_color)
                    .render(&short_model_name(model))
            ));
            if let Some((_, limit)) = self.context {
                raw.push_str(&format!(
                    " {}",
                    Style::new()
                        .fg(self.muted_color)
                        .render(&format!("({} context)", context_window_label(limit)))
                ));
            }
        }

        if let Some((used, limit)) = self.context {
            raw.push_str(&format!(
                " {} {}",
                Style::new()
                    .fg(self.context_color())
                    .render(&format!("ctx:{}%", context_percent(used, limit))),
                self.context_meter(used, limit).view()
            ));
        } else if let Some(tokens) = self.output_tokens.filter(|tokens| *tokens > 0) {
            raw.push_str(&format!(
                " {}",
                Style::new()
                    .fg(self.muted_color)
                    .render(&format!("{tokens} tok"))
            ));
        }

        for chip in &self.chips {
            let color = chip.color.unwrap_or(self.muted_color);
            let label = chip.visible_label();
            raw.push_str(&format!(
                "  {} {}",
                Style::new().fg(color).render(&chip.glyph),
                Style::new().fg(color).render(&label)
            ));
        }

        raw
    }

    fn margin_for_width(&self, width: usize) -> usize {
        self.margin.min(width).min(MAX_SESSION_STATUS_MARGIN)
    }

    fn margin_for_element(&self) -> usize {
        self.margin.min(MAX_SESSION_STATUS_MARGIN)
    }

    fn workspace_name(&self) -> String {
        let trimmed = self.cwd.trim_end_matches(['/', '\\']);
        trimmed
            .rsplit(['/', '\\'])
            .next()
            .filter(|name| !name.is_empty())
            .unwrap_or(trimmed)
            .to_string()
    }

    fn context_color(&self) -> Color {
        let Some((used, limit)) = self.context else {
            return self.muted_color;
        };
        let pct = context_percent(used, limit);
        if pct >= 85 {
            self.danger_color
        } else if pct >= 70 {
            self.warning_color
        } else {
            self.muted_color
        }
    }

    fn context_meter(&self, used: usize, limit: usize) -> Meter {
        Meter::new(context_percent(used, limit) as f64)
            .width(CONTEXT_METER_WIDTH)
            .glyphs('▰', '▱')
            .show_value(false)
            .fg(self.context_color())
            .empty_fg(self.muted_color)
    }
}

impl Default for SessionStatus {
    fn default() -> Self {
        Self::new("")
    }
}

/// Short live status chip appended to a [`SessionStatus`] row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionStatusChip {
    glyph: String,
    label: String,
    color: Option<Color>,
    max_label_width: Option<usize>,
}

impl SessionStatusChip {
    pub fn new(glyph: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            glyph: glyph.into(),
            label: label.into(),
            color: None,
            max_label_width: None,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn max_label_width(mut self, width: usize) -> Self {
        self.max_label_width = Some(width.clamp(1, MAX_SESSION_STATUS_CHIP_LABEL_WIDTH));
        self
    }

    pub fn glyph(&self) -> &str {
        &self.glyph
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn color_value(&self) -> Option<Color> {
        self.color
    }

    fn visible_label(&self) -> String {
        self.max_label_width
            .map(|width| crate::style::truncate_visible(&self.label, width))
            .unwrap_or_else(|| self.label.clone())
    }
}

fn short_model_name(model: &str) -> String {
    model
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(model)
        .to_string()
}

fn context_window_label(limit: usize) -> String {
    if limit >= 1_000_000 {
        format!("{}M", limit / 1_000_000)
    } else if limit >= 1_000 {
        format!("{}k", limit / 1_000)
    } else {
        limit.to_string()
    }
}

fn context_percent(used: usize, limit: usize) -> usize {
    if limit == 0 || used == 0 {
        0
    } else if used >= limit {
        100
    } else {
        ((used as u128 * 100) / limit as u128) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, visible_len};

    #[test]
    fn renders_workspace_model_context_and_chips_at_fixed_width() {
        let rendered = SessionStatus::new("/Users/roylin/code/a3s")
            .branch("main")
            .model("openai/gpt-5")
            .context(90_000, 128_000)
            .chip("🎯", "ship tui")
            .status_chip(SessionStatusChip::new("⚙", "2 running").color(Color::Yellow))
            .view(96);
        let plain = strip_ansi(&rendered);

        assert_eq!(visible_len(&rendered), 96);
        assert!(plain.contains("a3s git:(main)"));
        assert!(plain.contains("gpt-5 (128k context)"));
        assert!(plain.contains("ctx:70%"));
        assert!(plain.contains('▰'));
        assert!(plain.contains('▱'));
        assert!(plain.contains("🎯 ship tui"));
        assert!(plain.contains("⚙ 2 running"));
        assert!(rendered.contains("\x1b[33mctx:70%\x1b[0m"));
    }

    #[test]
    fn context_color_turns_danger_at_auto_compact_threshold() {
        let rendered = SessionStatus::new("a3s")
            .model("claude-sonnet")
            .context(85, 100)
            .view(48);

        assert!(strip_ansi(&rendered).contains("ctx:85%"));
        assert!(rendered.contains("\x1b[31mctx:85%\x1b[0m"));
    }

    #[test]
    fn context_status_uses_shared_meter() {
        let status = SessionStatus::new("a3s").context(50, 100);
        let rendered = status.view(32);
        let plain = strip_ansi(&rendered);
        let expected = Meter::new(50.0)
            .width(CONTEXT_METER_WIDTH)
            .glyphs('▰', '▱')
            .show_value(false)
            .plain();

        assert!(plain.contains("ctx:50%"), "{plain}");
        assert!(plain.contains(&expected), "{plain}");
    }

    #[test]
    fn with_theme_applies_semantic_colors() {
        let theme = Theme::tokyo_night();
        let status = SessionStatus::new("/tmp/a3s").with_theme(&theme);

        assert_eq!(status.accent_color, theme.color(ThemeRole::Primary));
        assert_eq!(status.branch_color, theme.color(ThemeRole::Secondary));
        assert_eq!(status.text_color, theme.color(ThemeRole::Foreground));
        assert_eq!(status.muted_color, theme.color(ThemeRole::Muted));
        assert_eq!(status.warning_color, theme.color(ThemeRole::Warning));
        assert_eq!(status.danger_color, theme.color(ThemeRole::Error));
    }

    #[test]
    fn context_percent_handles_large_token_counts() {
        let limit = usize::MAX / 2;
        let used = limit / 2;
        let expected = ((used as u128 * 100) / limit as u128) as usize;

        assert_eq!(expected, 49);
        assert_eq!(context_percent(used, limit), expected);
    }

    #[test]
    fn falls_back_to_output_tokens_without_context_window() {
        let rendered = SessionStatus::new("/tmp/work")
            .model("anthropic/claude")
            .output_tokens(2048)
            .view(48);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("work"));
        assert!(plain.contains("claude"));
        assert!(plain.contains("2048 tok"));
    }

    #[test]
    fn truncates_cjk_rows_by_display_width() {
        let rendered = SessionStatus::new("/tmp/项目")
            .branch("功能分支")
            .model("provider/模型")
            .context(100, 100)
            .status_chip(SessionStatusChip::new("⇉", "并行代理任务").max_label_width(6))
            .view(32);

        assert_eq!(visible_len(&rendered), 32);
        assert!(strip_ansi(&rendered).contains('…'));
    }

    #[test]
    fn oversized_margin_is_clamped_to_render_width() {
        let status = SessionStatus::new("/tmp/a3s").margin(usize::MAX);
        let rendered = status.view(8);

        assert_eq!(status.margin, MAX_SESSION_STATUS_MARGIN);
        assert_eq!(visible_len(&rendered), 8);

        let Element::Box(row) = status.element::<()>() else {
            panic!("expected row element");
        };
        let Element::Text(margin) = &row.children[0] else {
            panic!("expected margin text");
        };
        assert_eq!(margin.content.len(), MAX_SESSION_STATUS_MARGIN);
    }

    #[test]
    fn oversized_chip_label_width_is_clamped() {
        let chip = SessionStatusChip::new("*", "running").max_label_width(usize::MAX);
        let rendered = SessionStatus::new("/tmp/a3s")
            .status_chip(chip.clone())
            .view(24);

        assert_eq!(
            chip.max_label_width,
            Some(MAX_SESSION_STATUS_CHIP_LABEL_WIDTH)
        );
        assert_eq!(visible_len(&rendered), 24);
    }

    #[test]
    fn element_produces_structured_status_segments() {
        let element: Element<()> = SessionStatus::new("/tmp/a3s")
            .branch("main")
            .model("openai/gpt-5")
            .context(86, 100)
            .status_chip(SessionStatusChip::new("⚙", "1 running").color(Color::Yellow))
            .element();

        match element {
            Element::Box(row) => {
                assert!(row.children.len() >= 12);
                match &row.children[1] {
                    Element::Text(text) => {
                        assert_eq!(text.content, "a3s");
                        assert_eq!(text.style.fg, Some(Color::Cyan));
                        assert!(text.style.bold);
                    }
                    _ => panic!("expected workspace text"),
                }
            }
            _ => panic!("expected row element"),
        }
    }
}
