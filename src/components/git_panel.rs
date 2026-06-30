use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, strip_ansi, truncate_visible, visible_len, Color, Style};

/// Active view inside a [`GitPanel`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitPanelView {
    Status,
    Log,
}

/// One changed file row in a [`GitPanel`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitStatusFile {
    index: char,
    worktree: char,
    path: String,
}

impl GitStatusFile {
    pub fn new(index: char, worktree: char, path: impl Into<String>) -> Self {
        Self {
            index,
            worktree,
            path: path.into(),
        }
    }

    pub fn index_status(&self) -> char {
        self.index
    }

    pub fn worktree_status(&self) -> char {
        self.worktree
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn staged(&self) -> bool {
        self.index != ' ' && self.index != '?'
    }

    pub fn untracked(&self) -> bool {
        self.index == '?'
    }
}

/// Git status/log panel renderer.
///
/// `GitPanel` extracts the CLI `/git` surface without owning git subprocesses:
/// callers provide changed files, log rows, selected indices, and the already
/// loaded diff/show output.
#[derive(Debug, Clone)]
pub struct GitPanel {
    branch: String,
    files: Vec<GitStatusFile>,
    selected_file: usize,
    log_entries: Vec<String>,
    selected_log: usize,
    view: GitPanelView,
    diff_lines: Vec<String>,
    diff_scroll: usize,
    commit_input: Option<String>,
    note: String,
    fill_height: bool,
    clean_text: String,
    log_empty_text: String,
    loading_log_text: String,
    footer_text: String,
    accent_color: Color,
    muted_color: Color,
    selected_fg: Color,
    staged_color: Color,
    modified_color: Color,
    untracked_color: Color,
    log_color: Color,
    hunk_color: Color,
    insert_color: Color,
    delete_color: Color,
    meta_color: Color,
}

impl GitPanel {
    pub fn new(branch: impl Into<String>) -> Self {
        Self {
            branch: branch.into(),
            files: Vec::new(),
            selected_file: 0,
            log_entries: Vec::new(),
            selected_log: 0,
            view: GitPanelView::Status,
            diff_lines: Vec::new(),
            diff_scroll: 0,
            commit_input: None,
            note: String::new(),
            fill_height: false,
            clean_text: "  working tree clean".to_string(),
            log_empty_text: "  no commits in this repository yet".to_string(),
            loading_log_text: "  loading commits...".to_string(),
            footer_text: "  ↑↓ select · Space/s stage · u unstage · a stage-all · c commit · Tab log · r refresh · Esc".to_string(),
            accent_color: Color::Cyan,
            muted_color: Color::BrightBlack,
            selected_fg: Color::Black,
            staged_color: Color::Green,
            modified_color: Color::Yellow,
            untracked_color: Color::Red,
            log_color: Color::Yellow,
            hunk_color: Color::Cyan,
            insert_color: Color::Green,
            delete_color: Color::Red,
            meta_color: Color::BrightBlack,
        }
    }

    pub fn branch(mut self, branch: impl Into<String>) -> Self {
        self.branch = branch.into();
        self
    }

    pub fn files(mut self, files: Vec<GitStatusFile>) -> Self {
        self.files = files;
        self.clamp_selection();
        self
    }

    pub fn file(mut self, file: GitStatusFile) -> Self {
        self.files.push(file);
        self.clamp_selection();
        self
    }

    pub fn selected_file(mut self, selected: usize) -> Self {
        self.selected_file = selected;
        self.clamp_selection();
        self
    }

    pub fn log_entries(mut self, entries: Vec<impl Into<String>>) -> Self {
        self.log_entries = entries.into_iter().map(Into::into).collect();
        self.clamp_selection();
        self
    }

    pub fn log_entry(mut self, entry: impl Into<String>) -> Self {
        self.log_entries.push(entry.into());
        self.clamp_selection();
        self
    }

    pub fn selected_log(mut self, selected: usize) -> Self {
        self.selected_log = selected;
        self.clamp_selection();
        self
    }

    pub fn active_view(mut self, view: GitPanelView) -> Self {
        self.view = view;
        self
    }

    pub fn diff_lines(mut self, lines: Vec<impl Into<String>>) -> Self {
        self.diff_lines = lines.into_iter().map(Into::into).collect();
        self
    }

    pub fn diff_line(mut self, line: impl Into<String>) -> Self {
        self.diff_lines.push(line.into());
        self
    }

    pub fn diff_scroll(mut self, scroll: usize) -> Self {
        self.diff_scroll = scroll;
        self
    }

