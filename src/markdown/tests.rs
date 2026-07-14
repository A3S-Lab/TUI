use super::ansi::ansi_segments;
use super::*;
use crate::element::Element;
use crate::style::{strip_ansi, visible_len};

fn osc8_targets(rendered: &str) -> Vec<&str> {
    let mut targets = Vec::new();
    let mut rest = rendered;
    while let Some(start) = rest.find("\x1b]8;;") {
        rest = &rest[start + "\x1b]8;;".len()..];
        let Some(end) = rest.find("\x1b\\") else {
            break;
        };
        let target = &rest[..end];
        if !target.is_empty() {
            targets.push(target);
        }
        rest = &rest[end + 2..];
    }
    targets
}

#[test]
fn render_plain_text() {
    let md = Markdown::new();
    let output = md.render("hello world");
    let plain = strip_ansi(&output);
    assert!(plain.contains("hello world"));
}

#[test]
fn trailing_background_tracks_the_final_visible_ansi_cell() {
    let red = Style::new().bg(Color::Rgb(80, 31, 27)).render("deleted ");
    let plain_tail = format!("{red}plain");

    assert_eq!(trailing_ansi_background(&red), Some(Color::Rgb(80, 31, 27)));
    assert_eq!(trailing_ansi_background(&plain_tail), None);
    assert_eq!(trailing_ansi_background("plain"), None);
}

#[test]
fn render_heading() {
    let md = Markdown::new();
    let output = md.render("# Title");
    let plain = strip_ansi(&output);
    assert_eq!(plain.lines().next(), Some("# Title"));
}

#[test]
fn heading_respects_configured_width() {
    let md = Markdown::new().with_width(8);
    let output = md.render("# abcdefghijklmnopqrstuvwxyz");
    let heading = output.lines().next().unwrap();

    assert!(visible_len(heading) <= 8, "{heading:?}");
}

#[test]
fn section_heading_has_one_blank_row_before_and_after() {
    let output = strip_ansi(
        &Markdown::new().render("Previous section body.\n\n## Next section\n\nNext section body."),
    );

    assert_eq!(
        output.lines().collect::<Vec<_>>(),
        vec![
            "Previous section body.",
            "",
            "## Next section",
            "",
            "Next section body."
        ]
    );
}

#[test]
fn render_code_block() {
    let md = Markdown::new();
    let output = md.render("```\nlet x = 1;\n```");
    let plain = strip_ansi(&output);
    assert!(plain.contains("let x = 1"));
}

#[test]
fn render_code_block_has_no_frame_or_gutter() {
    let md = Markdown::new();
    let output = md.render("```rust\nlet x = 1;\n```");
    let plain = strip_ansi(&output);
    let rows = plain.lines().collect::<Vec<_>>();

    assert_eq!(rows, vec!["let x = 1;"]);
    assert!(!plain.contains(['┌', '│', '└']));
}

#[test]
fn code_block_body_lines_do_not_wrap_or_clip() {
    let md = Markdown::new().with_width(8);
    let output = md.render("```\nabcdefghijklmnopqrstuvwxyz\n```");
    let plain = strip_ansi(&output);

    assert_eq!(
        plain.lines().collect::<Vec<_>>(),
        vec!["abcdefghijklmnopqrstuvwxyz"]
    );
    assert!(visible_len(plain.lines().next().unwrap()) > 8);
}

#[test]
fn unterminated_code_block_has_no_temporary_footer() {
    let output = strip_ansi(&Markdown::new().render("```rust\nlet x = 1;\n"));

    assert_eq!(output.lines().collect::<Vec<_>>(), vec!["let x = 1;"]);
    assert!(!output.contains(['┌', '│', '└', '─']));
}

#[test]
fn render_metadata_marks_only_code_rows_as_non_wrapping() {
    let rendered = Markdown::new().render_with_metadata(
        "before\n\n```rust\nlet extraordinarily_long_identifier = 1;\n```\n\nafter",
    );
    let code_row = rendered
        .as_str()
        .lines()
        .position(|line| strip_ansi(line).contains("extraordinarily_long_identifier"))
        .expect("rendered code row");

    assert_eq!(rendered.non_wrapping_line_indices(), &[code_row]);
}

