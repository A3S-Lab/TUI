//! Terminal Unicode renderer for Mermaid `sequenceDiagram` fences.
//!
//! Cursor CLI-style architecture diagrams: participant boxes, lifelines,
//! solid/dashed arrows, self-calls, and alt/opt/loop frames. Unsupported
//! Mermaid dialects fall back to the plain code-block path.

use crate::style::{truncate_visible, visible_len, Color, Style};

const MIN_PARTICIPANT_GAP: usize = 3;
const BOX_PAD: usize = 1;

#[derive(Debug, Clone)]
struct Participant {
    id: String,
    label: String,
}

#[derive(Debug, Clone)]
enum ArrowKind {
    Solid,
    Dashed,
}

#[derive(Debug, Clone)]
enum Statement {
    Message {
        from: String,
        to: String,
        kind: ArrowKind,
        text: String,
    },
    Note {
        over: Vec<String>,
        text: String,
    },
    FrameStart {
        kind: FrameKind,
        label: String,
    },
    FrameElse {
        label: String,
    },
    FrameEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FrameKind {
    Alt,
    Opt,
    Loop,
    Par,
    Critical,
    Rect,
}

#[derive(Debug, Default)]
struct Diagram {
    title: Option<String>,
    participants: Vec<Participant>,
    statements: Vec<Statement>,
}

/// Try to render a Mermaid fence body as a sequence diagram.
///
/// Returns `None` when the body is not a `sequenceDiagram` or cannot be laid
/// out (caller should keep the plain code path).
pub(super) fn try_render_sequence(source: &str, width: usize) -> Option<Vec<String>> {
    let diagram = parse_sequence(source)?;
    if diagram.participants.is_empty() {
        return None;
    }
    Some(render_diagram(&diagram, width.max(16)))
}

fn parse_sequence(source: &str) -> Option<Diagram> {
    let mut lines = source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("%%"));
    let first = lines.next()?;
    if !first.eq_ignore_ascii_case("sequencediagram") {
        return None;
    }

    let mut diagram = Diagram::default();
    for line in lines {
        if let Some(title) = strip_prefix_ci(line, "title") {
            diagram.title = Some(title.trim().to_string());
            continue;
        }
        if let Some(rest) =
            strip_prefix_ci(line, "participant").or_else(|| strip_prefix_ci(line, "actor"))
        {
            let (id, label) = parse_participant(rest.trim());
            ensure_participant(&mut diagram, &id, Some(label));
            continue;
        }
        if let Some(rest) = strip_prefix_ci(line, "note") {
            if let Some(note) = parse_note(rest.trim()) {
                for id in &note.over {
                    ensure_participant(&mut diagram, id, None);
                }
                diagram.statements.push(Statement::Note {
                    over: note.over,
                    text: note.text,
                });
            }
            continue;
        }
        if let Some(stmt) = parse_frame_line(line) {
            diagram.statements.push(stmt);
            continue;
        }
        if let Some((from, to, kind, text)) = parse_message(line) {
            ensure_participant(&mut diagram, &from, None);
            ensure_participant(&mut diagram, &to, None);
            diagram.statements.push(Statement::Message {
                from,
                to,
                kind,
                text,
            });
            continue;
        }
        // Soft-ignore activate/deactivate/autonumber/etc.
        if starts_with_ci(line, "activate")
            || starts_with_ci(line, "deactivate")
            || starts_with_ci(line, "autonumber")
            || starts_with_ci(line, "create")
            || starts_with_ci(line, "destroy")
            || starts_with_ci(line, "link")
        {
            continue;
        }
        // Unknown directive — keep parsing remaining lines rather than failing
        // the whole diagram for one oddity.
    }
    Some(diagram)
}

fn parse_participant(rest: &str) -> (String, String) {
    if let Some((id, label)) = rest.split_once(" as ") {
        (id.trim().to_string(), label.trim().to_string())
    } else {
        let id = rest.trim().to_string();
        (id.clone(), id)
    }
}

struct NoteParsed {
    over: Vec<String>,
    text: String,
}

