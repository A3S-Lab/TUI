use super::Markdown;
use crate::element::{BoxElement, Element, FlexDirection, TextElement, TextStyle};
use crate::style::{
    ansi_escape_sequence_end, next_display_cell_boundary, osc8_link_target,
    split_lines_preserving_trailing_blank, Color, Style,
};

#[derive(Clone)]
struct StyledGrapheme {
    text: String,
    style: RenderStyle,
    width: usize,
}

#[derive(Clone, Default)]
struct RenderStyle {
    text: crate::element::TextStyle,
    hyperlink: Option<String>,
}

pub(super) fn wrap_styled_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }

    let mut words = Vec::<(Vec<StyledGrapheme>, Option<RenderStyle>)>::new();
    let mut word = Vec::new();
    let mut separator_before = None;
    let mut pending_separator = None;
    for segment in ansi_segments(text) {
        let segment_style = RenderStyle {
            text: segment.style.clone(),
            hyperlink: segment.hyperlink.clone(),
        };
        let mut offset = 0usize;
        while offset < segment.text.len() {
            let Some((end, cluster_width)) = next_display_cell_boundary(&segment.text, offset)
            else {
                break;
            };
            let cluster = &segment.text[offset..end];
            offset = end;
            if cluster.chars().all(char::is_whitespace) {
                if !word.is_empty() {
                    words.push((std::mem::take(&mut word), separator_before.take()));
                }
                pending_separator.get_or_insert_with(|| segment_style.clone());
            } else {
                if word.is_empty() {
                    separator_before = pending_separator.take();
                }
                word.push(StyledGrapheme {
                    text: cluster.to_string(),
                    style: segment_style.clone(),
                    width: cluster_width,
                });
            }
        }
    }
    if !word.is_empty() {
        words.push((word, separator_before));
    }
    if words.is_empty() {
        return vec![String::new()];
    }

    let mut rows = Vec::<Vec<StyledGrapheme>>::new();
    let mut row = Vec::new();
    let mut row_width = 0usize;
    for (word, separator_style) in words {
        let word_width = word.iter().map(|cluster| cluster.width).sum::<usize>();
        let separator_width = usize::from(!row.is_empty());
        if !row.is_empty() && row_width + separator_width + word_width > width {
            rows.push(std::mem::take(&mut row));
            row_width = 0;
        }
        if word_width <= width {
            if !row.is_empty() {
                row.push(StyledGrapheme {
                    text: " ".to_string(),
                    style: separator_style.unwrap_or_default(),
                    width: 1,
                });
                row_width += 1;
            }
            row_width += word_width;
            row.extend(word);
            continue;
        }

        if !row.is_empty() {
            rows.push(std::mem::take(&mut row));
            row_width = 0;
        }
        for cluster in word {
            if row_width > 0 && row_width + cluster.width > width {
                rows.push(std::mem::take(&mut row));
                row_width = 0;
            }
            row_width += cluster.width;
            row.push(cluster);
        }
    }
    if !row.is_empty() {
        rows.push(row);
    }
    rows.into_iter().map(render_styled_graphemes).collect()
}

fn render_styled_graphemes(graphemes: Vec<StyledGrapheme>) -> String {
    let mut rendered = String::new();
    let mut run = String::new();
    let mut run_style = RenderStyle::default();
    let mut has_run = false;
    for grapheme in graphemes {
        if has_run && !same_render_style(&run_style, &grapheme.style) {
            rendered.push_str(&render_text_style(&run_style, &run));
            run.clear();
        }
        if !has_run || !same_render_style(&run_style, &grapheme.style) {
            run_style = grapheme.style;
            has_run = true;
        }
        run.push_str(&grapheme.text);
    }
    if has_run {
        rendered.push_str(&render_text_style(&run_style, &run));
    }
    rendered
}

fn same_render_style(a: &RenderStyle, b: &RenderStyle) -> bool {
    same_text_style(&a.text, &b.text) && a.hyperlink == b.hyperlink
}