#[test]
fn render_metadata_tracks_table_source_through_blockquote_prefixes() {
    let rendered = Markdown::new().with_width(80).render_with_metadata(
        "Before\n\n> | Key | Value |\n> | --- | --- |\n> | one | two |\n\nAfter",
    );
    let rendered_lines = rendered.as_str().lines().collect::<Vec<_>>();

    assert_eq!(rendered.line_metadata().len(), rendered_lines.len());
    let table_rows = rendered
        .line_metadata()
        .iter()
        .enumerate()
        .filter(|(_, metadata)| metadata.is_table())
        .collect::<Vec<_>>();
    assert!(!table_rows.is_empty());
    assert!(table_rows
        .iter()
        .all(|(_, metadata)| { metadata.source() == Some(SourceLineRange { start: 3, end: 5 }) }));
    assert!(table_rows
        .iter()
        .all(|(index, _)| strip_ansi(rendered_lines[*index]).starts_with("│ ")));
}

#[cfg(feature = "syntax-highlighting")]
#[test]
fn render_code_block_without_themes_falls_back_to_plain_text() {
    let md = Markdown {
        width: 80,
        syntax_set: Arc::new(SyntaxSet::load_defaults_newlines()),
        theme_set: Arc::new(ThemeSet::new()),
        theme_name: "missing-theme".to_string(),
    };

    let output = md.render("```rust\nlet x = 1;\n```");
    let plain = strip_ansi(&output);

    assert!(plain.contains("let x = 1"));
}

#[cfg(feature = "syntax-highlighting")]
#[test]
fn markdown_instances_share_default_syntax_assets() {
    let first = Markdown::new();
    let themed = Markdown::new().with_theme("InspiredGitHub");

    assert!(Arc::ptr_eq(&first.syntax_set, &themed.syntax_set));
    assert!(Arc::ptr_eq(&first.theme_set, &themed.theme_set));
    assert_eq!(themed.theme_name, "InspiredGitHub");
}

#[cfg(feature = "syntax-highlighting")]
#[test]
fn unknown_fence_language_stays_completely_unstyled() {
    let output = Markdown::new().render("```definitely-not-a-language\nlet value = 42;\n```");

    assert_eq!(strip_ansi(&output), "let value = 42;\n");
    assert!(!output.contains("\x1b[38;2;"), "{output:?}");
}

#[cfg(feature = "syntax-highlighting")]
#[test]
fn syntax_highlighting_obeys_codex_size_and_line_guardrails() {
    let md = Markdown::new();
    let oversized = "x".repeat(MAX_HIGHLIGHT_BYTES + 1);
    assert_eq!(md.highlight_code(&oversized, "rust"), oversized);

    let too_many_lines = "let value = 1;\n".repeat(MAX_HIGHLIGHT_LINES + 1);
    assert_eq!(md.highlight_code(&too_many_lines, "rust"), too_many_lines);
}

#[cfg(feature = "syntax-highlighting")]
#[test]
fn syntax_lookup_resolves_common_codex_aliases() {
    let syntax_set = SyntaxSet::load_defaults_newlines();
    for alias in [
        "csharp", "c-sharp", "cppm", "CPPM", "cxxm", "CxXm", "ixx", "IXX", "golang", "python3",
        "shell",
    ] {
        assert!(
            find_syntax(&syntax_set, alias).is_some(),
            "alias {alias:?} did not resolve"
        );
    }
    assert!(find_syntax(&syntax_set, "xyzlang").is_none());
}

#[cfg(feature = "syntax-highlighting")]
#[test]
fn highlighted_code_normalizes_crlf_without_stray_carriage_returns() {
    let highlighted =
        Markdown::new().highlight_code("fn main() {\r\n    println!(\"hello\");\r\n}\r\n", "rust");
    let plain = strip_ansi(&highlighted);

    assert!(!plain.contains('\r'), "{plain:?}");
    assert_eq!(plain, "fn main() {\n    println!(\"hello\");\n}\n");
}

#[cfg(feature = "syntax-highlighting")]
#[test]
fn code_tokens_keep_distinct_foreground_colors() {
    let output = Markdown::new()
        .render("```rust\nfn greet() { let answer = format_value(\"hello\", 42); // note\n}\n```");
    let colors = ["fn", "greet", "hello", "42", "//"].map(|token| {
        foreground_rgb_before(&output, token)
            .unwrap_or_else(|| panic!("missing foreground for {token:?} in {output:?}"))
    });

    for (index, color) in colors.iter().enumerate() {
        assert!(
            !colors[..index].contains(color),
            "token colors must remain distinct: {colors:?}"
        );
    }
}