fn parse_note(rest: &str) -> Option<NoteParsed> {
    let rest = rest.trim();
    let (target, text) = rest.split_once(':')?;
    let text = text.trim().to_string();
    let target = target.trim();
    let over = if let Some(ids) = strip_prefix_ci(target, "over") {
        ids.split(',')
            .map(|id| id.trim().to_string())
            .filter(|id| !id.is_empty())
            .collect()
    } else {
        let id =
            strip_prefix_ci(target, "right of").or_else(|| strip_prefix_ci(target, "left of"))?;
        vec![id.trim().to_string()]
    };
    if over.is_empty() {
        return None;
    }
    Some(NoteParsed { over, text })
}

fn parse_frame_line(line: &str) -> Option<Statement> {
    if line.eq_ignore_ascii_case("end") {
        return Some(Statement::FrameEnd);
    }
    if let Some(label) = strip_prefix_ci(line, "else") {
        return Some(Statement::FrameElse {
            label: label.trim().to_string(),
        });
    }
    for (prefix, kind) in [
        ("alt", FrameKind::Alt),
        ("opt", FrameKind::Opt),
        ("loop", FrameKind::Loop),
        ("par", FrameKind::Par),
        ("critical", FrameKind::Critical),
        ("rect", FrameKind::Rect),
    ] {
        if let Some(label) = strip_prefix_ci(line, prefix) {
            return Some(Statement::FrameStart {
                kind,
                label: label.trim().to_string(),
            });
        }
    }
    if let Some(label) = strip_prefix_ci(line, "and") {
        return Some(Statement::FrameElse {
            label: label.trim().to_string(),
        });
    }
    None
}

fn parse_message(line: &str) -> Option<(String, String, ArrowKind, String)> {
    let (left, text) = line.split_once(':')?;
    let text = text.trim().to_string();
    let left = left.trim();
    for (token, kind) in [
        ("-->>", ArrowKind::Dashed),
        ("-->", ArrowKind::Dashed),
        ("->>", ArrowKind::Solid),
        ("->", ArrowKind::Solid),
        ("--x", ArrowKind::Dashed),
        ("-x", ArrowKind::Solid),
        ("<<-->>", ArrowKind::Dashed),
        ("<<->>", ArrowKind::Solid),
    ] {
        if let Some((from, to)) = left.split_once(token) {
            let from = from.trim().to_string();
            let to = to.trim().to_string();
            if !from.is_empty() && !to.is_empty() {
                return Some((from, to, kind, text));
            }
        }
    }
    None
}

fn ensure_participant(diagram: &mut Diagram, id: &str, label: Option<String>) {
    if diagram.participants.iter().any(|p| p.id == id) {
        if let Some(label) = label {
            if let Some(existing) = diagram.participants.iter_mut().find(|p| p.id == id) {
                existing.label = label;
            }
        }
        return;
    }
    diagram.participants.push(Participant {
        id: id.to_string(),
        label: label.unwrap_or_else(|| id.to_string()),
    });
}

fn render_diagram(diagram: &Diagram, width: usize) -> Vec<String> {
    let centers = participant_centers(&diagram.participants, width);
    let mut out = Vec::new();

    if let Some(title) = &diagram.title {
        let title = Style::new()
            .fg(Color::Cyan)
            .bold()
            .render(&truncate_visible(title, width));
        out.push(fit_row(&title, width));
        out.push(fit_row("", width));
    }

    out.extend(render_participant_boxes(
        &diagram.participants,
        &centers,
        width,
    ));

    let mut frame_depth = 0usize;
    for stmt in &diagram.statements {
        match stmt {
            Statement::Message {
                from,
                to,
                kind,
                text,
            } => {
                out.extend(render_message(
                    &diagram.participants,
                    &centers,
                    from,
                    to,
                    kind,
                    text,
                    width,
                    frame_depth,
                ));
            }
            Statement::Note { over, text } => {
                out.extend(render_note(
                    &diagram.participants,
                    &centers,
                    over,
                    text,
                    width,
                    frame_depth,
                ));
            }
            Statement::FrameStart { kind, label } => {
                out.push(render_frame_top(*kind, label, width, frame_depth));
                frame_depth = frame_depth.saturating_add(1);
            }
            Statement::FrameElse { label } => {
                out.push(render_frame_else(
                    label,
                    width,
                    frame_depth.saturating_sub(1),
                ));
            }
            Statement::FrameEnd => {
                frame_depth = frame_depth.saturating_sub(1);
                out.push(render_frame_bottom(width, frame_depth));
            }
        }
    }

    // Close any unclosed frames softly.
    while frame_depth > 0 {
        frame_depth -= 1;
        out.push(render_frame_bottom(width, frame_depth));
    }

    out.push(render_lifelines(&centers, width, 0));
    out
}

