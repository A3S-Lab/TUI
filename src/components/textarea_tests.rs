use super::*;
use crate::event::Event;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
    }
}

fn ctrl(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::CONTROL,
    }
}

fn alt(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::ALT,
    }
}

#[test]
fn typing_text() {
    let mut ta = Textarea::new();
    ta.handle_key(&key(KeyCode::Char('h')));
    ta.handle_key(&key(KeyCode::Char('i')));
    assert_eq!(ta.value(), "hi");
}

#[test]
fn handle_event_paste_inserts_multiline_text_without_submitting() {
    let mut ta = Textarea::new().with_submit_on_enter(true);

    let msg = ta.handle_event(&Event::Paste("one\r\ntwo\tthree\u{7}".to_string()));

    assert!(matches!(msg, Some(TextareaMsg::Changed(value)) if value == "one\ntwo three"));
    assert_eq!(ta.value(), "one\ntwo three");
    assert_eq!(ta.cursor(), (1, 9));
}

#[test]
fn control_modified_characters_are_not_text_input() {
    let mut ta = Textarea::new();

    assert!(ta.handle_key(&ctrl(KeyCode::Char('c'))).is_none());

    assert_eq!(ta.value(), "");
}

#[test]
fn word_navigation_and_deletion() {
    let mut ta = Textarea::new();
    ta.set_value("alpha beta\ngamma delta");

    ta.handle_key(&alt(KeyCode::Char('b')));
    assert_eq!(ta.cursor(), (1, 6));

    ta.handle_key(&alt(KeyCode::Char('d')));
    assert_eq!(ta.value(), "alpha beta\ngamma ");
    assert_eq!(ta.cursor(), (1, 6));

    ta.handle_key(&ctrl(KeyCode::Char('w')));
    assert_eq!(ta.value(), "alpha beta\n");
    assert_eq!(ta.cursor(), (1, 0));
}

#[test]
fn word_navigation_crosses_line_boundaries() {
    let mut ta = Textarea::new();
    ta.set_value("alpha beta\ngamma");
    ta.cursor_row = 1;
    ta.cursor_col = 0;

    ta.handle_key(&alt(KeyCode::Char('b')));

    assert_eq!(ta.cursor(), (0, 6));
}

#[test]
fn multibyte_input_no_panic() {
    // Regression: cursor_col mixed char/byte indexing → panic on CJK input.
    let mut ta = Textarea::new();
    for c in "你好abc世界".chars() {
        ta.handle_key(&key(KeyCode::Char(c)));
    }
    assert_eq!(ta.value(), "你好abc世界");
    // Move left over multibyte chars and edit — must not panic on a boundary.
    // chars: 你 好 a b c 世 界 ; cursor at end (7), Left x4 -> col 3 (before 'b')
    for _ in 0..4 {
        ta.handle_key(&key(KeyCode::Left));
    }
    ta.handle_key(&key(KeyCode::Backspace)); // col 3->2, deletes 'a'
    ta.handle_key(&key(KeyCode::Delete)); // deletes 'b' at col 2
    assert_eq!(ta.value(), "你好c世界");
    let _ = ta.view();
}

#[test]
fn cjk_cursor_stays_in_view_when_line_overflows() {
    // Narrow box; type a long CJK line. The visible render must never exceed
    // the width and must always include the cursor block at the end.
    let mut ta = Textarea::new().with_width(8).with_height(1);
    for c in "你好世界你好世界".chars() {
        ta.handle_key(&key(KeyCode::Char(c)));
    }
    let line = ta.view();
    let visible = crate::style::visible_len(&line); // ANSI-stripped display width
    assert!(visible <= 8, "rendered width {visible} exceeds box width 8");
    // Insertion point must stay within the visible window for the host to
    // place the real cursor on it.
    assert!(
        ta.cursor_display_col() <= 8,
        "cursor col {} out of view",
        ta.cursor_display_col()
    );
}

#[test]
fn horizontal_scroll_keeps_zero_width_marks_with_visible_base_glyph() {
    let mut ta = Textarea::new().with_width(1).with_height(1);
    ta.set_value("e\u{301}x");
    ta.handle_key(&key(KeyCode::Home));

    assert_eq!(ta.view(), "e\u{301}");
    assert_eq!(crate::style::visible_len(&ta.view()), 1);

    let Element::Box(box_el) = ta.element::<()>() else {
        panic!("expected textarea element box");
    };
    let Element::Text(line) = &box_el.children[0] else {
        panic!("expected textarea text line");
    };
    assert_eq!(line.content, "e\u{301}");
}