#[cfg(feature = "syntax-highlighting")]
fn foreground_rgb_before(rendered: &str, token: &str) -> Option<(u8, u8, u8)> {
    let token_start = rendered.find(token)?;
    let prefix = &rendered[..token_start];
    let marker = "\x1b[38;2;";
    let start = prefix.rfind(marker)? + marker.len();
    let end = rendered[start..].find('m')? + start;
    let mut channels = rendered[start..end]
        .split(';')
        .filter_map(|part| part.parse::<u8>().ok());
    Some((channels.next()?, channels.next()?, channels.next()?))
}

#[test]
fn render_bold() {
    let md = Markdown::new();
    let output = md.render("**bold text**");
    assert!(output.contains("bold text"));
}

#[test]
fn wrapped_inline_style_is_self_contained_on_every_rendered_row() {
    let md = Markdown::new().with_width(4);
    let output = md.render("**abcdefgh**");
    let rows = output.lines().collect::<Vec<_>>();

    assert_eq!(
        rows.iter().map(|row| strip_ansi(row)).collect::<Vec<_>>(),
        ["abcd", "efgh"]
    );
    assert!(rows.iter().all(|row| row.starts_with("\x1b[1m")));

    let Element::Box(column) = md.render_element::<()>("**abcdefgh**") else {
        panic!("expected Markdown column");
    };
    assert_eq!(column.children.len(), 2);
    for child in column.children {
        let Element::Text(text) = child else {
            panic!("expected one styled text segment per wrapped row");
        };
        assert!(text.style.bold);
    }
}

#[test]
fn soft_break_preserves_source_line_boundary() {
    let output = strip_ansi(&Markdown::new().render("alpha\nbeta"));

    assert_eq!(output.lines().collect::<Vec<_>>(), vec!["alpha", "beta"]);
}

#[test]
fn hard_break_preserves_source_line_boundary() {
    let output = strip_ansi(&Markdown::new().render("alpha  \nbeta"));

    assert_eq!(output.lines().collect::<Vec<_>>(), vec!["alpha", "beta"]);
}

#[test]
fn render_list() {
    let md = Markdown::new();
    let output = md.render("- item 1\n- item 2");
    let plain = strip_ansi(&output);
    assert_eq!(
        plain.lines().collect::<Vec<_>>(),
        vec!["- item 1", "- item 2"]
    );
}

#[test]
fn unordered_list_wraps_with_hanging_indent() {
    let output = strip_ansi(
        &Markdown::new()
            .with_width(18)
            .render("- alpha beta gamma delta epsilon"),
    );
    let lines = output.lines().collect::<Vec<_>>();

    assert!(lines[0].starts_with("- "), "{output}");
    assert!(lines[1].starts_with("  "), "{output}");
}

#[test]
fn list_items_respect_configured_width() {
    let md = Markdown::new().with_width(8);
    let output = md.render("- abcdefghijklmnopqrstuvwxyz");

    for line in output.lines() {
        assert!(visible_len(line) <= 8, "{line:?}");
    }
}

#[test]
fn narrow_list_preserves_the_complete_ascii_body() {
    let output = strip_ansi(&Markdown::new().with_width(4).render("- abcdefgh"));
    let body = output
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>();

    assert_eq!(body, "abcdefgh", "{output}");
    assert!(output.lines().all(|line| visible_len(line) <= 4));
}

#[test]
fn nested_paragraphs_respect_configured_width() {
    let md = Markdown::new().with_width(2);
    let output = md.render("- item\n\n  continuation text");

    for line in output.lines() {
        assert!(visible_len(line) <= 2, "{line:?}");
    }
}

#[test]
fn table_respects_configured_width() {
    let md = Markdown::new().with_width(12);
    let output =
        md.render("| Name | Value |\n| --- | --- |\n| alpha | abcdefghijklmnopqrstuvwxyz |");

    for line in output.lines() {
        assert!(visible_len(line) <= 12, "{line:?}");
    }
}

#[test]
fn narrow_record_table_preserves_labels_and_values_without_overflow() {
    let output = strip_ansi(
        &Markdown::new()
            .with_width(4)
            .render("| Long | B |\n| --- | --- |\n| x | y |"),
    );
    let content = output
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>();

    assert_eq!(content, "LongxBy", "{output}");
    assert!(output.lines().all(|line| visible_len(line) <= 4));
}