    pub fn commit_input(mut self, input: impl Into<String>) -> Self {
        self.commit_input = Some(input.into());
        self
    }

    pub fn clear_commit_input(mut self) -> Self {
        self.commit_input = None;
        self
    }

    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.note = note.into();
        self
    }

    pub fn footer_text(mut self, text: impl Into<String>) -> Self {
        self.footer_text = text.into();
        self
    }

    pub fn clean_text(mut self, text: impl Into<String>) -> Self {
        self.clean_text = text.into();
        self
    }

    pub fn log_empty_text(mut self, text: impl Into<String>) -> Self {
        self.log_empty_text = text.into();
        self
    }

    pub fn loading_log_text(mut self, text: impl Into<String>) -> Self {
        self.loading_log_text = text.into();
        self
    }

    pub fn fill_height(mut self, enabled: bool) -> Self {
        self.fill_height = enabled;
        self
    }

    pub fn accent_color(mut self, color: Color) -> Self {
        self.accent_color = color;
        self
    }

    pub fn muted_color(mut self, color: Color) -> Self {
        self.muted_color = color;
        self
    }

    pub fn selected_fg(mut self, color: Color) -> Self {
        self.selected_fg = color;
        self
    }

    pub fn status_colors(mut self, staged: Color, modified: Color, untracked: Color) -> Self {
        self.staged_color = staged;
        self.modified_color = modified;
        self.untracked_color = untracked;
        self
    }

    pub fn diff_colors(mut self, hunk: Color, insert: Color, delete: Color, meta: Color) -> Self {
        self.hunk_color = hunk;
        self.insert_color = insert;
        self.delete_color = delete;
        self.meta_color = meta;
        self
    }

    pub fn branch_value(&self) -> &str {
        &self.branch
    }

    pub fn files_value(&self) -> &[GitStatusFile] {
        &self.files
    }

    pub fn log_entries_value(&self) -> &[String] {
        &self.log_entries
    }

    pub fn active_view_value(&self) -> GitPanelView {
        self.view
    }