#[test]
fn horizontal_scroll_drops_zero_width_marks_with_hidden_base_glyph() {
    let mut ta = Textarea::new().with_width(2).with_height(1);
    ta.set_value("e\u{301}x");

    assert_eq!(ta.view(), "x");
    assert_eq!(crate::style::visible_len(&ta.view()), 1);
    assert_eq!(ta.cursor_display_col(), 1);

    let Element::Box(box_el) = ta.element::<()>() else {
        panic!("expected textarea element box");
    };
    let Element::Text(line) = &box_el.children[0] else {
        panic!("expected textarea text line");
    };
    assert_eq!(line.content, "x");
}

#[test]
fn cursor_movement_skips_zero_width_marks() {
    let mut ta = Textarea::new().with_width(4).with_height(1);
    ta.set_value("e\u{301}x");
    ta.handle_key(&key(KeyCode::Home));

    ta.handle_key(&key(KeyCode::Right));
    assert_eq!(ta.cursor_col, 2);
    assert_eq!(ta.cursor_display_col(), 1);

    ta.handle_key(&key(KeyCode::Right));
    assert_eq!(ta.cursor_col, 3);
    assert_eq!(ta.cursor_display_col(), 2);

    ta.handle_key(&key(KeyCode::Left));
    assert_eq!(ta.cursor_col, 2);
    ta.handle_key(&key(KeyCode::Left));
    assert_eq!(ta.cursor_col, 0);
}

#[test]
fn edit_keys_remove_zero_width_span_with_base_glyph() {
    let mut ta = Textarea::new();
    ta.set_value("e\u{301}x");
    ta.handle_key(&key(KeyCode::Home));
    ta.handle_key(&key(KeyCode::Delete));

    assert_eq!(ta.value(), "x");
    assert_eq!(ta.cursor_col, 0);

    ta.set_value("e\u{301}x");
    ta.handle_key(&key(KeyCode::Home));
    ta.handle_key(&key(KeyCode::Right));
    ta.handle_key(&key(KeyCode::Backspace));

    assert_eq!(ta.value(), "x");
    assert_eq!(ta.cursor_col, 0);
}

#[test]
fn newline_creates_new_line() {
    let mut ta = Textarea::new();
    ta.handle_key(&key(KeyCode::Char('a')));
    ta.handle_key(&key(KeyCode::Enter));
    ta.handle_key(&key(KeyCode::Char('b')));
    assert_eq!(ta.value(), "a\nb");
}

#[test]
fn auto_grow_shows_all_lines_after_newline() {
    // Regression: with auto-grow, adding a newline grew the height but left
    // the scroll offset following the cursor, hiding the first line. The box
    // must show BOTH lines (height grows, offset resets).
    let mut ta = Textarea::new().with_height(1).with_auto_grow(8);
    ta.handle_key(&key(KeyCode::Char('a')));
    ta.handle_key(&key(KeyCode::Enter));
    ta.handle_key(&key(KeyCode::Char('b')));
    assert_eq!(ta.height(), 2, "box grew to two rows");
    let view = ta.view();
    assert!(view.contains('a'), "first line still visible");
    assert!(view.contains('b'), "second line visible");
}

#[test]
fn zero_height_keeps_offset_safe_after_content_changes() {
    let mut ta = Textarea::new().with_height(0);
    ta.handle_key(&key(KeyCode::Enter));
    ta.handle_key(&key(KeyCode::Enter));
    assert_eq!(ta.offset, 0);

    ta.set_value("short");

    assert_eq!(ta.view(), "");
    let Element::Box(box_el) = ta.element::<()>() else {
        panic!("expected empty textarea element box");
    };
    assert!(box_el.children.is_empty());
}

#[test]
fn zero_height_hides_placeholder() {
    let mut ta = Textarea::new()
        .with_height(0)
        .with_placeholder("compose message");
    ta.blur();

    assert_eq!(ta.view(), "");
    let Element::Box(box_el) = ta.element::<()>() else {
        panic!("expected empty textarea element box");
    };
    assert!(box_el.children.is_empty());
}

#[test]
fn zero_width_renders_empty() {
    let mut ta = Textarea::new().with_width(0).with_height(2);
    ta.set_value("abc");

    assert_eq!(ta.view(), "");

    let Element::Box(box_el) = ta.element::<()>() else {
        panic!("expected empty textarea element box");
    };
    assert!(box_el.children.is_empty());
}