fn participant_centers(participants: &[Participant], width: usize) -> Vec<usize> {
    let n = participants.len().max(1);
    let labels: Vec<usize> = participants
        .iter()
        .map(|p| visible_len(&p.label).saturating_add(BOX_PAD * 2).max(3))
        .collect();
    let total_boxes: usize = labels.iter().sum();
    let gaps = n.saturating_sub(1);
    let remaining = width.saturating_sub(total_boxes);
    let gap = match remaining.checked_div(gaps) {
        Some(g) => g.max(MIN_PARTICIPANT_GAP),
        None => 0,
    };

    let mut centers = Vec::with_capacity(n);
    let mut cursor = 0usize;
    for (i, box_w) in labels.iter().enumerate() {
        centers.push(cursor + box_w / 2);
        cursor = cursor.saturating_add(*box_w);
        if i + 1 < n {
            cursor = cursor.saturating_add(gap);
        }
    }
    // If overflow, compress toward equal spacing within width.
    if cursor > width && n > 1 {
        let step = width.saturating_sub(1) / (n - 1);
        centers = (0..n).map(|i| i * step).collect();
    }
    centers
}

fn render_participant_boxes(
    participants: &[Participant],
    centers: &[usize],
    width: usize,
) -> Vec<String> {
    let boxes: Vec<(usize, usize, String)> = participants
        .iter()
        .zip(centers.iter())
        .map(|(p, &center)| {
            let inner = truncate_visible(&p.label, width.saturating_sub(4).max(1));
            let box_w = visible_len(&inner).saturating_add(BOX_PAD * 2).max(3);
            let start = center
                .saturating_sub(box_w / 2)
                .min(width.saturating_sub(box_w));
            (start, box_w, inner)
        })
        .collect();

    let mut top = vec![' '; width];
    let mut mid = vec![' '; width];
    let mut bot = vec![' '; width];
    for (start, box_w, inner) in &boxes {
        place_box_row(&mut top, *start, *box_w, '╭', '─', '╮');
        place_box_mid(&mut mid, *start, *box_w, inner);
        place_box_row(&mut bot, *start, *box_w, '╰', '─', '╯');
        // Lifeline stub through bottom border center.
        let cx = start + box_w / 2;
        if cx < width {
            bot[cx] = '┬';
        }
    }
    vec![
        fit_row(&chars_to_string(&top), width),
        fit_row(
            &Style::new().fg(Color::White).render(&chars_to_string(&mid)),
            width,
        ),
        fit_row(&chars_to_string(&bot), width),
    ]
}

fn place_box_row(
    row: &mut [char],
    start: usize,
    box_w: usize,
    left: char,
    fill: char,
    right: char,
) {
    if box_w == 0 || start >= row.len() {
        return;
    }
    row[start] = left;
    for i in 1..box_w.saturating_sub(1) {
        if start + i < row.len() {
            row[start + i] = fill;
        }
    }
    if box_w > 1 {
        let end = start + box_w - 1;
        if end < row.len() {
            row[end] = right;
        }
    }
}

fn place_box_mid(row: &mut [char], start: usize, box_w: usize, inner: &str) {
    if box_w == 0 || start >= row.len() {
        return;
    }
    row[start] = '│';
    let content_start = start + BOX_PAD;
    for (i, ch) in inner.chars().enumerate() {
        let idx = content_start + i;
        if idx < row.len() && idx < start + box_w.saturating_sub(1) {
            row[idx] = ch;
        }
    }
    let end = start + box_w - 1;
    if end < row.len() {
        row[end] = '│';
    }
}