    pub fn view(&self, width: u16, height: usize) -> String {
        let width = width as usize;
        if width == 0 || height == 0 {
            return String::new();
        }

        let mut lines = self.render_lines(width, height);
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

    pub fn element<Msg>(&self, width: u16, height: usize) -> Element<Msg> {
        let width = width as usize;
        let mut rows = self.render_lines(width, height);
        rows.truncate(height);
        if self.fill_height {
            while rows.len() < height {
                rows.push(String::new());
            }
        }
        let row_count = rows.len();
        let mut children = Vec::new();

        for (index, row) in rows.into_iter().enumerate() {
            let plain = strip_ansi(&fit_visible(&row, width));
            let mut text = TextElement::new(plain);
            if index == 0 {
                text = text.fg(self.accent_color).bold();
            } else if index == 1 || index + 1 == row_count {
                text = text.fg(self.muted_color);
            }
            children.push(Element::Text(text));
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(children),
        )
    }

    fn render_lines(&self, width: usize, height: usize) -> Vec<String> {
        let mut out = vec![
            fit_visible(&self.render_header(), width),
            fit_visible(
                &Style::new().fg(self.muted_color).render(&"─".repeat(width)),
                width,
            ),
        ];
        let body = height.saturating_sub(3);

        match self.view {
            GitPanelView::Log => self.render_log_body(&mut out, width, body),
            GitPanelView::Status => self.render_status_body(&mut out, width, body),
        }

        if height > 2 {
            while out.len() + 1 < height {
                out.push(String::new());
            }
            out.push(fit_visible(&self.render_footer(), width));
        }

        out
    }

    fn render_header(&self) -> String {
        let branch = if self.branch.is_empty() {
            "(detached)"
        } else {
            self.branch.as_str()
        };
        let log_tab = if self.log_entries.is_empty() {
            "Log".to_string()
        } else {
            format!("Log ({})", self.log_entries.len())
        };
        let note = if self.note.is_empty() {
            String::new()
        } else {
            Style::new().fg(self.muted_color).render(&self.note)
        };

        format!(
            "  git · {branch}   {} {}  {}   {}",
            self.render_tab("Status", self.view == GitPanelView::Status),
            self.render_tab(&log_tab, self.view == GitPanelView::Log),
            Style::new()
                .fg(self.accent_color)
                .render("⇄ Tab to switch · commits in Log"),
            note
        )
    }

    fn render_tab(&self, label: &str, active: bool) -> String {
        if active {
            Style::new()
                .fg(self.selected_fg)
                .bg(self.accent_color)
                .bold()
                .render(&format!(" {label} "))
        } else {
            Style::new()
                .fg(self.muted_color)
                .render(&format!(" {label} "))
        }
    }

    fn render_status_body(&self, out: &mut Vec<String>, width: usize, body: usize) {
        let pane = PaneWidths::new(width);
        let start = self.selected_file.saturating_sub(body.saturating_sub(1));

        for offset in 0..body {
            let file_index = start + offset;
            let left = if let Some(file) = self.files.get(file_index) {
                self.render_status_file(file, file_index == self.selected_file, pane.left)
            } else if file_index == 0 && self.files.is_empty() {
                fit_visible(
                    &Style::new().fg(self.muted_color).render(&self.clean_text),
                    pane.left,
                )
            } else {
                " ".repeat(pane.left)
            };
            let right = self
                .diff_lines
                .get(self.diff_scroll + offset)
                .map(|line| self.render_diff_line(line, pane.right, false))
                .unwrap_or_default();
            out.push(fit_visible(
                &format!("{left}{}{right}", pane.separator),
                width,
            ));
        }
    }

    fn render_log_body(&self, out: &mut Vec<String>, width: usize, body: usize) {
        if self.log_entries.is_empty() {
            let msg = if self.note.is_empty() {
                self.log_empty_text.as_str()
            } else {
                self.loading_log_text.as_str()
            };
            if body > 0 {
                out.push(fit_visible(
                    &Style::new().fg(self.muted_color).render(msg),
                    width,
                ));
            }
            return;
        }

        let pane = PaneWidths::new(width);
        let start = self.selected_log.saturating_sub(body.saturating_sub(1));

        for offset in 0..body {
            let commit_index = start + offset;
            let left = if let Some(line) = self.log_entries.get(commit_index) {
                self.render_log_entry(line, commit_index == self.selected_log, pane.left)
            } else {
                " ".repeat(pane.left)
            };
            let right = self
                .diff_lines
                .get(self.diff_scroll + offset)
                .map(|line| self.render_diff_line(line, pane.right, true))
                .unwrap_or_default();
            out.push(fit_visible(
                &format!("{left}{}{right}", pane.separator),
                width,
            ));
        }
    }

    fn render_status_file(&self, file: &GitStatusFile, selected: bool, width: usize) -> String {
        let mark = format!("{}{}", file.index_status(), file.worktree_status());
        let raw = fit_visible(&format!(" {mark}  {}", file.path()), width);
        let color = if file.untracked() {
            self.untracked_color
        } else if file.staged() {
            self.staged_color
        } else {
            self.modified_color
        };

        if selected {
            Style::new().fg(self.selected_fg).bg(color).render(&raw)
        } else {
            Style::new().fg(color).render(&raw)
        }
    }

    fn render_log_entry(&self, line: &str, selected: bool, width: usize) -> String {
        let (hash, rest) = line.split_once(' ').unwrap_or((line, ""));
        let raw = fit_visible(&format!(" {hash}  {rest}"), width);

        if selected {
            return Style::new()
                .fg(self.selected_fg)
                .bg(self.log_color)
                .render(&raw);
        }

        let hash_cell = format!(" {hash} ");
        let hash_width = visible_len(&hash_cell).min(width);
        let rest_width = width.saturating_sub(hash_width);
        let row = format!(
            "{}{}",
            Style::new()
                .fg(self.log_color)
                .render(&fit_visible(&hash_cell, hash_width)),
            truncate_visible(rest, rest_width)
        );
        fit_visible(&row, width)
    }

    fn render_diff_line(&self, line: &str, width: usize, log_view: bool) -> String {
        let style = self.diff_style(line, log_view);
        style.render(&truncate_visible(line, width))
    }

    fn diff_style(&self, line: &str, log_view: bool) -> Style {
        if line.starts_with("@@") {
            return Style::new().fg(self.hunk_color);
        }
        if log_view && line.starts_with("commit ") {
            return Style::new().fg(self.log_color).bold();
        }
        if line.starts_with("diff ")
            || line.starts_with("index ")
            || line.starts_with("--- ")
            || line.starts_with("+++ ")
            || line.starts_with("\\ ")
        {
            return Style::new().fg(self.meta_color);
        }
        if line.starts_with('+') {
            return Style::new().fg(self.insert_color);
        }
        if line.starts_with('-') {
            return Style::new().fg(self.delete_color);
        }
        Style::new()
    }

    fn render_footer(&self) -> String {
        if let Some(input) = self.commit_input.as_deref() {
            Style::new().fg(self.log_color).bold().render(&format!(
                "  commit message: {input}_   (Enter commit · Esc cancel)"
            ))
        } else {
            Style::new().fg(self.muted_color).render(&self.footer_text)
        }
    }

    fn clamp_selection(&mut self) {
        if self.files.is_empty() {
            self.selected_file = 0;
        } else {
            self.selected_file = self.selected_file.min(self.files.len() - 1);
        }

        if self.log_entries.is_empty() {
            self.selected_log = 0;
        } else {
            self.selected_log = self.selected_log.min(self.log_entries.len() - 1);
        }
    }
}

#[derive(Debug, Clone)]
struct PaneWidths {
    left: usize,
    right: usize,
    separator: String,
}

impl PaneWidths {
    fn new(width: usize) -> Self {
        let separator = Style::new().fg(Color::BrightBlack).render(" │ ");
        let separator_width = visible_len(&separator);
        let desired_left = (width / 3).clamp(20, 46);
        let max_left = width.saturating_sub(separator_width + 1);
        let left = desired_left.min(max_left);
        let right = width.saturating_sub(left + separator_width);

        Self {
            left,
            right,
            separator,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::strip_ansi;

    #[test]
    fn status_view_renders_files_diff_and_footer() {
        let panel = sample_panel().fill_height(true);
        let rendered = panel.view(78, 8);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("git · main"), "{plain:?}");
        assert!(plain.contains("Status"), "{plain:?}");
        assert!(plain.contains("Log (2)"), "{plain:?}");
        assert!(plain.contains("M   src/lib.rs"), "{plain:?}");
        assert!(plain.contains("+new line"), "{plain:?}");
        assert!(plain.contains("Space/s stage"), "{plain:?}");
        assert!(rendered.contains("\x1b["));
        assert_lines_fit(&rendered, 78);
    }

    #[test]
    fn log_view_renders_commit_list_and_show_output() {
        let panel = sample_panel()
            .active_view(GitPanelView::Log)
            .selected_log(1)
            .diff_lines(vec![
                "commit cafebabe",
                "Author: A3S",
                "diff --git a/src/lib.rs b/src/lib.rs",
                "@@ -1 +1 @@",
            ]);
        let rendered = panel.view(84, 7);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("Log (2)"), "{plain:?}");
        assert!(plain.contains("cafebabe"), "{plain:?}");
        assert!(plain.contains("add reusable pan"), "{plain:?}");
        assert!(plain.contains("commit cafebabe"), "{plain:?}");
        assert_lines_fit(&rendered, 84);
    }

