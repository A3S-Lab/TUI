//! Color, style, and text formatting utilities.

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    Ansi256(u8),
    Rgb(u8, u8, u8),
}

impl Color {
    pub fn fg_ansi(&self) -> String {
        self.fg_code()
    }

    pub fn bg_ansi(&self) -> String {
        self.bg_code()
    }

    fn fg_code(&self) -> String {
        match self {
            Color::Black => "30".into(),
            Color::Red => "31".into(),
            Color::Green => "32".into(),
            Color::Yellow => "33".into(),
            Color::Blue => "34".into(),
            Color::Magenta => "35".into(),
            Color::Cyan => "36".into(),
            Color::White => "37".into(),
            Color::BrightBlack => "90".into(),
            Color::BrightRed => "91".into(),
            Color::BrightGreen => "92".into(),
            Color::BrightYellow => "93".into(),
            Color::BrightBlue => "94".into(),
            Color::BrightMagenta => "95".into(),
            Color::BrightCyan => "96".into(),
            Color::BrightWhite => "97".into(),
            Color::Ansi256(n) => format!("38;5;{}", n),
            Color::Rgb(r, g, b) => format!("38;2;{};{};{}", r, g, b),
        }
    }

    fn bg_code(&self) -> String {
        match self {
            Color::Black => "40".into(),
            Color::Red => "41".into(),
            Color::Green => "42".into(),
            Color::Yellow => "43".into(),
            Color::Blue => "44".into(),
            Color::Magenta => "45".into(),
            Color::Cyan => "46".into(),
            Color::White => "47".into(),
            Color::BrightBlack => "100".into(),
            Color::BrightRed => "101".into(),
            Color::BrightGreen => "102".into(),
            Color::BrightYellow => "103".into(),
            Color::BrightBlue => "104".into(),
            Color::BrightMagenta => "105".into(),
            Color::BrightCyan => "106".into(),
            Color::BrightWhite => "107".into(),
            Color::Ansi256(n) => format!("48;5;{}", n),
            Color::Rgb(r, g, b) => format!("48;2;{};{};{}", r, g, b),
        }
    }

    /// Parse a hex color string (e.g., "#ff0000" or "ff0000").
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Color::Rgb(r, g, b))
    }

    /// Lighten an RGB color by a factor (0.0 to 1.0).
    pub fn lighten(&self, amount: f64) -> Self {
        match self {
            Color::Rgb(r, g, b) => {
                let factor = normalized_color_amount(amount);
                Color::Rgb(
                    (*r as f64 + (255.0 - *r as f64) * factor) as u8,
                    (*g as f64 + (255.0 - *g as f64) * factor) as u8,
                    (*b as f64 + (255.0 - *b as f64) * factor) as u8,
                )
            }
            other => *other,
        }
    }

    /// Darken an RGB color by a factor (0.0 to 1.0).
    pub fn darken(&self, amount: f64) -> Self {
        match self {
            Color::Rgb(r, g, b) => {
                let factor = 1.0 - normalized_color_amount(amount);
                Color::Rgb(
                    (*r as f64 * factor) as u8,
                    (*g as f64 * factor) as u8,
                    (*b as f64 * factor) as u8,
                )
            }
            other => *other,
        }
    }

    /// Create a grayscale color (0 = black, 255 = white).
    pub fn gray(level: u8) -> Self {
        Color::Rgb(level, level, level)
    }

    /// Create an RGB color.
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color::Rgb(r, g, b)
    }

    /// Create a color from ANSI 256 palette index.
    pub fn ansi(n: u8) -> Self {
        Color::Ansi256(n)
    }
}