fn render_lifelines(centers: &[usize], width: usize, frame_depth: usize) -> String {
    let mut row = vec![' '; width];
    for &cx in centers {
        if cx < width {
            row[cx] = '│';
        }
    }
    let line = chars_to_string(&row);
    let styled = Style::new().fg(Color::BrightBlack).render(&line);
    with_frame_gutter(&styled, width, frame_depth)
}

#[allow(clippy::too_many_arguments)]
fn render_message(
    participants: &[Participant],
    centers: &[usize],
    from: &str,
    to: &str,
    kind: &ArrowKind,
    text: &str,
    width: usize,
    frame_depth: usize,
) -> Vec<String> {
    let Some(from_i) = participants.iter().position(|p| p.id == from) else {
        return Vec::new();
    };
    let Some(to_i) = participants.iter().position(|p| p.id == to) else {
        return Vec::new();
    };
    let from_c = centers[from_i];
    let to_c = centers[to_i];

    let mut rows = Vec::new();
    rows.push(render_lifelines(centers, width, frame_depth));

    if from_i == to_i {
        // Self-call: short hook to the right of the lifeline.
        let label = truncate_visible(text, width.saturating_sub(from_c + 6).max(1));
        let mut hook = vec![' '; width];
        for &cx in centers {
            if cx < width {
                hook[cx] = '│';
            }
        }
        let right = (from_c + 4).min(width.saturating_sub(1));
        if from_c + 1 < width {
            for cell in hook.iter_mut().take(right + 1).skip(from_c + 1) {
                *cell = '─';
            }
            hook[right] = '┐';
        }
        rows.push(with_frame_gutter(
            &Style::new()
                .fg(Color::White)
                .render(&chars_to_string(&hook)),
            width,
            frame_depth,
        ));
        let mut mid = vec![' '; width];
        for &cx in centers {
            if cx < width {
                mid[cx] = '│';
            }
        }
        if right < width {
            mid[right] = '│';
        }
        let label_at = (from_c + 2).min(width.saturating_sub(1));
        place_text(&mut mid, label_at, &label);
        rows.push(with_frame_gutter(
            &Style::new()
                .fg(Color::BrightWhite)
                .render(&chars_to_string(&mid)),
            width,
            frame_depth,
        ));
        let mut back = vec![' '; width];
        for &cx in centers {
            if cx < width {
                back[cx] = '│';
            }
        }
        if from_c + 1 < width {
            for cell in back.iter_mut().take(right + 1).skip(from_c + 1) {
                *cell = '─';
            }
            back[right] = '┘';
            back[from_c] = '◀';
        }
        rows.push(with_frame_gutter(
            &Style::new()
                .fg(Color::White)
                .render(&chars_to_string(&back)),
            width,
            frame_depth,
        ));
        return rows;
    }

    let (left, right, forward) = if from_c <= to_c {
        (from_c, to_c, true)
    } else {
        (to_c, from_c, false)
    };
    let span = right.saturating_sub(left).max(1);
    let label = truncate_visible(text, span.saturating_sub(1).max(1));
    let label_start = left + span.saturating_sub(visible_len(&label)) / 2;

    let mut label_row = vec![' '; width];
    for &cx in centers {
        if cx < width {
            label_row[cx] = '│';
        }
    }
    place_text(&mut label_row, label_start, &label);
    rows.push(with_frame_gutter(
        &Style::new()
            .fg(Color::BrightWhite)
            .render(&chars_to_string(&label_row)),
        width,
        frame_depth,
    ));

    let mut arrow_row = vec![' '; width];
    for &cx in centers {
        if cx < width {
            arrow_row[cx] = '│';
        }
    }
    let fill = match kind {
        ArrowKind::Solid => '─',
        ArrowKind::Dashed => '╌',
    };
    for (x, cell) in arrow_row.iter_mut().enumerate().take(right + 1).skip(left) {
        if x < width {
            *cell = fill;
        }
    }
    if forward {
        if right < width {
            arrow_row[right] = '▶';
        }
        if left < width {
            arrow_row[left] = '│';
        }
    } else {
        if left < width {
            arrow_row[left] = '◀';
        }
        if right < width {
            arrow_row[right] = '│';
        }
    }
    rows.push(with_frame_gutter(
        &Style::new()
            .fg(Color::White)
            .render(&chars_to_string(&arrow_row)),
        width,
        frame_depth,
    ));
    rows
}