    #[test]
    fn commit_input_replaces_default_footer() {
        let panel = sample_panel().commit_input("extract git panel");
        let plain = strip_ansi(&panel.view(72, 6));

        assert!(
            plain.contains("commit message: extract git panel_"),
            "{plain:?}"
        );
        assert!(!plain.contains("Space/s stage"), "{plain:?}");
    }

    #[test]
    fn empty_log_and_clean_status_have_stable_states() {
        let clean = GitPanel::new("main");
        let clean_plain = strip_ansi(&clean.view(46, 5));
        assert!(
            clean_plain.contains("working tree clean"),
            "{clean_plain:?}"
        );

        let loading = GitPanel::new("main")
            .active_view(GitPanelView::Log)
            .note("loading...");
        let loading_plain = strip_ansi(&loading.view(46, 5));
        assert!(
            loading_plain.contains("loading commits"),
            "{loading_plain:?}"
        );
    }

    #[test]
    fn narrow_width_does_not_overflow() {
        let rendered = sample_panel().view(24, 5);
        assert_lines_fit(&rendered, 24);
    }

    fn sample_panel() -> GitPanel {
        GitPanel::new("main")
            .files(vec![
                GitStatusFile::new('M', ' ', "src/lib.rs"),
                GitStatusFile::new('?', '?', "tests/git_panel.rs"),
            ])
            .selected_file(0)
            .log_entries(vec![
                "1234567 initial commit",
                "cafebabe add reusable panel",
            ])
            .diff_lines(vec![
                "diff --git a/src/lib.rs b/src/lib.rs",
                "index 111..222 100644",
                "@@ -1,2 +1,3 @@",
                " context",
                "+new line",
                "-old line",
            ])
            .note("ready")
    }

    fn assert_lines_fit(rendered: &str, width: usize) {
        for line in rendered.lines() {
            assert_eq!(
                visible_len(line),
                width,
                "line should fit width {width}: {line:?}"
            );
        }
    }
}