#[test]
fn rendered_markdown_rows_respect_every_narrow_width_except_code() {
    let source = "# Heading\n\n\
                  **abcdefgh** and [docs](https://example.com)\n\n\
                  - abcdefghijkl\n\n\
                  > quoted words that wrap\n\n\
                  | Long | B |\n| --- | --- |\n| x | y |\n\n\
                  ```text\ncode row deliberately stays long\n```";

    for width in 1..=32 {
        let rendered = Markdown::new()
            .with_width(width)
            .render_with_metadata(source);
        let rows = rendered.as_str().lines().collect::<Vec<_>>();
        assert_eq!(rows.len(), rendered.line_metadata().len());
        for (row, metadata) in rows.into_iter().zip(rendered.line_metadata()) {
            if !metadata.is_non_wrapping() {
                assert!(
                    visible_len(row) <= width,
                    "width {width} overflowed with {row:?}"
                );
            }
        }
    }
}

#[test]
fn table_honors_left_center_and_right_alignment() {
    let output = strip_ansi(
        &Markdown::new()
            .with_width(80)
            .render("| Left | Mid | Right |\n| :--- | :---: | ---: |\n| x | x | x |"),
    );
    let body = output
        .lines()
        .find(|line| line.matches('x').count() == 3)
        .expect("table body row");
    let positions = body
        .match_indices('x')
        .map(|(index, _)| index)
        .collect::<Vec<_>>();

    assert_eq!(positions, vec![1, 10, 20]);
}

#[test]
fn narrow_header_only_table_preserves_all_headers_and_alignment() {
    let output = strip_ansi(
        &Markdown::new()
            .with_width(12)
            .render("| First heading | Second heading |\n| :--- | ---: |"),
    );
    let flattened = output
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();

    assert!(flattened.contains("Firstheading"), "{output}");
    assert!(flattened.contains("Secondheading"), "{output}");
    assert!(!output.contains('|'), "{output}");
    assert!(!output.contains(":---"), "{output}");
    assert!(!output.contains("---:"), "{output}");
    assert!(
        output.lines().all(|line| visible_len(line) <= 12),
        "{output}"
    );
}

#[test]
fn bare_links_are_not_duplicated_or_extended_into_following_prose() {
    let url = "https://github.com/A3S-Lab/Code/actions/runs/29246228334";
    let source = format!("Started: {url}\n({url})。继续。 ");
    let output = Markdown::new().with_width(52).render(&source);
    let plain = strip_ansi(&output);
    let flattened = plain
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();

    assert_eq!(
        flattened,
        source
            .chars()
            .filter(|ch| !ch.is_whitespace())
            .collect::<String>()
    );
    assert_eq!(flattened.matches(url).count(), 2, "{plain}");
    let targets = osc8_targets(&output);
    assert!(targets.len() >= 2, "{output:?}");
    assert!(targets.iter().all(|target| *target == url), "{targets:?}");
}

#[test]
fn moderately_narrow_table_wraps_columns_before_falling_back_to_records() {
    let output = strip_ansi(&Markdown::new().with_width(28).render(
        "| Tool | Description |\n| --- | --- |\n| bash | a detailed explanation with several words |",
    ));
    let flattened = output
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();

    assert!(output.contains('━'), "{output}");
    assert!(
        !output.lines().any(|line| line.starts_with("Tool:")),
        "{output}"
    );
    assert!(
        flattened.contains("adetailedexplanationwithseveralwords"),
        "{output}"
    );
    assert!(
        output.lines().all(|line| visible_len(line) <= 28),
        "{output}"
    );
}

#[test]
fn extremely_narrow_many_column_table_uses_records() {
    let output = strip_ansi(
        &Markdown::new()
            .with_width(18)
            .render("| A | B | C | D |\n| --- | --- | --- | --- |\n| one | two | three | four |"),
    );

    assert!(!output.contains('━'), "{output}");
    for expected in ["A: one", "B: two", "C: three", "D: four"] {
        assert!(
            output.contains(expected),
            "missing {expected:?} in {output}"
        );
    }
    assert!(
        output.lines().all(|line| visible_len(line) <= 18),
        "{output}"
    );
}

#[test]
fn table_cells_render_inline_styles_and_resolved_links() {
    let output = Markdown::new().with_width(120).render(
        "| Strong | Code | Link |\n\
         | --- | --- | --- |\n\
         | **bold** | `value` | [Docs][docs] |\n\n\
         [docs]: https://example.com/reference",
    );
    let plain = strip_ansi(&output);

    assert!(output.contains("\x1b[1m"), "{output:?}");
    assert!(output.contains("48;5;236"), "{output:?}");
    assert!(plain.contains("Docs"), "{plain}");
    assert_eq!(osc8_targets(&output), vec!["https://example.com/reference"]);
    assert!(!plain.contains("[Docs][docs]"), "{plain}");
}