fn render_note(
    participants: &[Participant],
    centers: &[usize],
    over: &[String],
    text: &str,
    width: usize,
    frame_depth: usize,
) -> Vec<String> {
    let idxs: Vec<usize> = over
        .iter()
        .filter_map(|id| participants.iter().position(|p| p.id == *id))
        .collect();
    if idxs.is_empty() {
        return Vec::new();
    }
    let left = idxs.iter().map(|&i| centers[i]).min().unwrap_or(0);
    let right = idxs.iter().map(|&i| centers[i]).max().unwrap_or(0);
    let box_left = left.saturating_sub(1);
    let box_right = (right + 1).min(width.saturating_sub(1));
    let inner_w = box_right.saturating_sub(box_left).saturating_sub(1).max(1);
    let label = truncate_visible(text, inner_w);

    let mut rows = vec![render_lifelines(centers, width, frame_depth)];
    let mut top = vec![' '; width];
    let mut mid = vec![' '; width];
    let mut bot = vec![' '; width];
    for &cx in centers {
        if cx < width {
            top[cx] = '│';
            mid[cx] = '│';
            bot[cx] = '│';
        }
    }
    place_box_row(&mut top, box_left, box_right - box_left + 1, '╭', '─', '╮');
    place_box_mid(&mut mid, box_left, box_right - box_left + 1, &label);
    place_box_row(&mut bot, box_left, box_right - box_left + 1, '╰', '─', '╯');
    let dim = Style::new().fg(Color::BrightBlack);
    rows.push(with_frame_gutter(
        &dim.render(&chars_to_string(&top)),
        width,
        frame_depth,
    ));
    rows.push(with_frame_gutter(
        &Style::new().fg(Color::White).render(&chars_to_string(&mid)),
        width,
        frame_depth,
    ));
    rows.push(with_frame_gutter(
        &dim.render(&chars_to_string(&bot)),
        width,
        frame_depth,
    ));
    rows
}

fn frame_kind_label(kind: FrameKind) -> &'static str {
    match kind {
        FrameKind::Alt => "alt",
        FrameKind::Opt => "opt",
        FrameKind::Loop => "loop",
        FrameKind::Par => "par",
        FrameKind::Critical => "critical",
        FrameKind::Rect => "rect",
    }
}

fn render_frame_top(kind: FrameKind, label: &str, width: usize, depth: usize) -> String {
    let tag = frame_kind_label(kind);
    let body = if label.is_empty() {
        format!("┌─ {tag} ")
    } else {
        format!("┌─ {tag} [{label}] ")
    };
    let fill_len = width.saturating_sub(visible_len(&body)).saturating_sub(1);
    let line = format!("{body}{}┐", "─".repeat(fill_len));
    with_frame_gutter(
        &Style::new()
            .fg(Color::BrightBlack)
            .render(&truncate_visible(&line, width)),
        width,
        depth,
    )
}

fn render_frame_else(label: &str, width: usize, depth: usize) -> String {
    let body = if label.is_empty() {
        "├─ else ".to_string()
    } else {
        format!("├─ else [{label}] ")
    };
    let fill_len = width.saturating_sub(visible_len(&body)).saturating_sub(1);
    let line = format!("{body}{}┤", "─".repeat(fill_len));
    with_frame_gutter(
        &Style::new()
            .fg(Color::BrightBlack)
            .render(&truncate_visible(&line, width)),
        width,
        depth,
    )
}