fn same_text_style(a: &crate::element::TextStyle, b: &crate::element::TextStyle) -> bool {
    a.fg == b.fg
        && a.bg == b.bg
        && a.bold == b.bold
        && a.italic == b.italic
        && a.underline == b.underline
        && a.reverse == b.reverse
        && a.dim == b.dim
        && a.strikethrough == b.strikethrough
}

fn render_text_style(style: &RenderStyle, text: &str) -> String {
    let mut terminal_style = Style::new();
    if let Some(color) = style.text.fg {
        terminal_style = terminal_style.fg(color);
    }
    if let Some(color) = style.text.bg {
        terminal_style = terminal_style.bg(color);
    }
    if style.text.bold {
        terminal_style = terminal_style.bold();
    }
    if style.text.italic {
        terminal_style = terminal_style.italic();
    }
    if style.text.underline {
        terminal_style = terminal_style.underline();
    }
    if style.text.reverse {
        terminal_style = terminal_style.reverse();
    }
    if style.text.dim {
        terminal_style = terminal_style.dim();
    }
    if style.text.strikethrough {
        terminal_style = terminal_style.strikethrough();
    }
    let rendered = if same_text_style(&style.text, &crate::element::TextStyle::default()) {
        text.to_string()
    } else {
        terminal_style.render(text)
    };
    if let Some(target) = &style.hyperlink {
        format!("\x1b]8;;{target}\x1b\\{rendered}\x1b]8;;\x1b\\")
    } else {
        rendered
    }
}

pub(super) struct StyledSegment {
    pub(super) text: String,
    pub(super) style: TextStyle,
    hyperlink: Option<String>,
}

impl Markdown {
    pub fn render_element<Msg>(&self, input: &str) -> Element<Msg> {
        let rendered = self.render(input);
        rendered_markdown_element(&rendered)
    }
}

pub(crate) fn rendered_markdown_element<Msg>(rendered: &str) -> Element<Msg> {
    let children: Vec<Element<Msg>> = split_rendered_lines(rendered)
        .into_iter()
        .map(ansi_line_element)
        .collect();

    Element::Box(
        BoxElement::new()
            .direction(FlexDirection::Column)
            .children(children),
    )
}

pub(crate) fn split_rendered_lines(rendered: &str) -> Vec<&str> {
    if rendered.is_empty() {
        Vec::new()
    } else {
        split_lines_preserving_trailing_blank(rendered)
    }
}

fn ansi_line_element<Msg>(line: &str) -> Element<Msg> {
    let segments = ansi_segments(line);
    if segments.len() == 1 {
        let Some(segment) = segments.into_iter().next() else {
            return Element::Text(TextElement::new(""));
        };
        return Element::Text(segment_text(segment));
    }

    Element::Box(
        BoxElement::new().direction(FlexDirection::Row).children(
            segments
                .into_iter()
                .map(|segment| Element::Text(segment_text(segment)))
                .collect(),
        ),
    )
}

fn segment_text(segment: StyledSegment) -> TextElement {
    let mut text = TextElement::new(segment.text);
    text.style = segment.style;
    text
}

pub(super) fn ansi_segments(line: &str) -> Vec<StyledSegment> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut style = TextStyle::default();
    let mut hyperlink = None;
    let mut index = 0usize;

    while index < line.len() {
        if let Some(end) = ansi_escape_sequence_end(line, index) {
            let sequence = &line[index..end];
            if let Some(params) = sequence
                .strip_prefix("\x1b[")
                .and_then(|value| value.strip_suffix('m'))
            {
                push_segment(&mut segments, &mut current, &style, &hyperlink);
                apply_sgr_sequence(&mut style, params);
            } else if let Some(target) = osc8_link_target(sequence) {
                push_segment(&mut segments, &mut current, &style, &hyperlink);
                hyperlink = (!target.is_empty()).then(|| target.to_string());
            }
            index = end;
            continue;
        }
        let ch = line[index..].chars().next().unwrap_or_default();
        current.push(ch);
        index += ch.len_utf8();
    }

    push_segment(&mut segments, &mut current, &style, &hyperlink);
    if segments.is_empty() {
        segments.push(StyledSegment {
            text: String::new(),
            style: TextStyle::default(),
            hyperlink: None,
        });
    }
    segments
}