#[test]
fn nested_inline_styles_keep_links_and_code_inside_table_cells() {
    let output = Markdown::new()
        .with_width(120)
        .render("| Rich |\n| --- |\n| **[Docs](https://example.com) and `value`** |");
    let plain = strip_ansi(&output);

    assert!(plain.contains("Docs and value"), "{plain}");
    assert_eq!(osc8_targets(&output), vec!["https://example.com"]);
    let docs = output
        .lines()
        .flat_map(ansi_segments)
        .find(|segment| segment.text.contains("Docs"))
        .expect("rendered link segment");
    assert!(docs.style.bold, "{output:?}");
    assert!(docs.style.underline, "{output:?}");
    assert_eq!(docs.style.fg, Some(Color::Blue), "{output:?}");
    assert!(output.contains("48;5;236"), "{output:?}");
    assert!(!plain.contains("[Docs]"), "{plain}");
}

#[test]
fn indented_code_with_table_pipes_is_not_rendered_as_a_table() {
    let output =
        strip_ansi(&Markdown::new().render("    | A | B |\n    | --- | --- |\n    | one | two |"));

    assert!(
        output.lines().any(|line| line == "    | A | B |"),
        "{output}"
    );
    assert!(!output.contains(['┌', '┼', '└']), "{output}");
}

#[test]
fn nested_list_item_not_duplicated() {
    // A nested list must render ONCE (as a sub-list), not also flattened into
    // the parent item's text — regression for "paragraph + its ordered list".
    let md = Markdown::new();
    let output = md.render("1. Parent\n   1. Child A\n   2. Child B");
    let plain = strip_ansi(&output);
    assert_eq!(
        plain.matches("Child A").count(),
        1,
        "nested item rendered twice:\n{plain}"
    );
    assert_eq!(plain.matches("Child B").count(), 1);
}

#[test]
fn render_blockquote() {
    let md = Markdown::new();
    let output = md.render("> quoted text");
    let plain = strip_ansi(&output);
    assert!(plain.contains("quoted text"));
}

#[test]
fn blockquote_wraps_to_configured_width() {
    let md = Markdown::new().with_width(8);
    let output = md.render("> abcdefghijklmnopqrstuvwxyz");
    let plain = strip_ansi(&output);

    assert_eq!(
        plain
            .lines()
            .map(|line| line.trim_start_matches("│ "))
            .collect::<String>(),
        "abcdefghijklmnopqrstuvwxyz"
    );
    assert!(output.lines().count() > 1);
    for line in output.lines() {
        assert!(visible_len(line) <= 8, "{line:?}");
    }
}

#[test]
fn blockquote_preserves_lists_and_their_hanging_indent() {
    let output = strip_ansi(
        &Markdown::new()
            .with_width(24)
            .render("> - alpha beta gamma delta epsilon"),
    );
    let lines = output.lines().collect::<Vec<_>>();

    assert!(lines[0].starts_with("│ - "), "{output}");
    assert!(lines[1].starts_with("│   "), "{output}");
}

#[test]
fn blockquote_inside_list_preserves_both_prefixes_when_wrapping() {
    let output = strip_ansi(
        &Markdown::new()
            .with_width(24)
            .render("- list item\n  > block quote inside list that wraps"),
    );
    let lines = output.lines().collect::<Vec<_>>();

    assert_eq!(lines[0], "- list item");
    assert!(lines[1].starts_with("  │ "), "{output}");
    assert!(lines[2].starts_with("  │ "), "{output}");
}

#[test]
fn blockquote_inside_ordered_list_aligns_under_item_content() {
    let output = strip_ansi(
        &Markdown::new()
            .with_width(24)
            .render("1. item with quote\n   > quoted text that should wrap"),
    );
    let lines = output.lines().collect::<Vec<_>>();

    assert_eq!(lines[0], "1. item with quote");
    assert!(lines[1].starts_with("   │ "), "{output}");
    assert!(lines[2].starts_with("   │ "), "{output}");
}

#[test]
fn nested_blockquotes_keep_each_quote_level() {
    let output = strip_ansi(&Markdown::new().render("> outer\n>\n> > inner"));

    assert!(output.lines().any(|line| line == "│ outer"), "{output}");
    assert!(output.lines().any(|line| line == "│ │ inner"), "{output}");
}