#[test]
fn set_value_clamps_stale_offset_after_content_shrinks() {
    let mut ta = Textarea::new().with_height(2);
    ta.set_value("one\ntwo\nthree\nfour\nfive");
    assert!(ta.view().contains("five"));

    ta.handle_key(&key(KeyCode::Up));
    ta.handle_key(&key(KeyCode::Down));
    assert!(ta.offset > 0);

    ta.set_value("short");

    assert_eq!(ta.offset, 0);
    assert!(ta.view().contains("short"));
    let Element::Box(box_el) = ta.element::<()>() else {
        panic!("expected textarea element box");
    };
    let Element::Text(first) = &box_el.children[0] else {
        panic!("expected first textarea line");
    };
    assert_eq!(first.content, "short");
}

#[test]
fn editing_normalizes_stale_cursor() {
    let mut ta = Textarea::new();
    ta.set_value("one\ntwo");
    ta.cursor_row = usize::MAX;
    ta.cursor_col = usize::MAX;

    ta.handle_key(&key(KeyCode::Char('!')));

    assert_eq!(ta.value(), "one\ntwo!");
    assert_eq!(ta.cursor_row, 1);
    assert_eq!(ta.cursor_col, 4);
}

#[test]
fn insert_str_normalizes_stale_cursor() {
    let mut ta = Textarea::new();
    ta.set_value("one\ntwo");
    ta.cursor_row = usize::MAX;
    ta.cursor_col = usize::MAX;

    ta.insert_str("!");

    assert_eq!(ta.value(), "one\ntwo!");
    assert_eq!(ta.cursor_row, 1);
    assert_eq!(ta.cursor_col, 4);
}

#[test]
fn rendering_normalizes_stale_cursor() {
    let mut ta = Textarea::new().with_height(2);
    ta.set_value("one\ntwo");
    ta.cursor_row = usize::MAX;
    ta.cursor_col = usize::MAX;

    assert_eq!(ta.cursor_row(), 1);
    assert_eq!(ta.cursor_display_col(), 3);
    assert_eq!(ta.view(), "one\ntwo");

    let Element::Box(box_el) = ta.element::<()>() else {
        panic!("expected textarea element box");
    };
    let Element::Text(last) = &box_el.children[1] else {
        panic!("expected textarea text line");
    };
    assert_eq!(last.content, "two");
}

#[test]
fn rendering_keeps_stale_cursor_visible() {
    let mut ta = Textarea::new().with_height(2);
    ta.set_value("one\ntwo\nthree\nfour");
    ta.cursor_row = usize::MAX;
    ta.cursor_col = usize::MAX;
    ta.offset = 0;

    assert_eq!(ta.cursor_row(), 1);
    assert_eq!(ta.view(), "three\nfour");

    let Element::Box(box_el) = ta.element::<()>() else {
        panic!("expected textarea element box");
    };
    let Element::Text(first) = &box_el.children[0] else {
        panic!("expected first visible textarea line");
    };
    let Element::Text(last) = &box_el.children[1] else {
        panic!("expected last visible textarea line");
    };
    assert_eq!(first.content, "three");
    assert_eq!(last.content, "four");
}

#[test]
fn set_value_preserves_trailing_empty_lines() {
    let mut ta = Textarea::new().with_auto_grow(4);

    ta.set_value("first\n");

    assert_eq!(ta.value(), "first\n");
    assert_eq!(ta.height(), 2);
    assert_eq!(ta.cursor_row, 1);
    assert_eq!(ta.cursor_col, 0);
}

#[test]
fn set_value_normalizes_crlf_lines() {
    let mut ta = Textarea::new();

    ta.set_value("first\r\nsecond");

    assert_eq!(ta.value(), "first\nsecond");
}

#[test]
fn backspace_joins_lines() {
    let mut ta = Textarea::new();
    ta.set_value("ab\ncd");
    ta.cursor_row = 1;
    ta.cursor_col = 0;
    ta.handle_key(&key(KeyCode::Backspace));
    assert_eq!(ta.value(), "abcd");
}