pub(super) fn trailing_background(line: &str) -> Option<Color> {
    ansi_segments(line)
        .into_iter()
        .rev()
        .find(|segment| {
            segment
                .text
                .chars()
                .any(|ch| !ch.is_control() && ch != '\u{200b}')
        })
        .and_then(|segment| segment.style.bg)
}

fn push_segment(
    segments: &mut Vec<StyledSegment>,
    current: &mut String,
    style: &TextStyle,
    hyperlink: &Option<String>,
) {
    if current.is_empty() {
        return;
    }

    segments.push(StyledSegment {
        text: std::mem::take(current),
        style: style.clone(),
        hyperlink: hyperlink.clone(),
    });
}

fn apply_sgr_sequence(style: &mut TextStyle, sequence: &str) {
    if sequence.is_empty() {
        *style = TextStyle::default();
        return;
    }

    let codes: Vec<u16> = sequence
        .split(';')
        .filter_map(|part| {
            if part.is_empty() {
                Some(0)
            } else {
                part.parse().ok()
            }
        })
        .collect();
    if codes.is_empty() {
        *style = TextStyle::default();
        return;
    }

    let mut index = 0usize;
    while index < codes.len() {
        match codes[index] {
            0 => *style = TextStyle::default(),
            1 => style.bold = true,
            2 => style.dim = true,
            3 => style.italic = true,
            4 => style.underline = true,
            7 => style.reverse = true,
            9 => style.strikethrough = true,
            22 => {
                style.bold = false;
                style.dim = false;
            }
            23 => style.italic = false,
            24 => style.underline = false,
            27 => style.reverse = false,
            29 => style.strikethrough = false,
            30..=37 | 90..=97 => style.fg = basic_sgr_color(codes[index]),
            39 => style.fg = None,
            40..=47 | 100..=107 => style.bg = basic_sgr_color(codes[index] - 10),
            49 => style.bg = None,
            38 | 48 => {
                let is_fg = codes[index] == 38;
                if let Some((color, consumed)) = extended_sgr_color(&codes[index + 1..]) {
                    if is_fg {
                        style.fg = Some(color);
                    } else {
                        style.bg = Some(color);
                    }
                    index += consumed;
                }
            }
            _ => {}
        }
        index += 1;
    }
}

fn basic_sgr_color(code: u16) -> Option<Color> {
    match code {
        30 => Some(Color::Black),
        31 => Some(Color::Red),
        32 => Some(Color::Green),
        33 => Some(Color::Yellow),
        34 => Some(Color::Blue),
        35 => Some(Color::Magenta),
        36 => Some(Color::Cyan),
        37 => Some(Color::White),
        90 => Some(Color::BrightBlack),
        91 => Some(Color::BrightRed),
        92 => Some(Color::BrightGreen),
        93 => Some(Color::BrightYellow),
        94 => Some(Color::BrightBlue),
        95 => Some(Color::BrightMagenta),
        96 => Some(Color::BrightCyan),
        97 => Some(Color::BrightWhite),
        _ => None,
    }
}

fn extended_sgr_color(codes: &[u16]) -> Option<(Color, usize)> {
    match codes {
        [5, value, ..] => Some((Color::Ansi256((*value).min(u8::MAX as u16) as u8), 2)),
        [2, r, g, b, ..] => Some((
            Color::Rgb(
                (*r).min(u8::MAX as u16) as u8,
                (*g).min(u8::MAX as u16) as u8,
                (*b).min(u8::MAX as u16) as u8,
            ),
            4,
        )),
        _ => None,
    }
}