#[test]
fn blockquote_keeps_separate_paragraph_blocks() {
    let output = strip_ansi(&Markdown::new().render("> first paragraph\n>\n> second paragraph"));
    let paragraphs = output
        .lines()
        .filter(|line| line.contains("paragraph"))
        .collect::<Vec<_>>();

    assert_eq!(paragraphs, vec!["│ first paragraph", "│ second paragraph"]);
}

#[test]
fn blockquoted_code_keeps_structure_and_non_wrapping_metadata() {
    let source = "let extraordinarily_long_identifier = 123456789;";
    let rendered = Markdown::new()
        .with_width(18)
        .render_with_metadata(&format!("> ```rust\n> {source}\n> ```"));
    let plain = strip_ansi(rendered.as_str());
    let code_row = plain
        .lines()
        .position(|line| line == format!("│ {source}"))
        .expect("quoted code row");

    assert_eq!(rendered.non_wrapping_line_indices(), &[code_row]);
    assert!(!plain.contains(['┌', '└']));
    assert!(visible_len(plain.lines().nth(code_row).unwrap()) > 18);
}

#[test]
fn render_element_produces_box() {
    let md = Markdown::new();
    let el: Element<()> = md.render_element("# Hello\n\nWorld");
    match el {
        Element::Box(b) => assert!(!b.children.is_empty()),
        _ => panic!("expected Box"),
    }
}

#[test]
fn render_element_preserves_heading_style() {
    let md = Markdown::new();
    let el: Element<()> = md.render_element("# Hello");

    let Element::Box(box_el) = el else {
        panic!("expected Box");
    };
    let Element::Text(text) = &box_el.children[0] else {
        panic!("expected heading text");
    };

    assert!(text.style.bold);
    assert_eq!(text.style.fg, Some(Color::Rgb(122, 162, 247)));
    assert!(!text.content.contains('\x1b'));
}

#[test]
fn render_element_preserves_trailing_blank_rows() {
    let md = Markdown::new();
    let el: Element<()> = md.render_element("# Hello");

    let Element::Box(box_el) = el else {
        panic!("expected Box");
    };

    assert_eq!(box_el.children.len(), 2);
    let Element::Text(blank) = &box_el.children[1] else {
        panic!("expected trailing blank text row");
    };
    assert_eq!(blank.content, "");
}

#[test]
fn with_width_wraps() {
    let md = Markdown::new().with_width(10);
    let output = md.render("this is a long sentence that should wrap");
    let lines: Vec<&str> = output.lines().collect();
    assert!(lines.len() > 1);
}

#[test]
fn paragraph_hard_breaks_long_word_by_display_width() {
    let md = Markdown::new().with_width(8);
    let output = md.render("abcdefghijklmnopqrstuvwxyz");
    let plain = strip_ansi(&output);

    assert_eq!(
        plain.lines().collect::<String>(),
        "abcdefghijklmnopqrstuvwxyz"
    );
    for line in output.lines() {
        assert!(visible_len(line) <= 8, "{line:?}");
    }
}

#[test]
fn wrap_text_hard_break_keeps_zero_width_marks_with_base_glyph() {
    let lines = wrap_text("e\u{301}e\u{301}e", 1);

    assert!(lines.iter().all(|line| visible_len(line) <= 1));
    assert_eq!(lines, vec!["e\u{301}", "e\u{301}", "e"]);
}

#[test]
fn wrap_text_hard_break_packs_zero_width_marks_by_display_width() {
    let lines = wrap_text("e\u{301}e\u{301}e", 2);

    assert!(lines.iter().all(|line| visible_len(line) <= 2));
    assert_eq!(lines, vec!["e\u{301}e\u{301}", "e"]);
}

#[test]
fn paragraph_clips_wide_glyphs_at_single_column_width() {
    let md = Markdown::new().with_width(1);
    let output = md.render("中文");

    for line in output.lines() {
        assert!(visible_len(line) <= 1, "{line:?}");
    }
}

#[test]
fn oversized_width_is_clamped() {
    let md = Markdown::new().with_width(usize::MAX);

    assert_eq!(md.width, MAX_MARKDOWN_WIDTH);
    assert!(md.render("hello").contains("hello"));
}

#[test]
fn wrap_text_basic() {
    let lines = wrap_text("hello world foo bar", 10);
    assert!(lines.len() >= 2);
    for line in &lines {
        assert!(crate::style::visible_len(line) <= 10);
    }
}