fn normalized_color_amount(amount: f64) -> f64 {
    if amount.is_finite() {
        amount.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Border {
    None,
    Single,
    Double,
    Rounded,
    Thick,
}

impl Border {
    fn chars(&self) -> Option<BorderChars> {
        match self {
            Border::None => None,
            Border::Single => Some(BorderChars {
                tl: '┌',
                tr: '┐',
                bl: '└',
                br: '┘',
                h: '─',
                v: '│',
            }),
            Border::Double => Some(BorderChars {
                tl: '╔',
                tr: '╗',
                bl: '╚',
                br: '╝',
                h: '═',
                v: '║',
            }),
            Border::Rounded => Some(BorderChars {
                tl: '╭',
                tr: '╮',
                bl: '╰',
                br: '╯',
                h: '─',
                v: '│',
            }),
            Border::Thick => Some(BorderChars {
                tl: '┏',
                tr: '┓',
                bl: '┗',
                br: '┛',
                h: '━',
                v: '┃',
            }),
        }
    }
}

struct BorderChars {
    tl: char,
    tr: char,
    bl: char,
    br: char,
    h: char,
    v: char,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Style {
    fg: Option<Color>,
    bg: Option<Color>,
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
    dim: bool,
    reverse: bool,
    padding: [u16; 4],
    margin: [u16; 4],
    border: Border,
    border_fg: Option<Color>,
    width: Option<u16>,
    height: Option<u16>,
    align: Align,
}

// PLACEHOLDER_STYLE_IMPL

impl Default for Style {
    fn default() -> Self {
        Self::new()
    }
}

impl Style {
    pub fn new() -> Self {
        Self {
            fg: None,
            bg: None,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            dim: false,
            reverse: false,
            padding: [0; 4],
            margin: [0; 4],
            border: Border::None,
            border_fg: None,
            width: None,
            height: None,
            align: Align::Left,
        }
    }

    pub fn fg(mut self, color: Color) -> Self {
        self.fg = Some(color);
        self
    }
    pub fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }
    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }
    pub fn underline(mut self) -> Self {
        self.underline = true;
        self
    }
    pub fn strikethrough(mut self) -> Self {
        self.strikethrough = true;
        self
    }
    pub fn dim(mut self) -> Self {
        self.dim = true;
        self
    }
    pub fn reverse(mut self) -> Self {
        self.reverse = true;
        self
    }

    pub fn foreground(&self) -> Option<Color> {
        self.fg
    }

    pub fn background(&self) -> Option<Color> {
        self.bg
    }

    pub fn is_bold(&self) -> bool {
        self.bold
    }

    pub fn is_italic(&self) -> bool {
        self.italic
    }

    pub fn is_underline(&self) -> bool {
        self.underline
    }

    pub fn is_strikethrough(&self) -> bool {
        self.strikethrough
    }

    pub fn is_dim(&self) -> bool {
        self.dim
    }

    pub fn is_reverse(&self) -> bool {
        self.reverse
    }

    pub fn padding_top(mut self, n: u16) -> Self {
        self.padding[0] = n;
        self
    }
    pub fn padding_right(mut self, n: u16) -> Self {
        self.padding[1] = n;
        self
    }
    pub fn padding_bottom(mut self, n: u16) -> Self {
        self.padding[2] = n;
        self
    }
    pub fn padding_left(mut self, n: u16) -> Self {
        self.padding[3] = n;
        self
    }

    pub fn padding(mut self, vertical: u16, horizontal: u16) -> Self {
        self.padding = [vertical, horizontal, vertical, horizontal];
        self
    }

    pub fn margin_top(mut self, n: u16) -> Self {
        self.margin[0] = n;
        self
    }
    pub fn margin_right(mut self, n: u16) -> Self {
        self.margin[1] = n;
        self
    }
    pub fn margin_bottom(mut self, n: u16) -> Self {
        self.margin[2] = n;
        self
    }
    pub fn margin_left(mut self, n: u16) -> Self {
        self.margin[3] = n;
        self
    }

    pub fn margin(mut self, vertical: u16, horizontal: u16) -> Self {
        self.margin = [vertical, horizontal, vertical, horizontal];
        self
    }

    pub fn border(mut self, border: Border) -> Self {
        self.border = border;
        self
    }
    pub fn border_fg(mut self, color: Color) -> Self {
        self.border_fg = Some(color);
        self
    }

    pub fn width(mut self, w: u16) -> Self {
        self.width = Some(w);
        self
    }
    pub fn height(mut self, h: u16) -> Self {
        self.height = Some(h);
        self
    }
    pub fn align(mut self, a: Align) -> Self {
        self.align = a;
        self
    }

    pub fn render(&self, content: &str) -> String {
        let lines = split_lines_preserving_trailing_blank(content);
        let content_height = self.content_height(lines.len());
        let mut content_lines = lines
            .iter()
            .copied()
            .take(content_height)
            .collect::<Vec<_>>();
        while content_lines.len() < content_height {
            content_lines.push("");
        }

        let content_width = self.content_width(&content_lines);
        let pad_left = self.padding[3] as usize;
        let pad_right = self.padding[1] as usize;
        let inner_width = content_width + pad_left + pad_right;

        let border_chars = self.border.chars();

        let mut result = Vec::new();

        // Top margin
        for _ in 0..self.margin[0] {
            result.push(String::new());
        }

        let margin_left = " ".repeat(self.margin[3] as usize);
        let margin_right = " ".repeat(self.margin[1] as usize);

        // Top border
        if let Some(ref bc) = border_chars {
            let border_line = format!(
                "{}{}{}{}{}",
                margin_left,
                bc.tl,
                str::repeat(&bc.h.to_string(), inner_width),
                bc.tr,
                margin_right
            );
            result.push(self.apply_border_style(&border_line));
        }

        // Top padding
        for _ in 0..self.padding[0] {
            result.push(self.apply_padding_line_style(
                border_chars.as_ref(),
                &margin_left,
                inner_width,
                &margin_right,
            ));
        }

        // Content lines
        for line in &content_lines {
            let visible_width = visible_len(line);
            let aligned = self.align_text(line, visible_width, content_width);
            let padded = format!(
                "{}{}{}",
                " ".repeat(pad_left),
                aligned,
                " ".repeat(pad_right)
            );

            let styled_line = match &border_chars {
                Some(bc) => self.apply_bordered_content_style(
                    &margin_left,
                    bc.v,
                    &padded,
                    bc.v,
                    &margin_right,
                ),
                None => {
                    let full_line = format!("{}{}{}", margin_left, padded, margin_right);
                    self.apply_text_style(&full_line)
                }
            };

            result.push(styled_line);
        }

        // Bottom padding
        for _ in 0..self.padding[2] {
            result.push(self.apply_padding_line_style(
                border_chars.as_ref(),
                &margin_left,
                inner_width,
                &margin_right,
            ));
        }

        // Bottom border
        if let Some(ref bc) = border_chars {
            let border_line = format!(
                "{}{}{}{}{}",
                margin_left,
                bc.bl,
                str::repeat(&bc.h.to_string(), inner_width),
                bc.br,
                margin_right
            );
            result.push(self.apply_border_style(&border_line));
        }

        // Bottom margin
        for _ in 0..self.margin[2] {
            result.push(String::new());
        }

        if let Some(height) = self.height {
            let height = height as usize;
            result.truncate(height);
            while result.len() < height {
                result.push(String::new());
            }
        }

        result.join("\n")
    }

    fn content_width(&self, lines: &[&str]) -> usize {
        if let Some(w) = self.width {
            let border_cost = if self.border != Border::None { 2 } else { 0 };
            let pad_cost = self.padding[1] as usize + self.padding[3] as usize;
            let margin_cost = self.margin[1] as usize + self.margin[3] as usize;
            (w as usize).saturating_sub(border_cost + pad_cost + margin_cost)
        } else {
            lines.iter().map(|l| visible_len(l)).max().unwrap_or(0)
        }
    }

    fn content_height(&self, line_count: usize) -> usize {
        if let Some(h) = self.height {
            let border_cost = if self.border != Border::None { 2 } else { 0 };
            let pad_cost = self.padding[0] as usize + self.padding[2] as usize;
            let margin_cost = self.margin[0] as usize + self.margin[2] as usize;
            (h as usize).saturating_sub(border_cost + pad_cost + margin_cost)
        } else {
            line_count
        }
    }

    fn align_text(&self, text: &str, text_width: usize, target_width: usize) -> String {
        if text_width >= target_width {
            return truncate_visible(text, target_width);
        }
        match self.align {
            Align::Left => pad_visible(text, target_width),
            Align::Right => right_visible(text, target_width),
            Align::Center => center_visible(text, target_width),
        }
    }

    fn apply_bordered_content_style(
        &self,
        margin_left: &str,
        left_border: char,
        content: &str,
        right_border: char,
        margin_right: &str,
    ) -> String {
        let left_border = left_border.to_string();
        let right_border = right_border.to_string();

        format!(
            "{}{}{}{}{}",
            self.apply_text_style_nonempty(margin_left),
            self.apply_border_style(&left_border),
            self.apply_text_style(content),
            self.apply_border_style(&right_border),
            self.apply_text_style_nonempty(margin_right)
        )
    }

    fn apply_padding_line_style(
        &self,
        border_chars: Option<&BorderChars>,
        margin_left: &str,
        inner_width: usize,
        margin_right: &str,
    ) -> String {
        let content = " ".repeat(inner_width);
        match border_chars {
            Some(bc) => {
                self.apply_bordered_content_style(margin_left, bc.v, &content, bc.v, margin_right)
            }
            None => {
                let line = format!("{margin_left}{content}{margin_right}");
                self.apply_text_style(&line)
            }
        }
    }

    fn apply_text_style_nonempty(&self, text: &str) -> String {
        if text.is_empty() {
            String::new()
        } else {
            self.apply_text_style(text)
        }
    }

    fn apply_text_style(&self, text: &str) -> String {
        if text.is_empty() {
            return String::new();
        }

        let mut codes = Vec::new();
        if self.bold {
            codes.push("1".to_string());
        }
        if self.dim {
            codes.push("2".to_string());
        }
        if self.italic {
            codes.push("3".to_string());
        }
        if self.underline {
            codes.push("4".to_string());
        }
        if self.reverse {
            codes.push("7".to_string());
        }
        if self.strikethrough {
            codes.push("9".to_string());
        }
        if let Some(ref c) = self.fg {
            codes.push(c.fg_code());
        }
        if let Some(ref c) = self.bg {
            codes.push(c.bg_code());
        }

        if codes.is_empty() {
            text.to_string()
        } else {
            format!("\x1b[{}m{}\x1b[0m", codes.join(";"), text)
        }
    }

    fn apply_border_style(&self, text: &str) -> String {
        let mut codes = Vec::new();
        if let Some(c) = self.border_fg.or(self.fg) {
            codes.push(c.fg_code());
        }
        if let Some(c) = self.bg {
            codes.push(c.bg_code());
        }

        if codes.is_empty() {
            text.to_string()
        } else {
            format!("\x1b[{}m{}\x1b[0m", codes.join(";"), text)
        }
    }
}

/// Measure visible width of a string, ignoring ANSI escape sequences.
pub fn visible_len(s: &str) -> usize {
    let stripped = strip_ansi(s);
    UnicodeWidthStr::width(stripped.as_str())
}

/// Truncate a string to a visible width, preserving ANSI escape sequences.
pub fn truncate_visible(s: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if visible_len(s) <= width {
        return s.to_string();
    }
    if width == 1 {
        return "…".to_string();
    }

    let target = width - 1;
    let mut out = String::new();
    let mut used = 0;
    let mut saw_ansi = false;
    let mut index = 0usize;

    while index < s.len() {
        let ch = s[index..].chars().next().unwrap_or_default();
        if starts_ansi_csi_at(s, index) {
            saw_ansi = true;
            out.push(ch);
            index += ch.len_utf8();
            let Some(introducer) = s[index..].chars().next() else {
                break;
            };
            out.push(introducer);
            index += introducer.len_utf8();
            for next in s[index..].chars() {
                out.push(next);
                index += next.len_utf8();
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
            continue;
        }

        let Some((end, cw)) = next_display_cell_boundary(s, index) else {
            break;
        };
        if used + cw > target {
            break;
        }
        out.push_str(&s[index..end]);
        used += cw;
        index = end;
    }

    out.push('…');
    if saw_ansi {
        out.push_str("\x1b[0m");
    }
    out
}

/// Pad a string with spaces to `width` display columns, preserving ANSI codes.
///
/// If `s` is already at least `width` columns wide, it is returned unchanged.
pub fn pad_visible(s: &str, width: usize) -> String {
    let len = visible_len(s);
    if len >= width {
        s.to_string()
    } else {
        format!("{s}{}", " ".repeat(width - len))
    }
}

/// Repeat a character until the output fills `width` display columns.
///
/// If the character is zero-width, or if a wide character would straddle the
/// final column, spaces are used for the remaining columns.
pub fn repeat_visible_char(ch: char, width: usize) -> String {
    let char_width = UnicodeWidthChar::width(ch).unwrap_or(0);
    if width == 0 {
        return String::new();
    }
    if char_width == 0 {
        return " ".repeat(width);
    }

    let count = width / char_width;
    let remainder = width % char_width;
    format!("{}{}", ch.to_string().repeat(count), " ".repeat(remainder))
}

/// Repeat a string pattern until the output fills `width` display columns.
///
/// ANSI escape sequences are stripped from the pattern before repeating. If the
/// pattern is zero-width, or if a wide glyph would straddle the final column,
/// spaces are used for the remaining columns.
pub fn repeat_visible(pattern: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }

    let pattern = strip_ansi(pattern);
    if UnicodeWidthStr::width(pattern.as_str()) == 0 {
        return " ".repeat(width);
    }

    let mut out = String::new();
    let mut used = 0usize;
    while used < width {
        let pass_start = used;
        let mut index = 0usize;
        while used < width {
            let Some((end, cell_width)) = next_display_cell_boundary(&pattern, index) else {
                break;
            };
            let cell = &pattern[index..end];
            index = end;

            if cell_width == 0 {
                if !out.is_empty() {
                    out.push_str(cell);
                }
                continue;
            }
            if used + cell_width > width {
                out.push_str(&" ".repeat(width - used));
                return out;
            }

            out.push_str(cell);
            used += cell_width;
        }

        if used == pass_start {
            out.push_str(&" ".repeat(width - used));
            return out;
        }
    }

    out
}

/// Truncate a string to `width` display columns, then pad it to exactly `width`.
pub fn fit_visible(s: &str, width: usize) -> String {
    pad_visible(&truncate_visible(s, width), width)
}

/// Right-align a string in `width` display columns, truncating first if needed.
pub fn right_visible(s: &str, width: usize) -> String {
    let truncated = truncate_visible(s, width);
    let len = visible_len(&truncated);
    if len >= width {
        truncated
    } else {
        format!("{}{truncated}", " ".repeat(width - len))
    }
}

/// Center a string in `width` display columns, truncating first if needed.
pub fn center_visible(s: &str, width: usize) -> String {
    let truncated = truncate_visible(s, width);
    let len = visible_len(&truncated);
    if len >= width {
        return truncated;
    }
    let left = (width - len) / 2;
    let right = width - len - left;
    format!("{}{}{}", " ".repeat(left), truncated, " ".repeat(right))
}

pub(crate) fn next_display_cell_boundary(value: &str, start: usize) -> Option<(usize, usize)> {
    if start >= value.len() {
        return None;
    }

    let mut chars = value[start..].char_indices();
    let (_, ch) = chars.next()?;
    let width = UnicodeWidthChar::width(ch).unwrap_or(0);
    let mut end = start + ch.len_utf8();

    if width != 0 {
        for (offset, next) in chars {
            if starts_ansi_csi_at(value, start + offset) {
                break;
            }
            if UnicodeWidthChar::width(next).unwrap_or(0) != 0 {
                break;
            }
            end = start + offset + next.len_utf8();
        }
    }

    Some((end, width))
}

pub(crate) fn display_cell_char_span(chars: &[char], cursor: usize) -> (usize, usize) {
    if cursor >= chars.len() {
        return (cursor, cursor);
    }

    let mut start = cursor;
    while start > 0 && UnicodeWidthChar::width(chars[start]).unwrap_or(0) == 0 {
        start -= 1;
    }

    let mut end = start + 1;
    while end < chars.len() && UnicodeWidthChar::width(chars[end]).unwrap_or(0) == 0 {
        end += 1;
    }
    (start, end)
}

pub(crate) fn previous_display_cell_char_span(chars: &[char], cursor: usize) -> (usize, usize) {
    if cursor == 0 || chars.is_empty() {
        return (0, 0);
    }

    let start = cursor.saturating_sub(1).min(chars.len() - 1);
    display_cell_char_span(chars, start)
}

fn starts_ansi_csi_at(value: &str, index: usize) -> bool {
    value
        .get(index..)
        .is_some_and(|tail| tail.starts_with("\x1b["))
}

/// Return the substring spanning display columns `[from, to)`.
///
/// ANSI escape sequences are stripped before slicing. Wide glyphs count by
/// terminal display columns: a glyph that starts before `from` is dropped, while
/// one that starts before `to` is kept even if it extends past `to`.
pub fn slice_visible_cols(s: &str, from: usize, to: usize) -> String {
    if from >= to {
        return String::new();
    }

    let plain = strip_ansi(s);
    let mut col = 0usize;
    let mut out = String::new();
    let mut index = 0usize;

    while let Some((end, width)) = next_display_cell_boundary(&plain, index) {
        let cell = &plain[index..end];
        index = end;
        if width == 0 {
            if col >= to {
                break;
            }
            if col >= from {
                out.push_str(cell);
            }
            continue;
        }

        if col >= to {
            break;
        }

        if col >= from {
            out.push_str(cell);
        }
        col = col.saturating_add(width);
    }

    out
}

/// Wrap plain text on whitespace using display-column width.
///
/// This helper is intended for unstyled text. It preserves blank input lines;
/// use [`wrap_words_compact`] when transient panels should collapse empty rows.
pub fn wrap_words(text: &str, width: usize) -> Vec<String> {
    wrap_words_inner(text, width, false)
}

/// Split text rows while retaining a final empty row from a trailing newline.
pub(crate) fn split_lines_preserving_trailing_blank(text: &str) -> Vec<&str> {
    text.split('\n').collect()
}

/// Split non-empty text rows while retaining a final empty row from a trailing newline.
pub(crate) fn split_nonempty_lines_preserving_trailing_blank(text: &str) -> Vec<&str> {
    if text.is_empty() {
        Vec::new()
    } else {
        split_lines_preserving_trailing_blank(text)
    }
}

/// Wrap plain text on whitespace and drop blank input lines.
pub fn wrap_words_compact(text: &str, width: usize) -> Vec<String> {
    wrap_words_inner(text, width, true)
}

fn wrap_words_inner(text: &str, width: usize, compact: bool) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }

    let mut out = Vec::new();
    for para in split_lines_preserving_trailing_blank(text) {
        if para.trim().is_empty() {
            if !compact {
                out.push(String::new());
            }
            continue;
        }

        let mut line = String::new();
        for word in para.split_whitespace() {
            if line.is_empty() {
                line.push_str(word);
            } else if visible_len(&line) + 1 + visible_len(word) <= width {
                line.push(' ');
                line.push_str(word);
            } else {
                out.push(std::mem::take(&mut line));
                line.push_str(word);
            }

            while visible_len(&line) > width {
                let (head, rest) = split_visible_prefix(&line, width);
                if head.is_empty() {
                    break;
                }
                out.push(head);
                line = rest;
            }
        }

        if !line.is_empty() {
            out.push(line);
        }
    }

    if out.is_empty() {
        out.push(String::new());
    }
    out
}