fn render_frame_bottom(width: usize, depth: usize) -> String {
    let line = format!("└{}┘", "─".repeat(width.saturating_sub(2)));
    with_frame_gutter(
        &Style::new()
            .fg(Color::BrightBlack)
            .render(&truncate_visible(&line, width)),
        width,
        depth,
    )
}

fn with_frame_gutter(line: &str, width: usize, depth: usize) -> String {
    if depth == 0 {
        return fit_row(line, width);
    }
    // Nested frames already draw their own borders; depth is reserved for a
    // future indent. Keep width stable.
    fit_row(line, width)
}

fn place_text(row: &mut [char], start: usize, text: &str) {
    for (i, ch) in text.chars().enumerate() {
        let idx = start + i;
        if idx < row.len() {
            row[idx] = ch;
        }
    }
}

fn chars_to_string(chars: &[char]) -> String {
    chars.iter().collect()
}

fn fit_row(line: &str, width: usize) -> String {
    let plain_len = visible_len(line);
    if plain_len >= width {
        // ANSI-aware truncate is not available for mixed styles here; callers
        // already truncate plain content. Pad/clip via truncate_visible on
        // stripped content when over-width without ANSI.
        if line.contains('\u{1b}') {
            return line.to_string();
        }
        return truncate_visible(line, width);
    }
    format!("{line}{}", " ".repeat(width - plain_len))
}

fn strip_prefix_ci<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    let line_bytes = line.as_bytes();
    let prefix_bytes = prefix.as_bytes();
    if line_bytes.len() < prefix_bytes.len() {
        return None;
    }
    if !line_bytes[..prefix_bytes.len()].eq_ignore_ascii_case(prefix_bytes) {
        return None;
    }
    let rest = &line[prefix.len()..];
    if rest.is_empty() || rest.starts_with(|c: char| c.is_whitespace() || c == ':' || c == '[') {
        Some(rest)
    } else {
        None
    }
}

fn starts_with_ci(line: &str, prefix: &str) -> bool {
    strip_prefix_ci(line, prefix).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::strip_ansi;

    #[test]
    fn renders_cursor_style_sequence_with_alt() {
        let source = r#"
sequenceDiagram
    title Architecture
    participant ToolAsk
    participant App
    participant ApprovalPrompt
    participant ApprovalTick
    ToolAsk->>App: pending_tools push + State::Awaiting
    App->>App: arm deadline Instant
    App->>ApprovalTick: cmd::tick ~100ms
    ApprovalTick-->>App: recompute remaining fraction
    App->>ApprovalPrompt: lines(width, remaining)
    ApprovalPrompt->>ApprovalPrompt: paint cyan bar + options
    alt user action
        App->>ApprovalPrompt: clear deadline; apply_approval
    else remaining elapsed
        App->>ApprovalPrompt: timeout Skip & tell
    end
"#;
        let lines = try_render_sequence(source, 88).expect("render");
        let plain: Vec<String> = lines.iter().map(|l| strip_ansi(l)).collect();
        assert!(plain[0].contains("Architecture"), "{plain:?}");
        assert!(plain.iter().any(|l| l.contains("ToolAsk")));
        assert!(plain.iter().any(|l| l.contains("▶") || l.contains('→')));
        assert!(plain.iter().any(|l| l.contains("alt")));
        assert!(plain.iter().any(|l| l.contains("else")));
        assert!(lines[0].contains(&Color::Cyan.fg_ansi()));
    }

    #[test]
    fn non_sequence_mermaid_returns_none() {
        assert!(try_render_sequence("flowchart TD\n  A-->B\n", 40).is_none());
    }

    #[test]
    fn participant_as_alias_is_used_for_box_label() {
        let source = "sequenceDiagram\nparticipant A as Agent\nparticipant B as Host\nA->>B: hi\n";
        let lines = try_render_sequence(source, 48).expect("render");
        let plain = lines
            .iter()
            .map(|l| strip_ansi(l))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(plain.contains("Agent"), "{plain}");
        assert!(plain.contains("Host"), "{plain}");
    }
}