#[test]
fn cursor_movement() {
    let mut ta = Textarea::new();
    ta.set_value("hello\nworld");
    ta.cursor_row = 1;
    ta.cursor_col = 5;
    ta.handle_key(&key(KeyCode::Up));
    assert_eq!(ta.cursor_row, 0);
    ta.handle_key(&key(KeyCode::Home));
    assert_eq!(ta.cursor_col, 0);
    ta.handle_key(&key(KeyCode::End));
    assert_eq!(ta.cursor_col, 5);
}

#[test]
fn ctrl_a_and_e() {
    let mut ta = Textarea::new();
    ta.set_value("test");
    ta.cursor_col = 2;
    ta.handle_key(&ctrl(KeyCode::Char('a')));
    assert_eq!(ta.cursor_col, 0);
    ta.handle_key(&ctrl(KeyCode::Char('e')));
    assert_eq!(ta.cursor_col, 4);
}

#[test]
fn ctrl_k_kills_line() {
    let mut ta = Textarea::new();
    ta.set_value("hello world");
    ta.cursor_col = 5;
    ta.handle_key(&ctrl(KeyCode::Char('k')));
    assert_eq!(ta.value(), "hello");
}

#[test]
fn char_limit() {
    let mut ta = Textarea::new().with_char_limit(3);
    ta.handle_key(&key(KeyCode::Char('a')));
    ta.handle_key(&key(KeyCode::Char('b')));
    ta.handle_key(&key(KeyCode::Char('c')));
    ta.handle_key(&key(KeyCode::Char('d')));
    assert_eq!(ta.value(), "abc");
}

#[test]
fn char_limit_counts_newlines() {
    let mut ta = Textarea::new().with_char_limit(1);
    ta.handle_key(&key(KeyCode::Char('a')));

    assert!(ta.handle_key(&key(KeyCode::Enter)).is_none());

    assert_eq!(ta.value(), "a");
    assert_eq!(ta.total_chars(), 1);
}

#[test]
fn insert_str_respects_char_limit() {
    let mut ta = Textarea::new().with_char_limit(5);

    ta.insert_str("ab\ncd\nef");

    assert_eq!(ta.value(), "ab\ncd");
    assert_eq!(ta.total_chars(), 5);
}

#[test]
fn insert_str_updates_auto_grow_height() {
    let mut ta = Textarea::new().with_height(1).with_auto_grow(8);

    ta.insert_str("a\nb\nc");

    assert_eq!(ta.height(), 3);
    assert_eq!(ta.view(), "a\nb\nc");
}

#[test]
fn set_value_respects_char_limit_with_newlines() {
    let mut ta = Textarea::new().with_char_limit(5);

    ta.set_value("ab\ncd\nef");

    assert_eq!(ta.value(), "ab\ncd");
    assert_eq!(ta.total_chars(), 5);
    assert_eq!(ta.cursor_row, 1);
    assert_eq!(ta.cursor_col, 2);
}

#[test]
fn set_value_respects_char_limit_on_char_boundaries() {
    let mut ta = Textarea::new().with_char_limit(2);

    ta.set_value("你好abc");

    assert_eq!(ta.value(), "你好");
    assert_eq!(ta.cursor_row, 0);
    assert_eq!(ta.cursor_col, 2);
}

#[test]
fn set_value_respects_zero_char_limit() {
    let mut ta = Textarea::new().with_char_limit(0);

    ta.set_value("hello");

    assert_eq!(ta.value(), "");
    assert_eq!(ta.total_chars(), 0);
    assert_eq!(ta.cursor_row, 0);
    assert_eq!(ta.cursor_col, 0);
}

#[test]
fn auto_grow_saturates_large_line_counts() {
    let mut ta = Textarea::new().with_auto_grow(u16::MAX);
    let text = (0..u16::MAX as usize + 1)
        .map(|_| "x")
        .collect::<Vec<_>>()
        .join("\n");

    ta.set_value(&text);

    assert_eq!(ta.height(), u16::MAX);
}

#[test]
fn clear_resets() {
    let mut ta = Textarea::new();
    ta.set_value("some\ntext");
    ta.clear();
    assert_eq!(ta.value(), "");
    assert_eq!(ta.cursor_row, 0);
    assert_eq!(ta.cursor_col, 0);
}

#[test]
fn submit_on_enter() {
    let mut ta = Textarea::new().with_submit_on_enter(true);
    ta.handle_key(&key(KeyCode::Char('x')));
    let msg = ta.handle_key(&key(KeyCode::Enter));
    assert!(matches!(msg, Some(TextareaMsg::Submit(s)) if s == "x"));
}