fn split_visible_prefix(value: &str, width: usize) -> (String, String) {
    let mut head = String::new();
    let mut used = 0usize;
    let mut consumed_bytes = 0usize;
    let mut index = 0usize;

    while let Some((end, ch_width)) = next_display_cell_boundary(value, index) {
        let cell = &value[index..end];
        if ch_width == 0 {
            head.push_str(cell);
            consumed_bytes = end;
            index = end;
            continue;
        }

        if ch_width > width {
            if head.is_empty() {
                head = truncate_visible(cell, width);
                consumed_bytes = end;
            }
            break;
        }
        if used > 0 && used + ch_width > width {
            break;
        }

        head.push_str(cell);
        consumed_bytes = end;
        index = end;
        used += ch_width;
        if used >= width {
            break;
        }
    }

    (head, value[consumed_bytes..].to_string())
}

/// Strip ANSI escape sequences from a string.
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

use std::str;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_from_hex_valid() {
        assert_eq!(Color::from_hex("#ff0000"), Some(Color::Rgb(255, 0, 0)));
        assert_eq!(Color::from_hex("00ff00"), Some(Color::Rgb(0, 255, 0)));
        assert_eq!(Color::from_hex("#1e1e2e"), Some(Color::Rgb(30, 30, 46)));
    }

    #[test]
    fn color_from_hex_invalid() {
        assert_eq!(Color::from_hex(""), None);
        assert_eq!(Color::from_hex("#fff"), None);
        assert_eq!(Color::from_hex("zzzzzz"), None);
    }

    #[test]
    fn color_lighten() {
        let c = Color::Rgb(100, 100, 100);
        let lighter = c.lighten(0.5);
        match lighter {
            Color::Rgb(r, g, b) => {
                assert!(r > 100);
                assert!(g > 100);
                assert!(b > 100);
            }
            _ => panic!("expected Rgb"),
        }
    }

    #[test]
    fn color_darken() {
        let c = Color::Rgb(200, 200, 200);
        let darker = c.darken(0.5);
        match darker {
            Color::Rgb(r, g, b) => {
                assert!(r < 200);
                assert!(g < 200);
                assert!(b < 200);
            }
            _ => panic!("expected Rgb"),
        }
    }

    #[test]
    fn color_lighten_non_rgb_is_noop() {
        let c = Color::Red;
        assert_eq!(c.lighten(0.5), Color::Red);
    }

    #[test]
    fn color_lighten_and_darken_ignore_non_finite_amounts() {
        let c = Color::Rgb(120, 80, 40);

        assert_eq!(c.lighten(f64::NAN), c);
        assert_eq!(c.lighten(f64::INFINITY), c);
        assert_eq!(c.lighten(f64::NEG_INFINITY), c);
        assert_eq!(c.darken(f64::NAN), c);
        assert_eq!(c.darken(f64::INFINITY), c);
        assert_eq!(c.darken(f64::NEG_INFINITY), c);
    }

    #[test]
    fn visible_len_plain() {
        assert_eq!(visible_len("hello"), 5);
        assert_eq!(visible_len(""), 0);
    }

    #[test]
    fn visible_len_with_ansi() {
        assert_eq!(visible_len("\x1b[31mhello\x1b[0m"), 5);
        assert_eq!(visible_len("\x1b[1;32mtest\x1b[0m"), 4);
    }

    #[test]
    fn strip_ansi_removes_codes() {
        assert_eq!(strip_ansi("\x1b[31mred\x1b[0m"), "red");
        assert_eq!(strip_ansi("plain"), "plain");
    }

    #[test]
    fn color_gray() {
        assert_eq!(Color::gray(128), Color::Rgb(128, 128, 128));
        assert_eq!(Color::gray(0), Color::Rgb(0, 0, 0));
        assert_eq!(Color::gray(255), Color::Rgb(255, 255, 255));
    }

    #[test]
    fn color_rgb_constructor() {
        assert_eq!(Color::rgb(10, 20, 30), Color::Rgb(10, 20, 30));
    }

    #[test]
    fn color_ansi_constructor() {
        assert_eq!(Color::ansi(196), Color::Ansi256(196));
    }

    #[test]
    fn pads_visible_width_with_ansi() {
        let styled = Style::new().fg(Color::Cyan).render("ok");
        let padded = pad_visible(&styled, 6);

        assert_eq!(visible_len(&padded), 6);
        assert!(padded.starts_with("\x1b[36m"));
    }

    #[test]
    fn repeats_visible_char_to_display_width() {
        assert_eq!(repeat_visible_char('·', 4), "····");
        assert_eq!(repeat_visible_char('界', 3), "界 ");
        assert_eq!(repeat_visible_char('\u{301}', 3), "   ");
        assert_eq!(visible_len(&repeat_visible_char('界', 5)), 5);
    }

    #[test]
    fn repeats_visible_pattern_to_display_width() {
        assert_eq!(repeat_visible("ab", 5), "ababa");
        assert_eq!(repeat_visible("界", 5), "界界 ");
        assert_eq!(repeat_visible("\u{301}", 3), "   ");
        assert_eq!(repeat_visible("\x1b[31m-\x1b[0m", 3), "---");
        assert_eq!(visible_len(&repeat_visible("界a", 6)), 6);
    }

    #[test]
    fn fits_visible_width_by_truncating_then_padding() {
        let out = fit_visible("一二三四五", 6);

        assert!(visible_len(&out) <= 6, "{out:?}");
        assert!(out.ends_with('…') || out.ends_with(' '));
    }

    #[test]
    fn truncate_visible_preserves_ansi_when_truncating() {
        let styled = Style::new().fg(Color::Red).render("abcdef");
        let out = truncate_visible(&styled, 4);

        assert_eq!(visible_len(&out), 4);
        assert_eq!(strip_ansi(&out), "abc…");
        assert!(out.starts_with("\x1b[31m"));
        assert!(out.ends_with("\x1b[0m"));
    }

    #[test]
    fn truncate_visible_skips_ansi_reset_between_segments() {
        let styled_prefix = Style::new().fg(Color::Green).render("ok");
        let out = truncate_visible(&format!("{styled_prefix}abcdef"), 5);

        assert_eq!(visible_len(&out), 5);
        assert_eq!(strip_ansi(&out), "okab…");
        assert!(out.contains("\x1b[0mab"));
    }

    #[test]
    fn truncate_visible_keeps_zero_width_marks_with_base_glyph() {
        let out = truncate_visible("e\u{301}xyz", 2);

        assert_eq!(out, "e\u{301}…");
        assert_eq!(visible_len(&out), 2);
    }

    #[test]
    fn render_border_with_padding() {
        let out = Style::new()
            .border(Border::Rounded)
            .padding(1, 1)
            .render("x");

        assert_eq!(
            out.lines().collect::<Vec<_>>(),
            vec!["╭───╮", "│   │", "│ x │", "│   │", "╰───╯"]
        );
    }

    #[test]
    fn render_border_preserves_trailing_blank_content_row() {
        let out = Style::new().border(Border::Rounded).render("x\n");

        assert_eq!(
            out.lines().collect::<Vec<_>>(),
            vec!["╭─╮", "│x│", "│ │", "╰─╯"]
        );
    }

    #[test]
    fn render_bordered_content_uses_border_style_for_vertical_edges() {
        let out = Style::new()
            .fg(Color::Red)
            .border(Border::Rounded)
            .border_fg(Color::Cyan)
            .render("x");
        let lines = out.lines().collect::<Vec<_>>();

        assert_eq!(lines[0], "\x1b[36m╭─╮\x1b[0m");
        assert_eq!(lines[1], "\x1b[36m│\x1b[0m\x1b[31mx\x1b[0m\x1b[36m│\x1b[0m");
        assert_eq!(lines[2], "\x1b[36m╰─╯\x1b[0m");
        assert_eq!(strip_ansi(lines[1]), "│x│");
        assert_eq!(visible_len(lines[1]), 3);
    }

    #[test]
    fn render_bordered_cells_inherit_background_style() {
        let out = Style::new()
            .fg(Color::Red)
            .bg(Color::Blue)
            .border(Border::Rounded)
            .border_fg(Color::Cyan)
            .render("x");
        let lines = out.lines().collect::<Vec<_>>();

        assert_eq!(lines[0], "\x1b[36;44m╭─╮\x1b[0m");
        assert_eq!(
            lines[1],
            "\x1b[36;44m│\x1b[0m\x1b[31;44mx\x1b[0m\x1b[36;44m│\x1b[0m"
        );
        assert_eq!(lines[2], "\x1b[36;44m╰─╯\x1b[0m");
        assert!(lines.iter().all(|line| visible_len(line) == 3));
    }

    #[test]
    fn render_bordered_padding_uses_text_style_inside_vertical_edges() {
        let out = Style::new()
            .bg(Color::Blue)
            .border(Border::Rounded)
            .border_fg(Color::Cyan)
            .padding(1, 1)
            .render("x");
        let lines = out.lines().collect::<Vec<_>>();

        assert_eq!(lines[0], "\x1b[36;44m╭───╮\x1b[0m");
        assert_eq!(
            lines[1],
            "\x1b[36;44m│\x1b[0m\x1b[44m   \x1b[0m\x1b[36;44m│\x1b[0m"
        );
        assert_eq!(strip_ansi(lines[1]), "│   │");
        assert_eq!(visible_len(lines[1]), 5);
    }

    #[test]
    fn render_unbordered_padding_uses_text_style() {
        let out = Style::new()
            .bg(Color::Blue)
            .border_fg(Color::Cyan)
            .padding(1, 0)
            .render("x");
        let lines = out.lines().collect::<Vec<_>>();

        assert_eq!(
            lines,
            vec!["\x1b[44m \x1b[0m", "\x1b[44mx\x1b[0m", "\x1b[44m \x1b[0m"]
        );
        assert!(lines.iter().all(|line| visible_len(line) == 1));
    }

    #[test]
    fn render_empty_styled_text_stays_empty() {
        let out = Style::new().fg(Color::Red).bold().render("");

        assert_eq!(out, "");
    }

    #[test]
    fn centers_visible_width_with_ansi_and_cjk() {
        let styled = Style::new().fg(Color::Green).render("中");
        let centered = center_visible(&styled, 6);

        assert_eq!(visible_len(&centered), 6);
        assert!(strip_ansi(&centered).starts_with("  中"));
    }

    #[test]
    fn right_aligns_visible_width_with_ansi_and_cjk() {
        let styled = Style::new().fg(Color::Green).render("中");
        let right = right_visible(&styled, 6);

        assert_eq!(visible_len(&right), 6);
        assert!(strip_ansi(&right).starts_with("    中"));
    }

    #[test]
    fn render_aligns_text_by_visible_width() {
        let styled = Style::new().fg(Color::Red).render("中");
        let right = Style::new().width(6).align(Align::Right).render(&styled);
        let center = Style::new().width(7).align(Align::Center).render("中");

        assert_eq!(visible_len(&right), 6);
        assert_eq!(strip_ansi(&right), "    中");
        assert!(right.ends_with("\x1b[0m"));
        assert_eq!(visible_len(&center), 7);
        assert_eq!(center, "  中   ");
    }

    #[test]
    fn render_width_truncates_long_content_by_visible_width() {
        let out = Style::new().width(4).render("abcdef");
        let cjk = Style::new().width(5).render("中文测试");

        assert_eq!(visible_len(&out), 4);
        assert_eq!(out, "abc…");
        assert_eq!(visible_len(&cjk), 5);
        assert_eq!(cjk, "中文…");
    }

    #[test]
    fn render_bordered_width_truncates_inner_content() {
        let out = Style::new()
            .width(6)
            .border(Border::Rounded)
            .render("abcdef");
        let lines = out.lines().collect::<Vec<_>>();

        assert_eq!(lines, vec!["╭────╮", "│abc…│", "╰────╯"]);
        assert!(lines.iter().all(|line| visible_len(line) == 6));
    }

    #[test]
    fn render_width_includes_right_margin() {
        let out = Style::new()
            .width(8)
            .margin_left(1)
            .margin_right(2)
            .render("abcd");

        assert_eq!(visible_len(&out), 8);
        assert_eq!(out, " abcd   ");
    }

    #[test]
    fn render_bordered_width_includes_right_margin() {
        let out = Style::new()
            .width(8)
            .margin_left(1)
            .margin_right(1)
            .border(Border::Rounded)
            .render("abcdef");
        let lines = out.lines().collect::<Vec<_>>();

        assert_eq!(lines, vec![" ╭────╮ ", " │abc…│ ", " ╰────╯ "]);
        assert!(lines.iter().all(|line| visible_len(line) == 8));
    }

    #[test]
    fn render_height_truncates_and_pads_content_rows() {
        let truncated = Style::new().width(4).height(2).render("one\ntwo\nthree");
        let padded = Style::new().width(4).height(3).render("one");

        assert_eq!(truncated.lines().collect::<Vec<_>>(), vec!["one ", "two "]);
        assert_eq!(
            padded.lines().collect::<Vec<_>>(),
            vec!["one ", "    ", "    "]
        );
    }

    #[test]
    fn render_bordered_height_limits_inner_content() {
        let out = Style::new()
            .width(6)
            .height(3)
            .border(Border::Rounded)
            .render("one\ntwo");
        let lines = out.lines().collect::<Vec<_>>();

        assert_eq!(lines, vec!["╭────╮", "│one │", "╰────╯"]);
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn render_height_includes_vertical_spacing() {
        let out = Style::new()
            .width(6)
            .height(6)
            .margin_top(1)
            .margin_bottom(1)
            .padding(1, 1)
            .render("x\ny");
        let lines = out.split('\n').collect::<Vec<_>>();

        assert_eq!(lines, vec!["", "      ", " x    ", " y    ", "      ", ""]);
        assert_eq!(lines.len(), 6);
    }

    #[test]
    fn slices_visible_columns_for_ascii_cjk_and_ansi() {
        assert_eq!(slice_visible_cols("hello", 1, 4), "ell");
        assert_eq!(slice_visible_cols("hello", 0, 100), "hello");
        assert_eq!(slice_visible_cols("你好", 0, 2), "你");
        assert_eq!(slice_visible_cols("你好", 2, 4), "好");

        let styled = Style::new().fg(Color::Red).render("hello");
        assert_eq!(slice_visible_cols(&styled, 1, 4), "ell");
    }

    #[test]
    fn slices_visible_columns_drop_glyph_straddling_start() {
        assert_eq!(slice_visible_cols("你好", 1, 4), "好");
        assert_eq!(slice_visible_cols("你好", 0, 3), "你好");
        assert_eq!(slice_visible_cols("hello", 3, 3), "");
    }

    #[test]
    fn slices_visible_columns_keep_zero_width_marks_with_base_glyph() {
        assert_eq!(slice_visible_cols("e\u{301}x", 0, 1), "e\u{301}");
        assert_eq!(slice_visible_cols("e\u{301}x", 1, 2), "x");
    }

    #[test]
    fn display_cell_char_span_keeps_zero_width_marks_with_base_glyph() {
        let chars = "e\u{301}x".chars().collect::<Vec<_>>();

        assert_eq!(display_cell_char_span(&chars, 0), (0, 2));
        assert_eq!(display_cell_char_span(&chars, 1), (0, 2));
        assert_eq!(display_cell_char_span(&chars, 2), (2, 3));
        assert_eq!(display_cell_char_span(&chars, 3), (3, 3));
    }

    #[test]
    fn previous_display_cell_char_span_keeps_zero_width_marks_with_base_glyph() {
        let chars = "e\u{301}x".chars().collect::<Vec<_>>();

        assert_eq!(previous_display_cell_char_span(&chars, 0), (0, 0));
        assert_eq!(previous_display_cell_char_span(&chars, 2), (0, 2));
        assert_eq!(previous_display_cell_char_span(&chars, 3), (2, 3));
    }

    #[test]
    fn wraps_words_on_display_columns() {
        let lines = wrap_words("the quick brown fox jumps", 9);

        assert!(lines.iter().all(|line| visible_len(line) <= 9));
        assert_eq!(
            lines.join(" ").split_whitespace().collect::<Vec<_>>(),
            vec!["the", "quick", "brown", "fox", "jumps"]
        );
    }

    #[test]
    fn wrap_words_preserves_blank_lines() {
        let lines = wrap_words("alpha\n\nbeta", 40);

        assert_eq!(lines, vec!["alpha", "", "beta"]);
    }

    #[test]
    fn wrap_words_preserves_trailing_blank_lines() {
        let lines = wrap_words("alpha\n", 40);

        assert_eq!(lines, vec!["alpha", ""]);
    }

    #[test]
    fn split_lines_preserves_trailing_blank_line() {
        assert_eq!(
            split_lines_preserving_trailing_blank("alpha\n"),
            vec!["alpha", ""]
        );
        assert_eq!(split_lines_preserving_trailing_blank(""), vec![""]);
    }

    #[test]
    fn split_nonempty_lines_preserves_trailing_blank_line() {
        assert_eq!(
            split_nonempty_lines_preserving_trailing_blank(""),
            Vec::<&str>::new()
        );
        assert_eq!(
            split_nonempty_lines_preserving_trailing_blank("alpha\n"),
            vec!["alpha", ""]
        );
    }

    #[test]
    fn compact_wrap_words_drops_blank_lines() {
        let lines = wrap_words_compact("alpha\n\nbeta", 40);

        assert_eq!(lines, vec!["alpha", "beta"]);
    }

    #[test]
    fn wrap_words_hard_breaks_wide_tokens_by_columns() {
        let lines = wrap_words_compact("中文测试内容", 8);

        assert!(lines.iter().all(|line| visible_len(line) <= 8));
        assert_eq!(lines.concat(), "中文测试内容");
    }

    #[test]
    fn wrap_words_hard_break_keeps_zero_width_marks_with_base_glyph() {
        let lines = wrap_words_compact("e\u{301}e\u{301}e", 1);

        assert!(lines.iter().all(|line| visible_len(line) <= 1));
        assert_eq!(lines, vec!["e\u{301}", "e\u{301}", "e"]);
    }

    #[test]
    fn wrap_words_hard_break_packs_zero_width_marks_by_display_width() {
        let lines = wrap_words_compact("e\u{301}e\u{301}e", 2);

        assert!(lines.iter().all(|line| visible_len(line) <= 2));
        assert_eq!(lines, vec!["e\u{301}e\u{301}", "e"]);
    }

    #[test]
    fn wrap_words_clips_wide_glyphs_at_single_column_width() {
        let lines = wrap_words_compact("中文", 1);

        assert!(lines.iter().all(|line| visible_len(line) <= 1));
    }
}
